use std::fmt;
use std::io;
use std::path::Path;
use std::time::Duration;

use rusqlite::types::ValueRef;
use rusqlite::{Connection, Row, Transaction, TransactionBehavior};

use crate::drafts::{DraftId, ProvisionalDraft};
use crate::{DraftCustomer, crate_row_codec};

mod draft_rows;
mod pending;
mod schema;
mod scoped;
#[cfg(test)]
mod tests;

/// Failure to open, save, or reopen a local SQLite draft.
#[derive(Debug)]
#[non_exhaustive]
pub enum DraftStorageError {
    /// The database path must be absolute and contain no NUL bytes.
    InvalidPath,
    /// A synthetic account ID is empty, too long, or contains disallowed characters.
    InvalidAccountScope,
    /// An existing scoped file is not bound to the selected synthetic account.
    AccountStoreMismatch,
    /// A file-system operation on the device root or account directory failed.
    Io(io::Error),
    /// SQLite rejected an open, statement, or transaction.
    Sqlite(rusqlite::Error),
    /// The stored rows could not be decoded as a draft.
    InvalidOutput,
    /// A line position exceeds SQLite's integer range.
    TooManyLines,
}

impl fmt::Display for DraftStorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPath => f.write_str("expected a safe absolute SQLite path or device root"),
            Self::InvalidAccountScope => f.write_str(
                "expected a synthetic account ID using ASCII letters, digits, '-' or '_'",
            ),
            Self::AccountStoreMismatch => {
                f.write_str("database is not bound to the selected account")
            }
            Self::Io(err) => write!(f, "local storage I/O failed: {err}"),
            Self::Sqlite(err) => write!(f, "SQLite failed: {err}"),
            Self::InvalidOutput => f.write_str("invalid SQLite draft output"),
            Self::TooManyLines => f.write_str("too many draft lines for SQLite"),
        }
    }
}

impl std::error::Error for DraftStorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Sqlite(err) => Some(err),
            Self::InvalidPath
            | Self::InvalidAccountScope
            | Self::AccountStoreMismatch
            | Self::InvalidOutput
            | Self::TooManyLines => None,
        }
    }
}

impl From<io::Error> for DraftStorageError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<rusqlite::Error> for DraftStorageError {
    fn from(err: rusqlite::Error) -> Self {
        Self::Sqlite(err)
    }
}

pub(crate) struct SqliteDraftStore {
    conn: Connection,
}

fn connect(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    // Fail on lock contention instead of waiting, as the sqlite3 CLI did.
    conn.busy_timeout(Duration::ZERO)?;
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
    pub(crate) fn open(path: &Path) -> Result<Self, DraftStorageError> {
        if !path.is_absolute() || path.as_os_str().as_encoded_bytes().contains(&0) {
            return Err(DraftStorageError::InvalidPath);
        }
        let store = Self {
            conn: connect(path)?,
        };
        let tx = store.begin()?;
        tx.execute_batch(schema::DRAFT_TABLES)?;
        tx.execute_batch(schema::OWNER_PENDING_TABLE)?;
        tx.commit()?;
        Ok(store)
    }

    fn begin(&self) -> rusqlite::Result<Transaction<'_>> {
        Transaction::new_unchecked(&self.conn, TransactionBehavior::Immediate)
    }

    pub(crate) fn save(&self, draft: &ProvisionalDraft) -> Result<(), DraftStorageError> {
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
        ] {
            tx.execute(&format!("DELETE FROM {table} WHERE draft_id = ?1"), [&id])?;
        }
        if let Some(date) = draft.bill_date() {
            tx.execute(
                "INSERT INTO provisional_draft_dates VALUES (?1, ?2)",
                (&id, date.to_string()),
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
        if let Some(row) = draft.crate_row() {
            tx.execute(
                "INSERT INTO provisional_draft_crates VALUES (?1, ?2)",
                (&id, crate_row_codec::encode(row)),
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub(crate) fn reopen(
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
        draft_rows::decode(id, decoded)
    }
}
