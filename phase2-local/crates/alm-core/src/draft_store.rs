use std::error::Error;
use std::fmt;

use crate::{DraftId, ProvisionalDraft};

/// Durable draft and pending-approval storage behind a [`crate::LocalSales`].
pub trait DurableDraftStore: Send {
    /// Saves or replaces a draft with the same ID in one transaction.
    ///
    /// # Errors
    /// Returns a storage failure if the save cannot be committed.
    fn save(&mut self, draft: &ProvisionalDraft) -> Result<(), DraftStoreFailure>;

    /// Reopens a saved draft without changing it.
    ///
    /// # Errors
    /// Returns a storage failure on a failed read rather than reporting a missing draft.
    fn reopen(&self, id: DraftId) -> Result<Option<ProvisionalDraft>, DraftStoreFailure>;

    /// Commits an encoded Owner approval; a duplicate request ID fails.
    ///
    /// # Errors
    /// Returns a storage failure if the approval cannot be committed.
    fn save_pending(&mut self, request: u64, snapshot: &str) -> Result<(), DraftStoreFailure>;

    /// Loads an encoded Owner approval.
    ///
    /// # Errors
    /// Returns a storage failure on a failed or undecodable read.
    fn load_pending(&self, request: u64) -> Result<Option<String>, DraftStoreFailure>;
}

/// A durable store's own error, carried opaquely; `alm-store` puts its `DraftStorageError` here.
#[derive(Debug)]
pub struct DraftStoreFailure(Box<dyn Error + Send + Sync>);

impl DraftStoreFailure {
    pub fn new(error: impl Error + Send + Sync + 'static) -> Self {
        Self(Box::new(error))
    }

    pub fn downcast_ref<E: Error + 'static>(&self) -> Option<&E> {
        self.0.downcast_ref()
    }
}

impl fmt::Display for DraftStoreFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Error for DraftStoreFailure {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.0.source()
    }
}

/// Stored draft, crate-row or approval text that does not decode.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidStoredText;

impl fmt::Display for InvalidStoredText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid SQLite draft output")
    }
}

impl Error for InvalidStoredText {}
