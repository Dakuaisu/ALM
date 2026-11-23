use rusqlite::OptionalExtension;

use super::{DraftStorageError, SqliteDraftStore, text};
use crate::confirmation::RequestId;

impl SqliteDraftStore {
    pub(crate) fn save_pending(
        &self,
        id: RequestId,
        snapshot: &str,
    ) -> Result<(), DraftStorageError> {
        let tx = self.begin()?;
        tx.execute(
            "INSERT INTO pending_owner_confirmations VALUES (?1, ?2)",
            (id.0.to_string(), snapshot),
        )?;
        tx.commit()?;
        Ok(())
    }

    pub(crate) fn load_pending(&self, id: RequestId) -> Result<Option<String>, DraftStorageError> {
        let snapshot = self
            .conn
            .query_row(
                "SELECT snapshot FROM pending_owner_confirmations WHERE request_id = ?1",
                [id.0.to_string()],
                |row| Ok(text(row, 0)),
            )
            .optional()?;
        match snapshot {
            None => Ok(None),
            Some(value) => value?.ok_or(DraftStorageError::InvalidOutput).map(Some),
        }
    }
}
