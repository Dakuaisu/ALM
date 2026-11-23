//! Synthetic, Owner-scoped read model persisted as one archive file; it cannot post sales.
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::ops::Deref;
use std::path::{Path, PathBuf};

use alm_core::Role;
use alm_core::confirmation::GstBill;
pub use alm_core::synthetic_read_model::*;

#[cfg(test)]
mod purchase_order_tests;

/// A [`ReadModel`] backed by one JSON archive file; every save replaces the file atomically.
pub struct Store {
    path: PathBuf,
    model: ReadModel,
}

impl Deref for Store {
    type Target = ReadModel;

    fn deref(&self) -> &ReadModel {
        &self.model
    }
}

impl Store {
    fn at(path: &Path, model: Result<ReadModel, Error>) -> Result<Self, Error> {
        model.map(|model| Self {
            path: path.into(),
            model,
        })
    }

    pub fn create(path: &Path, role: Role, records: Records) -> Result<Self, Error> {
        Self::at(
            path,
            ReadModel::create(role, records, |bytes| create_new(path, bytes)),
        )
    }

    pub fn create_scoped(
        path: &Path,
        role: Role,
        records: Records,
        scope: AccountScope,
    ) -> Result<Self, Error> {
        Self::at(
            path,
            ReadModel::create_scoped(role, records, scope, |bytes| create_new(path, bytes)),
        )
    }

    pub fn create_scoped_with_gst(
        path: &Path,
        role: Role,
        records: Records,
        scope: AccountScope,
        gst: Vec<HistoricalGst>,
    ) -> Result<Self, Error> {
        Self::at(
            path,
            ReadModel::create_scoped_with_gst(role, records, scope, gst, |bytes| {
                create_new(path, bytes)
            }),
        )
    }

    pub fn create_scoped_with_gst_and_purchase_orders(
        path: &Path,
        role: Role,
        input: ScopedArchiveInput,
    ) -> Result<Self, Error> {
        Self::at(
            path,
            ReadModel::create_scoped_with_gst_and_purchase_orders(role, input, |bytes| {
                create_new(path, bytes)
            }),
        )
    }

    pub fn create_with_standalone_gst(
        path: &Path,
        role: Role,
        records: Records,
        bills: Vec<GstBill>,
    ) -> Result<Self, Error> {
        Self::at(
            path,
            ReadModel::create_with_standalone_gst(role, records, bills, |bytes| {
                create_new(path, bytes)
            }),
        )
    }

    pub fn open(path: &Path, role: Role) -> Result<Self, Error> {
        Self::at(path, ReadModel::open(role, || fs::read(path)))
    }

    pub fn restore(path: &Path, role: Role, bytes: &[u8]) -> Result<Self, Error> {
        Self::at(
            path,
            ReadModel::restore(role, bytes, |bytes| create_new(path, bytes)),
        )
    }

    pub fn set_reorder_default(&mut self, role: Role, value: u32) -> Result<(), Error> {
        let path = &self.path;
        self.model
            .set_reorder_default(role, value, |bytes| replace(path, bytes))
    }

    pub fn save_business_profile(
        &mut self,
        role: Role,
        profile: &BusinessProfile,
        modified_at: u64,
        device: &str,
    ) -> Result<(), Error> {
        let path = &self.path;
        self.model
            .save_business_profile(role, profile, modified_at, device, |bytes| {
                replace(path, bytes)
            })
    }
}

fn create_new(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

fn replace(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let staging = path.with_extension("setting-stage");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&staging)?;
    let result = file
        .write_all(bytes)
        .and_then(|()| file.sync_all())
        .and_then(|()| fs::rename(&staging, path));
    if result.is_err() {
        let _ = fs::remove_file(staging);
    }
    result
}
