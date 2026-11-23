use std::path::Path;

use alm_core::{
    DraftId, DraftStorageError, DurableDraftStore, LocalSales, Money, ProvisionalDraft,
};

use crate::sqlite::SqliteDraftStore;

/// A draft-only capability; it has no Owner session, posting, or profit API.
pub struct ManagerDraftStore(SqliteDraftStore);

/// SQLite-backed constructors for [`LocalSales`], kept here so `alm-core` stays I/O-free.
pub trait SqliteSales: Sized {
    /// Opens an explicit legacy, single-account SQLite file for the trusted fixture.
    ///
    /// # Errors
    /// Returns a storage error if SQLite cannot initialize the file.
    fn open_sqlite(path: &Path, total_profit: Money) -> Result<Self, DraftStorageError>;

    /// Opens drafts for a trusted synthetic account under an existing absolute device root.
    /// Account IDs are case-sensitive. This selects local storage, not an authenticated role.
    ///
    /// # Errors
    /// Rejects invalid IDs, unsafe locations, and databases not bound to this account.
    fn open_manager_account(
        device_root: &Path,
        account: &str,
    ) -> Result<ManagerDraftStore, DraftStorageError>;

    /// Opens an explicit, unscoped path for the legacy single-account synthetic demo.
    ///
    /// # Errors
    /// Returns a storage error if the legacy database cannot be opened.
    fn open_legacy_manager_sqlite(path: &Path) -> Result<ManagerDraftStore, DraftStorageError>;
}

impl SqliteSales for LocalSales {
    fn open_sqlite(path: &Path, total_profit: Money) -> Result<Self, DraftStorageError> {
        Ok(Self::with_durable_drafts(
            Box::new(SqliteDraftStore::open(path)?),
            total_profit,
        ))
    }

    fn open_manager_account(
        device_root: &Path,
        account: &str,
    ) -> Result<ManagerDraftStore, DraftStorageError> {
        SqliteDraftStore::open_scoped(device_root, account).map(ManagerDraftStore)
    }

    fn open_legacy_manager_sqlite(path: &Path) -> Result<ManagerDraftStore, DraftStorageError> {
        SqliteDraftStore::open(path).map(ManagerDraftStore)
    }
}

impl ManagerDraftStore {
    pub fn save_draft(&mut self, draft: ProvisionalDraft) -> Result<(), DraftStorageError> {
        self.0.save(&draft)
    }

    pub fn reopen_draft(&self, id: DraftId) -> Result<Option<ProvisionalDraft>, DraftStorageError> {
        self.0.reopen(id)
    }
}
