use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

mod standalone_gst;

use super::{AccountScope, BusinessProfile, HistoricalGst, Records};
use super::{ArchivedPurchaseOrder, ScopedArchiveInput};
use crate::Role;
use crate::confirmation::{ConfirmationFixture, GstBill, ProductEntryError};

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    purchase_orders: Option<Vec<ArchivedPurchaseOrder>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    standalone_gst: Option<Vec<GstBill>>,
}

impl Archive {
    fn new(records: Records) -> Self {
        Self {
            version: 1,
            records,
            profile: BusinessProfile::default(),
            scope: None,
            gst: None,
            purchase_orders: None,
            standalone_gst: None,
        }
    }

    fn validate(&self) -> Result<(), Error> {
        if self.version != 1 {
            return Err(Error::InvalidArchive);
        }
        if let Some(scope) = &self.scope {
            scope.validate(&self.records)?;
        }
        super::purchase_order::validate_orders(self.purchase_orders.as_deref())?;
        let mut ids = std::collections::BTreeSet::new();
        for bill in self.standalone_gst.iter().flatten() {
            if !ids.insert(bill.id) || bill.validate_saved().is_err() {
                return Err(Error::InvalidArchive);
            }
        }
        Ok(())
    }
}

pub(super) fn owner(role: Role) -> Result<(), Error> {
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
    pub(super) purchase_orders: Option<Vec<ArchivedPurchaseOrder>>,
    standalone_gst: Option<Vec<GstBill>>,
}

impl Store {
    pub fn create(path: &Path, role: Role, records: Records) -> Result<Self, Error> {
        owner(role)?;
        Self::create_archive(path, Archive::new(records))
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
                scope: Some(scope),
                ..Archive::new(records)
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
                scope: Some(scope),
                gst: Some(gst),
                ..Archive::new(records)
            },
        )
    }

    pub fn create_scoped_with_gst_and_purchase_orders(
        path: &Path,
        role: Role,
        input: ScopedArchiveInput,
    ) -> Result<Self, Error> {
        owner(role)?;
        Self::create_archive(
            path,
            Archive {
                scope: Some(input.scope),
                gst: Some(input.gst),
                purchase_orders: Some(input.purchase_orders),
                ..Archive::new(input.records)
            },
        )
    }

    fn create_archive(path: &Path, archive: Archive) -> Result<Self, Error> {
        archive.validate()?;
        let data = serde_json::to_vec(&archive)?;
        let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
        file.write_all(&data)?;
        file.sync_all()?;
        Ok(Self::from_archive(path, archive))
    }

    fn from_archive(path: &Path, archive: Archive) -> Self {
        Self {
            path: path.into(),
            records: archive.records,
            profile: archive.profile,
            scope: archive.scope,
            gst: archive.gst,
            purchase_orders: archive.purchase_orders,
            standalone_gst: archive.standalone_gst,
        }
    }

    pub fn open(path: &Path, role: Role) -> Result<Self, Error> {
        owner(role)?;
        let archive: Archive = serde_json::from_slice(&fs::read(path)?)?;
        archive.validate()?;
        Ok(Self::from_archive(path, archive))
    }

    pub fn restore(path: &Path, role: Role, bytes: &[u8]) -> Result<Self, Error> {
        owner(role)?;
        Self::create_archive(path, serde_json::from_slice(bytes)?)
    }

    pub fn records(&self, role: Role) -> Result<&Records, Error> {
        owner(role)?;
        Ok(&self.records)
    }

    pub fn backup(&self, role: Role) -> Result<Vec<u8>, Error> {
        owner(role)?;
        self.archive_bytes(&self.records, &self.profile)
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

    fn archive_bytes(
        &self,
        records: &Records,
        profile: &BusinessProfile,
    ) -> Result<Vec<u8>, Error> {
        Ok(serde_json::to_vec(&Archive {
            version: 1,
            records: records.clone(),
            profile: profile.clone(),
            scope: self.scope.clone(),
            gst: self.gst.clone(),
            purchase_orders: self.purchase_orders.clone(),
            standalone_gst: self.standalone_gst.clone(),
        })?)
    }

    fn save_archive(&mut self, records: Records, profile: BusinessProfile) -> Result<(), Error> {
        let data = self.archive_bytes(&records, &profile)?;
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
