use std::fmt;
use std::io;

use alm_core::InvalidStoredText;

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

impl From<InvalidStoredText> for DraftStorageError {
    fn from(_: InvalidStoredText) -> Self {
        Self::InvalidOutput
    }
}
