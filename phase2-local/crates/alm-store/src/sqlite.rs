use std::path::Path;
use std::time::Duration;

use rusqlite::types::ValueRef;
use rusqlite::{Connection, Row, Transaction, TransactionBehavior};

use alm_core::{
    DraftCustomer, DraftId, DraftStorageError, DurableDraftStore, ProvisionalDraft,
    encode_crate_row,
};

mod draft_rows;
mod pending;
mod schema;
mod scoped;
#[cfg(test)]
mod tests;

/// Wraps a driver error; the error and `?` conversions are defined in `alm-core`.
pub(crate) fn sqlite(err: rusqlite::Error) -> DraftStorageError {
    DraftStorageError::Sqlite(Box::new(err))
}

pub struct SqliteDraftStore {
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
    match row.get_ref(index).map_err(sqlite)? {
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
    match row.get_ref(index).map_err(sqlite)? {
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
            conn: connect(path).map_err(sqlite)?,
        };
        let tx = store.begin().map_err(sqlite)?;
        tx.execute_batch(schema::DRAFT_TABLES).map_err(sqlite)?;
        tx.execute_batch(schema::OWNER_PENDING_TABLE)
            .map_err(sqlite)?;
        tx.commit().map_err(sqlite)?;
        Ok(store)
    }

    fn begin(&mut self) -> rusqlite::Result<Transaction<'_>> {
        self.conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
    }
}

impl DurableDraftStore for SqliteDraftStore {
    fn save(&mut self, draft: &ProvisionalDraft) -> Result<(), DraftStorageError> {
        let id = draft.id().value().to_string();
        let tx = self.begin().map_err(sqlite)?;
        tx.execute(
            "INSERT INTO provisional_drafts (id) VALUES (?1) ON CONFLICT(id) DO NOTHING",
            [&id],
        )
        .map_err(sqlite)?;
        for table in [
            "provisional_draft_customers",
            "provisional_draft_lines",
            "provisional_draft_crates",
            "provisional_draft_dates",
        ] {
            tx.execute(&format!("DELETE FROM {table} WHERE draft_id = ?1"), [&id])
                .map_err(sqlite)?;
        }
        if let Some(date) = draft.bill_date() {
            tx.execute(
                "INSERT INTO provisional_draft_dates VALUES (?1, ?2)",
                (&id, date.to_string()),
            )
            .map_err(sqlite)?;
        }
        match draft.customer() {
            DraftCustomer::Unassigned => {}
            DraftCustomer::Typed(name) => {
                tx.execute(
                    "INSERT INTO provisional_draft_customers VALUES (?1, 1, NULL, ?2)",
                    (&id, name),
                )
                .map_err(sqlite)?;
            }
            DraftCustomer::Selected {
                id: customer_id,
                name,
            } => {
                tx.execute(
                    "INSERT INTO provisional_draft_customers VALUES (?1, 2, ?2, ?3)",
                    (&id, customer_id.value(), name),
                )
                .map_err(sqlite)?;
            }
        }
        {
            let mut insert = tx
                .prepare(
                    "INSERT INTO provisional_draft_lines (draft_id, position, product_id, quantity)
                 VALUES (?1, ?2, ?3, ?4)",
                )
                .map_err(sqlite)?;
            for (position, line) in draft.lines().iter().enumerate() {
                let position =
                    i64::try_from(position).map_err(|_| DraftStorageError::TooManyLines)?;
                insert
                    .execute((
                        &id,
                        position,
                        line.product_id().value().to_string(),
                        line.quantity().get(),
                    ))
                    .map_err(sqlite)?;
            }
        }
        if let Some(row) = draft.crate_row() {
            tx.execute(
                "INSERT INTO provisional_draft_crates VALUES (?1, ?2)",
                (&id, encode_crate_row(row)),
            )
            .map_err(sqlite)?;
        }
        tx.commit().map_err(sqlite)?;
        Ok(())
    }

    fn reopen(&self, id: DraftId) -> Result<Option<ProvisionalDraft>, DraftStorageError> {
        let mut query = self.conn.prepare(
            "SELECT b.bill_date, COALESCE(c.kind, 0), c.customer_id, c.name, k.definition, l.position, l.product_id, l.quantity
             FROM provisional_drafts AS d
             LEFT JOIN provisional_draft_dates AS b ON b.draft_id = d.id
             LEFT JOIN provisional_draft_customers AS c ON c.draft_id = d.id
             LEFT JOIN provisional_draft_lines AS l ON l.draft_id = d.id
             LEFT JOIN provisional_draft_crates AS k ON k.draft_id = d.id
             WHERE d.id = ?1
             ORDER BY l.position",
        ).map_err(sqlite)?;
        let mut rows = query.query([id.value().to_string()]).map_err(sqlite)?;
        let mut decoded = Vec::new();
        while let Some(row) = rows.next().map_err(sqlite)? {
            decoded.push(draft_rows::DraftRow::read(row)?);
        }
        draft_rows::decode(id, decoded)
    }

    fn save_pending(&mut self, request: u64, snapshot: &str) -> Result<(), DraftStorageError> {
        self.insert_pending(request, snapshot)
    }

    fn load_pending(&self, request: u64) -> Result<Option<String>, DraftStorageError> {
        self.select_pending(request)
    }
}
