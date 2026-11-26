use std::path::Path;
use std::time::Duration;

use rusqlite::types::ValueRef;
use rusqlite::{Connection, OptionalExtension, Row, Transaction, TransactionBehavior};

use alm_core::{
    DraftCustomer, DraftId, DraftStoreFailure, DurableDraftStore, ProvisionalDraft,
    encode_crate_row,
};

use crate::DraftStorageError;

mod draft_rows;
mod gst_bills;
mod outbox;
mod pending;
mod projection;
mod schema;
mod scoped;
#[cfg(test)]
mod tests;

pub use gst_bills::GstBillDatabase;
pub use outbox::{Outbox, OutboxEntry, OutboxError, OutboxStatus};
pub use projection::{ProjectionError, ProjectionStore, StoredProjection};

pub struct SqliteDraftStore {
    conn: Connection,
}

const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

fn connect(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    // Wait out short lock contention from another connection, such as a reader.
    conn.busy_timeout(BUSY_TIMEOUT)?;
    conn.pragma_update(None, "foreign_keys", true)?;
    conn.pragma_update(None, "synchronous", "FULL")?;
    Ok(conn)
}

fn text(row: &Row<'_>, index: usize) -> Result<Option<String>, DraftStorageError> {
    match row.get_ref(index)? {
        ValueRef::Null => Ok(None),
        ValueRef::Text(bytes) => String::from_utf8(bytes.to_vec())
            .map(Some)
            .map_err(|_| DraftStorageError::InvalidOutput),
        ValueRef::Integer(_) | ValueRef::Real(_) | ValueRef::Blob(_) => {
            Err(DraftStorageError::InvalidOutput)
        }
    }
}

fn integer(row: &Row<'_>, index: usize) -> Result<Option<i64>, DraftStorageError> {
    match row.get_ref(index)? {
        ValueRef::Null => Ok(None),
        ValueRef::Integer(value) => Ok(Some(value)),
        ValueRef::Text(_) | ValueRef::Real(_) | ValueRef::Blob(_) => {
            Err(DraftStorageError::InvalidOutput)
        }
    }
}

impl SqliteDraftStore {
    pub fn open(path: &Path) -> Result<Self, DraftStorageError> {
        if !path.is_absolute() || path.as_os_str().as_encoded_bytes().contains(&0) {
            return Err(DraftStorageError::InvalidPath);
        }
        let mut store = Self {
            conn: connect(path)?,
        };
        let tx = store.begin()?;
        tx.execute_batch(schema::DRAFT_TABLES)?;
        tx.execute_batch(schema::LOADING_TABLE)?;
        tx.execute_batch(schema::OWNER_PENDING_TABLE)?;
        tx.commit()?;
        store.enable_wal()?;
        Ok(store)
    }

    /// Switches to WAL; called only once a file is known to be ours, since it rewrites the header.
    fn enable_wal(&self) -> Result<(), DraftStorageError> {
        let mode: String =
            self.conn
                .pragma_update_and_check(None, "journal_mode", "WAL", |row| row.get(0))?;
        if mode.eq_ignore_ascii_case("wal") {
            Ok(())
        } else {
            Err(DraftStorageError::InvalidOutput)
        }
    }

    fn begin(&mut self) -> rusqlite::Result<Transaction<'_>> {
        self.conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
    }
}

impl DurableDraftStore for SqliteDraftStore {
    fn save(&mut self, draft: &ProvisionalDraft) -> Result<(), DraftStoreFailure> {
        self.save_rows(draft).map_err(DraftStoreFailure::new)
    }

    fn reopen(&self, id: DraftId) -> Result<Option<ProvisionalDraft>, DraftStoreFailure> {
        self.reopen_rows(id).map_err(DraftStoreFailure::new)
    }

    fn save_pending(&mut self, request: u64, snapshot: &str) -> Result<(), DraftStoreFailure> {
        self.insert_pending(request, snapshot)
            .map_err(DraftStoreFailure::new)
    }

    fn load_pending(&self, request: u64) -> Result<Option<String>, DraftStoreFailure> {
        self.select_pending(request).map_err(DraftStoreFailure::new)
    }
}

