use rusqlite::OptionalExtension;

use super::{DraftStorageError, SqliteDraftStore, text};

impl SqliteDraftStore {
    pub(super) fn insert_pending(
        &mut self,
        request: u64,
        snapshot: &str,
    ) -> Result<(), DraftStorageError> {
        let tx = self.begin()?;
        tx.execute(
            "INSERT INTO pending_owner_confirmations VALUES (?1, ?2)",
            (request.to_string(), snapshot),
        )?;
        Ok(tx.commit()?)
    }

    pub(super) fn select_pending(&self, request: u64) -> Result<Option<String>, DraftStorageError> {
        let snapshot = self
            .conn
            .query_row(
                "SELECT snapshot FROM pending_owner_confirmations WHERE request_id = ?1",
                [request.to_string()],
                |row| Ok(text(row, 0)),
            )
            .optional()?;
        match snapshot {
            None => Ok(None),
            Some(value) => value?.ok_or(DraftStorageError::InvalidOutput).map(Some),
        }
    }
}
