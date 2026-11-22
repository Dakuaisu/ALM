use std::path::Path;

use crate::{DraftId, DraftStorageError, LocalSales, ProvisionalDraft, SqliteDraftStore};

/// A draft-only capability; it has no Owner session, posting, or profit API.
pub struct ManagerDraftStore(SqliteDraftStore);

impl LocalSales {
    /// Opens drafts for a trusted synthetic account under an existing absolute device root.
    /// Account IDs are case-sensitive. This selects local storage, not an authenticated role.
    ///
    /// # Errors
    /// Rejects invalid IDs, unsafe locations, and databases not bound to this account.
    pub fn open_manager_account(
        device_root: &Path,
        account: &str,
    ) -> Result<ManagerDraftStore, DraftStorageError> {
        SqliteDraftStore::open_scoped(device_root, account).map(ManagerDraftStore)
    }

    /// Opens an explicit, unscoped path for the legacy single-account synthetic demo.
    ///
    /// # Errors
    /// Returns a storage error if the legacy database cannot be opened.
    pub fn open_legacy_manager_sqlite(path: &Path) -> Result<ManagerDraftStore, DraftStorageError> {
        SqliteDraftStore::open(path).map(ManagerDraftStore)
    }
}

impl ManagerDraftStore {
    pub fn save_draft(&self, draft: ProvisionalDraft) -> Result<(), DraftStorageError> {
        self.0.save(&draft)
    }

    pub fn reopen_draft(&self, id: DraftId) -> Result<Option<ProvisionalDraft>, DraftStorageError> {
        self.0.reopen(id)
    }
}
