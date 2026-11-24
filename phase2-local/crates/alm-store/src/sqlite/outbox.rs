//! The confirmation outbox: each intent is stored with its request ID before it is sent, so a
//! lost response or a restart can be resolved by status lookup or an explicit retry with the
//! same ID. Nothing here sends; a stored intent is never posted automatically.
use std::path::Path;

use rusqlite::OptionalExtension;

use super::{DraftStorageError, SqliteDraftStore, text};

const OUTBOX_TABLE: &str = "
CREATE TABLE IF NOT EXISTS confirmation_outbox (
    seq INTEGER PRIMARY KEY AUTOINCREMENT,
    request_id TEXT NOT NULL UNIQUE CHECK (length(request_id) BETWEEN 1 AND 64),
    intent TEXT NOT NULL CHECK (length(intent) > 0),
    status TEXT NOT NULL CHECK (status IN ('pending', 'unknown', 'confirmed', 'rejected')),
    outcome TEXT
) STRICT;
";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutboxStatus {
    /// Stored and being sent by this session.
    Pending,
    /// Sent with no answer (lost response or restart mid-send): resolve before anything else.
    Unknown,
    Confirmed,
    Rejected,
}

impl OutboxStatus {
    const fn text(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Unknown => "unknown",
            Self::Confirmed => "confirmed",
            Self::Rejected => "rejected",
        }
    }

    fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "pending" => Self::Pending,
            "unknown" => Self::Unknown,
            "confirmed" => Self::Confirmed,
            "rejected" => Self::Rejected,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutboxEntry {
    pub request_id: String,
    pub intent: String,
    pub status: OutboxStatus,
    /// The canonical outcome when confirmed, or the reason when rejected.
    pub outcome: Option<String>,
}

#[derive(Debug)]
pub enum OutboxError {
    Storage(DraftStorageError),
    /// The request ID is already stored with a different intent.
    Conflict,
    NotFound,
}

impl From<DraftStorageError> for OutboxError {
    fn from(error: DraftStorageError) -> Self {
        Self::Storage(error)
    }
}

impl From<rusqlite::Error> for OutboxError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Storage(error.into())
    }
}

/// One account's outbox, in that account's device database.
pub struct Outbox {
    store: SqliteDraftStore,
}

impl Outbox {
    /// Opens the account's outbox. Intents still `pending` from an earlier session were in
    /// flight when it ended, so they become `unknown`.
    ///
    /// # Errors
    /// Rejects invalid IDs, unsafe locations and files bound to another account.
    pub fn open_scoped(device_root: &Path, account: &str) -> Result<Self, DraftStorageError> {
        let mut store = SqliteDraftStore::open_scoped(device_root, account)?;
        let tx = store.begin()?;
        tx.execute_batch(OUTBOX_TABLE)?;
        tx.execute(
            "UPDATE confirmation_outbox SET status = 'unknown' WHERE status = 'pending'",
            [],
        )?;
        tx.commit()?;
        Ok(Self { store })
    }

    /// A request ID this outbox has not used: `<prefix>-<n>`.
    ///
    /// # Errors
    /// Fails on storage errors.
    pub fn next_request_id(&self, prefix: &str) -> Result<String, DraftStorageError> {
        // The AUTOINCREMENT high-water mark, so a purged outbox never reuses a request ID.
        let last: Option<i64> = self.store.conn.query_row(
            "SELECT max(n) FROM (SELECT max(seq) AS n FROM confirmation_outbox
             UNION ALL SELECT seq FROM sqlite_sequence WHERE name = 'confirmation_outbox')",
            [],
            |row| row.get(0),
        )?;
        Ok(format!("{prefix}-{}", last.unwrap_or(0) + 1))
    }

    /// Stores `intent` under `request_id` as pending, before it is sent. The same pair again
    /// returns the stored entry unchanged (an explicit retry); a different intent conflicts.
    ///
    /// # Errors
    /// `Conflict` on reuse with another intent; storage errors.
    pub fn record(&mut self, request_id: &str, intent: &str) -> Result<OutboxEntry, OutboxError> {
        if let Some(entry) = self.get(request_id)? {
            return if entry.intent == intent {
                Ok(entry)
            } else {
                Err(OutboxError::Conflict)
            };
        }
        let tx = self.store.begin()?;
        tx.execute(
            "INSERT INTO confirmation_outbox (request_id, intent, status) VALUES (?1, ?2, 'pending')",
            (request_id, intent),
        )?;
        tx.commit()?;
        self.get(request_id)?.ok_or(OutboxError::NotFound)
    }

    /// Records what happened to a sent intent.
    ///
    /// # Errors
    /// `NotFound` for an unknown request; storage errors.
    pub fn mark(
        &mut self,
        request_id: &str,
        status: OutboxStatus,
        outcome: Option<&str>,
    ) -> Result<(), OutboxError> {
        let tx = self.store.begin()?;
        let changed = tx.execute(
            "UPDATE confirmation_outbox SET status = ?2, outcome = ?3 WHERE request_id = ?1",
            (request_id, status.text(), outcome),
        )?;
        tx.commit()?;
        if changed == 1 {
            Ok(())
        } else {
            Err(OutboxError::NotFound)
        }
    }

    /// # Errors
    /// Fails on storage errors or a corrupt row.
    pub fn get(&self, request_id: &str) -> Result<Option<OutboxEntry>, DraftStorageError> {
        self.store
            .conn
            .query_row(
                "SELECT request_id, intent, status, outcome FROM confirmation_outbox WHERE request_id = ?1",
                [request_id],
                |row| Ok(read(row)),
            )
            .optional()?
            .transpose()
    }

    /// Every stored intent, oldest first.
    ///
    /// # Errors
    /// Fails on storage errors or a corrupt row.
    pub fn entries(&self) -> Result<Vec<OutboxEntry>, DraftStorageError> {
        let mut query = self.store.conn.prepare(
            "SELECT request_id, intent, status, outcome FROM confirmation_outbox ORDER BY seq",
        )?;
        let mut rows = query.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(read(row)?);
        }
        Ok(out)
    }

    /// Removes every stored intent and outcome (the account lost the role that sent them) and
    /// overwrites their bytes in the file; request numbering continues after the last one.
    ///
    /// # Errors
    /// Fails on storage errors.
    pub fn purge(&mut self) -> Result<(), DraftStorageError> {
        let conn = &self.store.conn;
        conn.pragma_update(None, "secure_delete", true)?;
        conn.execute("DELETE FROM confirmation_outbox", [])?;
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE); VACUUM;")?;
        Ok(())
    }

    /// Intents whose outcome this device does not know yet.
    ///
    /// # Errors
    /// Fails on storage errors or a corrupt row.
    pub fn unresolved(&self) -> Result<Vec<OutboxEntry>, DraftStorageError> {
        Ok(self
            .entries()?
            .into_iter()
            .filter(|e| matches!(e.status, OutboxStatus::Pending | OutboxStatus::Unknown))
            .collect())
    }
}

fn read(row: &rusqlite::Row<'_>) -> Result<OutboxEntry, DraftStorageError> {
    let required = |index| text(row, index)?.ok_or(DraftStorageError::InvalidOutput);
    Ok(OutboxEntry {
        request_id: required(0)?,
        intent: required(1)?,
        status: OutboxStatus::parse(&required(2)?).ok_or(DraftStorageError::InvalidOutput)?,
        outcome: text(row, 3)?,
    })
}
