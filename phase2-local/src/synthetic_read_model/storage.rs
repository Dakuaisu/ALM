use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::Records;
use crate::Role;
use crate::confirmation::{ConfirmationFixture, ProductEntryError};

#[derive(Debug)]
pub enum Error {
    AccessDenied,
    InvalidArchive,
    ProductEntry(ProductEntryError),
    Io(std::io::Error),
    Json(serde_json::Error),
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}
impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Archive {
    version: u32,
    records: Records,
}

fn owner(role: Role) -> Result<(), Error> {
    match role {
        Role::Owner => Ok(()),
        Role::Manager => Err(Error::AccessDenied),
    }
}

pub struct Store {
    path: PathBuf,
    records: Records,
}

impl Store {
    pub fn create(path: &Path, role: Role, records: Records) -> Result<Self, Error> {
        owner(role)?;
        let data = serde_json::to_vec(&Archive {
            version: 1,
            records: records.clone(),
        })?;
        let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
        file.write_all(&data)?;
        file.sync_all()?;
        Ok(Self {
            path: path.into(),
            records,
        })
    }

    pub fn open(path: &Path, role: Role) -> Result<Self, Error> {
        owner(role)?;
        let archive: Archive = serde_json::from_slice(&fs::read(path)?)?;
        if archive.version != 1 {
            return Err(Error::InvalidArchive);
        }
        Ok(Self {
            path: path.into(),
            records: archive.records,
        })
    }

    pub fn restore(path: &Path, role: Role, bytes: &[u8]) -> Result<Self, Error> {
        owner(role)?;
        let archive: Archive = serde_json::from_slice(bytes)?;
        if archive.version != 1 {
            return Err(Error::InvalidArchive);
        }
        Self::create(path, role, archive.records)
    }

    pub fn records(&self, role: Role) -> Result<&Records, Error> {
        owner(role)?;
        Ok(&self.records)
    }

    pub fn backup(&self, role: Role) -> Result<Vec<u8>, Error> {
        owner(role)?;
        Ok(serde_json::to_vec(&Archive {
            version: 1,
            records: self.records.clone(),
        })?)
    }

    pub fn apply_product_settings(
        &self,
        role: Role,
        fixture: &mut ConfirmationFixture,
    ) -> Result<(), Error> {
        let default = self.records(role)?.reorder_default;
        fixture
            .save_product_reorder_default(role, default)
            .map_err(Error::ProductEntry)
    }

    pub fn set_reorder_default(&mut self, role: Role, value: u32) -> Result<(), Error> {
        owner(role)?;
        let mut records = self.records.clone();
        records.reorder_default = value;
        let data = serde_json::to_vec(&Archive {
            version: 1,
            records: records.clone(),
        })?;
        let staging = self.path.with_extension("setting-stage");
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&staging)?;
        let result = (|| -> Result<(), Error> {
            file.write_all(&data)?;
            file.sync_all()?;
            fs::rename(&staging, &self.path)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(staging);
        }
        result?;
        self.records = records;
        Ok(())
    }
}
