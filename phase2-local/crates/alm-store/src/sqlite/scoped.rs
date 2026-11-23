use std::fs::{self, OpenOptions};
use std::io::ErrorKind;
use std::path::Path;

use super::{DraftStorageError, SqliteDraftStore, connect, schema, sqlite, text};

const DIRECTORY: &str = "manager-accounts";
const SCOPE_TABLE: &str = "manager_account_scope";

fn account_key(account: &str) -> Result<String, DraftStorageError> {
    let bytes = account.as_bytes();
    if !(1..=64).contains(&bytes.len())
        || !bytes
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
    {
        return Err(DraftStorageError::InvalidAccountScope);
    }
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut key = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        key.push(char::from(HEX[usize::from(byte >> 4)]));
        key.push(char::from(HEX[usize::from(byte & 15)]));
    }
    Ok(key)
}

fn check_file_slot(path: &Path) -> Result<(), DraftStorageError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    if !metadata.is_file() {
        return Err(DraftStorageError::InvalidPath);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.nlink() != 1 {
            return Err(DraftStorageError::InvalidPath);
        }
    }
    Ok(())
}

impl SqliteDraftStore {
    pub(crate) fn open_scoped(
        device_root: &Path,
        account: &str,
    ) -> Result<Self, DraftStorageError> {
        let key = account_key(account)?;
        if !device_root.is_absolute() || device_root.as_os_str().as_encoded_bytes().contains(&0) {
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
        let filename = format!("account-{key}.sqlite3");
        let path = directory.join(&filename);
        for suffix in ["", "-journal", "-wal", "-shm"] {
            check_file_slot(&directory.join(format!("{filename}{suffix}")))?;
        }
        let reservation = OpenOptions::new().write(true).create_new(true).open(&path);
        match reservation {
            Ok(file) => {
                drop(file);
                let mut store = Self {
                    conn: connect(&path).map_err(sqlite)?,
                };
                store.initialize_scoped(&key)?;
                Ok(store)
            }
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                check_file_slot(&path)?;
                let store = connect(&path)
                    .map(|conn| Self { conn })
                    .map_err(|_| DraftStorageError::AccountStoreMismatch)?;
                store.check_scope(&key)?;
                Ok(store)
            }
            Err(error) => Err(error.into()),
        }
    }

    fn initialize_scoped(&mut self, key: &str) -> Result<(), DraftStorageError> {
        let tx = self.begin().map_err(sqlite)?;
        tx.execute_batch(schema::DRAFT_TABLES).map_err(sqlite)?;
        tx.execute_batch(&format!(
            "CREATE TABLE {SCOPE_TABLE} (account_hex TEXT PRIMARY KEY NOT NULL) STRICT;"
        ))
        .map_err(sqlite)?;
        tx.execute(&format!("INSERT INTO {SCOPE_TABLE} VALUES (?1)"), [key])
            .map_err(sqlite)?;
        tx.commit().map_err(sqlite)
    }

    fn check_scope(&self, key: &str) -> Result<(), DraftStorageError> {
        let bound = (|| {
            let mut query = self
                .conn
                .prepare(&format!("SELECT account_hex FROM {SCOPE_TABLE}"))
                .map_err(sqlite)?;
            let mut rows = query.query([]).map_err(sqlite)?;
            let mut keys = Vec::new();
            while let Some(row) = rows.next().map_err(sqlite)? {
                keys.push(text(row, 0)?);
            }
            Ok::<_, DraftStorageError>(keys)
        })();
        match bound {
            Ok(keys) if keys == [Some(key.to_owned())] => Ok(()),
            Ok(_) | Err(DraftStorageError::Sqlite(_) | DraftStorageError::InvalidOutput) => {
                Err(DraftStorageError::AccountStoreMismatch)
            }
            Err(error) => Err(error),
        }
    }
}