impl SqliteDraftStore {
    pub(crate) fn save_rows(&mut self, draft: &ProvisionalDraft) -> Result<(), DraftStorageError> {
        let id = draft.id().value().to_string();
        let tx = self.begin()?;
        tx.execute(
            "INSERT INTO provisional_drafts (id) VALUES (?1) ON CONFLICT(id) DO NOTHING",
            [&id],
        )?;
        for table in [
            "provisional_draft_customers",
            "provisional_draft_lines",
            "provisional_draft_crates",
            "provisional_draft_dates",
            "provisional_draft_loading",
        ] {
            tx.execute(&format!("DELETE FROM {table} WHERE draft_id = ?1"), [&id])?;
        }
        if let Some(date) = draft.bill_date() {
            tx.execute(
                "INSERT INTO provisional_draft_dates VALUES (?1, ?2)",
                (&id, date.to_string()),
            )?;
        }
        let loading = draft.loading_rate().minor_units();
        if loading > 0 {
            tx.execute(
                "INSERT INTO provisional_draft_loading VALUES (?1, ?2)",
                (&id, loading),
            )?;
        }
        match draft.customer() {
            DraftCustomer::Unassigned => {}
            DraftCustomer::Typed(name) => {
                tx.execute(
                    "INSERT INTO provisional_draft_customers VALUES (?1, 1, NULL, ?2)",
                    (&id, name),
                )?;
            }
            DraftCustomer::Selected {
                id: customer_id,
                name,
            } => {
                tx.execute(
                    "INSERT INTO provisional_draft_customers VALUES (?1, 2, ?2, ?3)",
                    (&id, customer_id.value(), name),
                )?;
            }
        }
        {
            let mut insert = tx.prepare(
                "INSERT INTO provisional_draft_lines (draft_id, position, product_id, quantity)
                 VALUES (?1, ?2, ?3, ?4)",
            )?;
            for (position, line) in draft.lines().iter().enumerate() {
                let position =
                    i64::try_from(position).map_err(|_| DraftStorageError::TooManyLines)?;
                insert.execute((
                    &id,
                    position,
                    line.product_id().value().to_string(),
                    line.quantity().get(),
                ))?;
            }
        }
        if !draft.crate_rows().is_empty() {
            // Several crate rows share the one definition, joined by `;;`.
            let definition = draft
                .crate_rows()
                .iter()
                .map(encode_crate_row)
                .collect::<Vec<_>>()
                .join(";;");
            tx.execute(
                "INSERT INTO provisional_draft_crates VALUES (?1, ?2)",
                (&id, definition),
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Every saved draft's local number (as an origin-less ID). A draft file belongs to one
    /// account on one install, so the number identifies the draft; the caller supplies the
    /// install's origin, and a draft is saved and reopened under the ID it is given.
    pub(crate) fn draft_ids(&self) -> Result<Vec<DraftId>, DraftStorageError> {
        let mut query = self.conn.prepare("SELECT id FROM provisional_drafts")?;
        let mut rows = query.query([])?;
        let mut ids = Vec::new();
        while let Some(row) = rows.next()? {
            let seq: u64 = row
                .get::<_, String>(0)?
                .parse()
                .map_err(|_| DraftStorageError::InvalidOutput)?;
            ids.push(DraftId::new(seq));
        }
        ids.sort_unstable();
        Ok(ids)
    }

    pub(crate) fn reopen_rows(
        &self,
        id: DraftId,
    ) -> Result<Option<ProvisionalDraft>, DraftStorageError> {
        let mut query = self.conn.prepare(
            "SELECT b.bill_date, COALESCE(c.kind, 0), c.customer_id, c.name, k.definition, l.position, l.product_id, l.quantity
             FROM provisional_drafts AS d
             LEFT JOIN provisional_draft_dates AS b ON b.draft_id = d.id
             LEFT JOIN provisional_draft_customers AS c ON c.draft_id = d.id
             LEFT JOIN provisional_draft_lines AS l ON l.draft_id = d.id
             LEFT JOIN provisional_draft_crates AS k ON k.draft_id = d.id
             WHERE d.id = ?1
             ORDER BY l.position",
        )?;
        let mut rows = query.query([id.value().to_string()])?;
        let mut decoded = Vec::new();
        while let Some(row) = rows.next()? {
            decoded.push(draft_rows::DraftRow::read(row)?);
        }
        let loading: Option<i64> = self
            .conn
            .query_row(
                "SELECT rate FROM provisional_draft_loading WHERE draft_id = ?1",
                [id.value().to_string()],
                |row| row.get(0),
            )
            .optional()?;
        Ok(draft_rows::decode(id, decoded)?.map(|draft| match loading {
            Some(rate) => draft.with_loading_rate(alm_core::Money::from_minor_units(rate)),
            None => draft,
        }))
    }
}
