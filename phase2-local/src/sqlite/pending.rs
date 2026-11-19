use super::{DraftStorageError, SqliteDraftStore, customer};
use crate::confirmation::RequestId;

impl SqliteDraftStore {
    pub(crate) fn save_pending(
        &self,
        id: RequestId,
        snapshot: &str,
    ) -> Result<(), DraftStorageError> {
        let sql = format!(
            "PRAGMA synchronous = FULL; BEGIN IMMEDIATE; INSERT INTO pending_owner_confirmations VALUES ('{}', {}); COMMIT;",
            id.0,
            customer::sql_text(snapshot)
        );
        self.run(sql.as_bytes(), false).map(|_| ())
    }

    pub(crate) fn load_pending(&self, id: RequestId) -> Result<Option<String>, DraftStorageError> {
        let sql = format!(
            "SELECT snapshot FROM pending_owner_confirmations WHERE request_id = '{}';",
            id.0
        );
        let output = self.run(sql.as_bytes(), true)?;
        if output.is_empty() {
            return Ok(None);
        }
        let text = String::from_utf8(output).map_err(|_| DraftStorageError::InvalidOutput)?;
        Ok(Some(
            text.strip_suffix('\n')
                .ok_or(DraftStorageError::InvalidOutput)?
                .to_owned(),
        ))
    }
}
