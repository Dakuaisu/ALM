use std::io;

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

/// The Owner-scoped archive state; the caller supplies the archive's durable bytes.
pub struct ReadModel {
    records: Records,
    profile: BusinessProfile,
    pub(super) scope: Option<AccountScope>,
    pub(super) gst: Option<Vec<HistoricalGst>>,
    pub(super) purchase_orders: Option<Vec<ArchivedPurchaseOrder>>,
    standalone_gst: Option<Vec<GstBill>>,
}

impl ReadModel {
    /// Creates an archive; `create_new` must fail if the archive already exists.
    pub fn create(
        role: Role,
        records: Records,
        create_new: impl FnOnce(&[u8]) -> io::Result<()>,
    ) -> Result<Self, Error> {
        owner(role)?;
        Self::create_archive(Archive::new(records), create_new)
    }

    pub fn create_scoped(
        role: Role,
        records: Records,
        scope: AccountScope,
        create_new: impl FnOnce(&[u8]) -> io::Result<()>,
    ) -> Result<Self, Error> {
        owner(role)?;
        Self::create_archive(
            Archive {
                scope: Some(scope),
                ..Archive::new(records)
            },
            create_new,
        )
    }

    pub fn create_scoped_with_gst(
        role: Role,
        records: Records,
        scope: AccountScope,
        gst: Vec<HistoricalGst>,
        create_new: impl FnOnce(&[u8]) -> io::Result<()>,
    ) -> Result<Self, Error> {
        owner(role)?;
        Self::create_archive(
            Archive {
                scope: Some(scope),
                gst: Some(gst),
                ..Archive::new(records)
            },
            create_new,
        )
    }

    pub fn create_scoped_with_gst_and_purchase_orders(
        role: Role,
        input: ScopedArchiveInput,
        create_new: impl FnOnce(&[u8]) -> io::Result<()>,
    ) -> Result<Self, Error> {
        owner(role)?;
        Self::create_archive(
            Archive {
                scope: Some(input.scope),
                gst: Some(input.gst),
                purchase_orders: Some(input.purchase_orders),
                ..Archive::new(input.records)
            },
            create_new,
        )
    }

    fn create_archive(
        archive: Archive,
        create_new: impl FnOnce(&[u8]) -> io::Result<()>,
    ) -> Result<Self, Error> {
        archive.validate()?;
        let data = serde_json::to_vec(&archive)?;
        create_new(&data)?;
        Ok(Self::from_archive(archive))
    }

    fn from_archive(archive: Archive) -> Self {
        Self {
            records: archive.records,
            profile: archive.profile,
            scope: archive.scope,
            gst: archive.gst,
            purchase_orders: archive.purchase_orders,
            standalone_gst: archive.standalone_gst,
        }
    }

    /// Opens an archive; `read` runs only after the Owner check.
    pub fn open(role: Role, read: impl FnOnce() -> io::Result<Vec<u8>>) -> Result<Self, Error> {
        owner(role)?;
        let archive: Archive = serde_json::from_slice(&read()?)?;
        archive.validate()?;
        Ok(Self::from_archive(archive))
    }

    pub fn restore(
        role: Role,
        bytes: &[u8],
        create_new: impl FnOnce(&[u8]) -> io::Result<()>,
    ) -> Result<Self, Error> {
        owner(role)?;
        Self::create_archive(serde_json::from_slice(bytes)?, create_new)
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

    /// Changes the default only after `replace` durably stores the new archive.
    pub fn set_reorder_default(
        &mut self,
        role: Role,
        value: u32,
        replace: impl FnOnce(&[u8]) -> io::Result<()>,
    ) -> Result<(), Error> {
        owner(role)?;
        let mut records = self.records.clone();
        records.reorder_default = value;
        self.save_archive(records, self.profile.clone(), replace)
    }

    pub fn business_profile(&self, role: Role) -> Result<&BusinessProfile, Error> {
        owner(role)?;
        Ok(&self.profile)
    }

    /// Changes the profile only after `replace` durably stores the new archive.
    pub fn save_business_profile(
        &mut self,
        role: Role,
        profile: &BusinessProfile,
        modified_at: u64,
        device: &str,
        replace: impl FnOnce(&[u8]) -> io::Result<()>,
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
        self.save_archive(self.records.clone(), profile, replace)
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

    fn save_archive(
        &mut self,
        records: Records,
        profile: BusinessProfile,
        replace: impl FnOnce(&[u8]) -> io::Result<()>,
    ) -> Result<(), Error> {
        let data = self.archive_bytes(&records, &profile)?;
        replace(&data)?;
        self.records = records;
        self.profile = profile;
        Ok(())
    }
}
