use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{AccountScope, BusinessProfile, HistoricalGst, Records};
use crate::Role;
use crate::confirmation::{ConfirmationFixture, ProductEntryError};

#[derive(Debug)]
pub enum Error {
    AccessDenied,
    InvalidArchive,
    VersionOverflow,
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
    #[serde(default)]
    profile: BusinessProfile,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    scope: Option<AccountScope>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    gst: Option<Vec<HistoricalGst>>,
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
    profile: BusinessProfile,
    pub(super) scope: Option<AccountScope>,
    pub(super) gst: Option<Vec<HistoricalGst>>,
}

impl Store {
    pub fn create(path: &Path, role: Role, records: Records) -> Result<Self, Error> {
        owner(role)?;
        Self::create_archive(
            path,
            Archive {
                version: 1,
                records,
                profile: BusinessProfile::default(),
                scope: None,
                gst: None,
            },
        )
    }

    pub fn create_scoped(
        path: &Path,
        role: Role,
        records: Records,
        scope: AccountScope,
    ) -> Result<Self, Error> {
        owner(role)?;
        Self::create_archive(
            path,
            Archive {
                version: 1,
                records,
                profile: BusinessProfile::default(),
                scope: Some(scope),
                gst: None,
            },
        )
    }

    pub fn create_scoped_with_gst(
        path: &Path,
        role: Role,
        records: Records,
        scope: AccountScope,
        gst: Vec<HistoricalGst>,
    ) -> Result<Self, Error> {
        owner(role)?;
        Self::create_archive(
            path,
            Archive {
                version: 1,
                records,
                profile: BusinessProfile::default(),
                scope: Some(scope),
                gst: Some(gst),
            },
        )
    }

    fn create_archive(path: &Path, archive: Archive) -> Result<Self, Error> {
        if let Some(scope) = &archive.scope {
            scope.validate(&archive.records)?;
        }
        let data = serde_json::to_vec(&archive)?;
        let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
        file.write_all(&data)?;
        file.sync_all()?;
        Ok(Self {
            path: path.into(),
            records: archive.records,
            profile: archive.profile,
            scope: archive.scope,
            gst: archive.gst,
        })
    }

    pub fn open(path: &Path, role: Role) -> Result<Self, Error> {
        owner(role)?;
        let archive: Archive = serde_json::from_slice(&fs::read(path)?)?;
        if archive.version != 1 {
            return Err(Error::InvalidArchive);
        }
        if let Some(scope) = &archive.scope {
            scope.validate(&archive.records)?;
        }
        Ok(Self {
            path: path.into(),
            records: archive.records,
            profile: archive.profile,
            scope: archive.scope,
            gst: archive.gst,
        })
    }

    pub fn restore(path: &Path, role: Role, bytes: &[u8]) -> Result<Self, Error> {
        owner(role)?;
        let archive: Archive = serde_json::from_slice(bytes)?;
        if archive.version != 1 {
            return Err(Error::InvalidArchive);
        }
        Self::create_archive(path, archive)
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
            profile: self.profile.clone(),
            scope: self.scope.clone(),
            gst: self.gst.clone(),
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
        self.save_archive(records, self.profile.clone())
    }

    pub fn business_profile(&self, role: Role) -> Result<&BusinessProfile, Error> {
        owner(role)?;
        Ok(&self.profile)
    }

    pub fn save_business_profile(
        &mut self,
        role: Role,
        profile: &BusinessProfile,
        modified_at: u64,
        device: &str,
    ) -> Result<(), Error> {
        owner(role)?;
        let mut profile = profile.clone();
        profile.version = self
            .profile
            .version
            .checked_add(1)
            .ok_or(Error::VersionOverflow)?;
        profile.modified_at = modified_at;
        profile.device = device.to_owned();
        self.save_archive(self.records.clone(), profile)
    }

    fn save_archive(&mut self, records: Records, profile: BusinessProfile) -> Result<(), Error> {
        let data = serde_json::to_vec(&Archive {
            version: 1,
            records: records.clone(),
            profile: profile.clone(),
            scope: self.scope.clone(),
            gst: self.gst.clone(),
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
        self.profile = profile;
        Ok(())
    }
}
