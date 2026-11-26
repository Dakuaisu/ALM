//! Removing what a role no longer allows from the device.
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use alm_core::Role;

use super::{DIRECTORY, ProjectionStore, account_key, role_text};
use crate::DraftStorageError;

impl ProjectionStore {
    /// Drops every projected row and the checkpoint and overwrites their bytes in the file, so
    /// nothing the role no longer allows stays readable on the device.
    ///
    /// # Errors
    /// Fails on storage errors.
    pub fn purge(&mut self) -> Result<(), DraftStorageError> {
        self.conn.pragma_update(None, "secure_delete", true)?;
        self.clear()?;
        self.conn.execute_batch(
            "PRAGMA wal_checkpoint(TRUNCATE); VACUUM; PRAGMA wal_checkpoint(TRUNCATE);",
        )?;
        Ok(())
    }

    /// Deletes the projection file of `account` for `role` (and its journal files), if present.
    /// The caller must not hold it open.
    ///
    /// # Errors
    /// Rejects invalid IDs and unsafe locations; I/O errors.
    pub fn remove_scoped(
        device_root: &Path,
        account: &str,
        role: Role,
    ) -> Result<(), DraftStorageError> {
        let key = account_key(account)?;
        if !device_root.is_absolute() {
            return Err(DraftStorageError::InvalidPath);
        }
        let base = device_root
            .canonicalize()?
            .join(DIRECTORY)
            .join(format!("account-{key}-{}.sqlite3", role_text(role)));
        for suffix in ["", "-wal", "-shm"] {
            let mut name = base.clone().into_os_string();
            name.push(suffix);
            match fs::remove_file(&name) {
                Ok(()) => {}
                Err(error) if error.kind() == ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
        Ok(())
    }
}
