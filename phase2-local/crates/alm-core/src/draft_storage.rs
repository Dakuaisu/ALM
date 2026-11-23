use std::fmt;
use std::io;

use crate::{DraftId, ProvisionalDraft};

/// Durable draft and pending-approval storage behind a [`crate::LocalSales`].
pub trait DurableDraftStore: Send {
    /// Saves or replaces a draft with the same ID in one transaction.
    ///
    /// # Errors
    /// Returns a storage error if the save cannot be committed.
    fn save(&self, draft: &ProvisionalDraft) -> Result<(), DraftStorageError>;

    /// Reopens a saved draft without changing it.
    ///
    /// # Errors
    /// Returns a storage error on a failed read rather than reporting a missing draft.
    fn reopen(&self, id: DraftId) -> Result<Option<ProvisionalDraft>, DraftStorageError>;

    /// Commits an encoded Owner approval; a duplicate request ID fails.
    ///
    /// # Errors
    /// Returns a storage error if the approval cannot be committed.
    fn save_pending(&self, request: u64, snapshot: &str) -> Result<(), DraftStorageError>;

    /// Loads an encoded Owner approval.
    ///
    /// # Errors
    /// Returns a storage error on a failed or undecodable read.
    fn load_pending(&self, request: u64) -> Result<Option<String>, DraftStorageError>;
}

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
    /// SQLite rejected an open, statement, or transaction; holds the driver's `rusqlite::Error`.
    Sqlite(Box<dyn std::error::Error + Send + Sync>),
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
            Self::Sqlite(err) => Some(err.as_ref()),
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
