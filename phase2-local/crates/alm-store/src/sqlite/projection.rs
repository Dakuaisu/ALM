//! Per-account, per-role read-only projection of posted data received from the authority.
//! A batch is applied with its checkpoint in one transaction; the file can be dropped and
//! rebuilt from the authority at any time. Owner-only tables exist only in an Owner file, and a
//! Manager file refuses owner values, so costs and FIFO values never reach a Manager device.
use std::fs::{self, OpenOptions};
use std::io::ErrorKind;
use std::path::Path;

use rusqlite::{Connection, OptionalExtension, TransactionBehavior};

use alm_core::Role;
use alm_core::projection::{ProjectionBatch, ProjectionSnapshot};

use super::{DraftStorageError, connect, scoped::account_key};

mod books;
mod purge;
mod read;
mod schema;
mod write;
use read::{date, id, money, read_snapshot};
use schema::{BOOK_TABLES, OWNER_BOOK_TABLES, OWNER_TABLES, SHARED_TABLES, clear_rows};
use write::write_snapshot;

const DIRECTORY: &str = "projections";

const fn role_text(role: Role) -> &'static str {
    match role {
        Role::Owner => "owner",
        Role::Manager => "manager",
    }
}

#[derive(Debug)]
pub enum ProjectionError {
    Storage(DraftStorageError),
    /// The batch is for another role, older than the stored checkpoint, or carries data this
    /// role's projection must not hold.
    Rejected,
}

impl From<DraftStorageError> for ProjectionError {
    fn from(error: DraftStorageError) -> Self {
        Self::Storage(error)
    }
}

impl From<rusqlite::Error> for ProjectionError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Storage(error.into())
    }
}

/// The stored projection with the checkpoint it reflects.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredProjection {
    pub checkpoint: u64,
    pub as_of_ms: u64,
    pub snapshot: ProjectionSnapshot,
}

pub struct ProjectionStore {
    conn: Connection,
    role: Role,
}

impl ProjectionStore {
    /// Opens (or creates) the projection file of `account` for `role` under `device_root`.
    ///
    /// # Errors
    /// Rejects invalid IDs, unsafe locations and a file bound to another account or role.
    pub fn open_scoped(
        device_root: &Path,
        account: &str,
        role: Role,
    ) -> Result<Self, DraftStorageError> {
        let key = account_key(account)?;
        if !device_root.is_absolute() {
            return Err(DraftStorageError::InvalidPath);
        }
        let directory = device_root.canonicalize()?.join(DIRECTORY);
        match fs::create_dir(&directory) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error.into()),
        }
        if !fs::symlink_metadata(&directory)?.is_dir() {
            return Err(DraftStorageError::InvalidPath);
        }
        let path = directory.join(format!("account-{key}-{}.sqlite3", role_text(role)));
        let created = match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => {
                drop(file);
                true
            }
            Err(error) if error.kind() == ErrorKind::AlreadyExists => false,
            Err(error) => return Err(error.into()),
        };
        if !fs::symlink_metadata(&path)?.is_file() {
            return Err(DraftStorageError::InvalidPath);
        }
        let mut conn = connect(&path)?;
        if created {
            let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
            tx.execute_batch(SHARED_TABLES)?;
            tx.execute_batch(BOOK_TABLES)?;
            if role == Role::Owner {
                tx.execute_batch(OWNER_TABLES)?;
                tx.execute_batch(OWNER_BOOK_TABLES)?;
            }
            tx.execute(
                "INSERT INTO projection_scope VALUES (?1, ?2)",
                (&key, role_text(role)),
            )?;
            tx.commit()?;
        } else {
            let bound: Option<(String, String)> = conn
                .query_row(
                    "SELECT account_hex, role FROM projection_scope",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()
                .map_err(|_| DraftStorageError::AccountStoreMismatch)?;
            if bound != Some((key, role_text(role).to_owned())) {
                return Err(DraftStorageError::AccountStoreMismatch);
            }
        }
        let current: i64 = conn.query_row(
            "SELECT count(*) FROM sqlite_master WHERE name = 'projected_products'",
            [],
            |row| row.get(0),
        )?;
        if current == 0 {
            // An older file: add the new tables and rebuild from the authority on next pull.
            let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
            tx.execute_batch(BOOK_TABLES)?;
            if role == Role::Owner {
                tx.execute_batch(OWNER_BOOK_TABLES)?;
            }
            clear_rows(&tx, role)?;
            tx.execute("DELETE FROM projection_checkpoint", [])?;
            tx.commit()?;
        }
        conn.pragma_update(None, "journal_mode", "WAL")?;
        Ok(Self { conn, role })
    }

    pub const fn role(&self) -> Role {
        self.role
    }

    /// The checkpoint the stored rows reflect, if any batch was applied.
    ///
    /// # Errors
    /// Fails on storage errors.
    pub fn checkpoint(&self) -> Result<Option<(u64, u64)>, DraftStorageError> {
        let row: Option<(i64, i64)> = self
            .conn
            .query_row(
                "SELECT checkpoint, as_of_ms FROM projection_checkpoint",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        row.map(|(c, a)| Ok((unsigned(c)?, unsigned(a)?)))
            .transpose()
    }

    /// Applies a batch and its checkpoint in one transaction: a snapshot replaces every row;
    /// without one only the checkpoint time moves.
    ///
    /// # Errors
    /// `Rejected` for another role, an older checkpoint or owner values on a Manager file;
    /// nothing changes on any error.
    pub fn apply(&mut self, batch: &ProjectionBatch) -> Result<(), ProjectionError> {
        if batch.role != self.role {
            return Err(ProjectionError::Rejected);
        }
        if let Some((stored, _)) = self.checkpoint()?
            && batch.checkpoint < stored
        {
            return Err(ProjectionError::Rejected);
        }
        if self.role != Role::Owner && batch.snapshot.as_ref().is_some_and(|s| s.owner.is_some()) {
            return Err(ProjectionError::Rejected);
        }
        let checkpoint = signed(batch.checkpoint)?;
        let as_of = signed(batch.as_of_ms)?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(snapshot) = &batch.snapshot {
            write_snapshot(&tx, self.role, snapshot)?;
        }
        tx.execute(
            "INSERT INTO projection_checkpoint (id, checkpoint, as_of_ms) VALUES (1, ?1, ?2)
             ON CONFLICT(id) DO UPDATE SET checkpoint = ?1, as_of_ms = ?2",
            (checkpoint, as_of),
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Drops every projected row and the checkpoint, so the next pull rebuilds from scratch.
    ///
    /// # Errors
    /// Fails on storage errors.
    pub fn clear(&mut self) -> Result<(), DraftStorageError> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        clear_rows(&tx, self.role)?;
        tx.execute("DELETE FROM projection_checkpoint", [])?;
        tx.commit()?;
        Ok(())
    }

    /// The stored projection, if any batch was applied.
    ///
    /// # Errors
    /// Fails on storage errors or corrupt rows.
    pub fn load(&self) -> Result<Option<StoredProjection>, DraftStorageError> {
        let Some((checkpoint, as_of_ms)) = self.checkpoint()? else {
            return Ok(None);
        };
        Ok(Some(StoredProjection {
            checkpoint,
            as_of_ms,
            snapshot: read_snapshot(&self.conn, self.role)?,
        }))
    }
}

fn signed(value: u64) -> Result<i64, ProjectionError> {
    i64::try_from(value).map_err(|_| ProjectionError::Rejected)
}

fn unsigned(value: i64) -> Result<u64, DraftStorageError> {
    u64::try_from(value).map_err(|_| DraftStorageError::InvalidOutput)
}
