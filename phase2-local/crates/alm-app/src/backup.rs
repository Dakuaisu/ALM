//! Backup & Restore (HTML L9273–9330 "💾 Backup & Restore"): backups are archive files in the
//! app-data `backups` folder; a restore validates the archive and opens it at a fresh path in
//! `restored`, never touching the live shop archive.
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use alm_core::Role;
use alm_store::synthetic_read_model::Store;
use tauri::State;

use crate::{AppState, Trial, debug, with};

fn valid_name(name: &str) -> bool {
    name.ends_with(".json")
        && !name.starts_with('.')
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
}

fn create_new(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

/// The first unused `<stem>-<n>.json` in `dir`.
fn fresh(dir: &Path, stem: &str) -> PathBuf {
    (1..)
        .map(|n| dir.join(format!("{stem}-{n}.json")))
        .find(|path| !path.exists())
        .unwrap_or_else(|| dir.join(format!("{stem}.json")))
}

impl Trial {
    pub(crate) fn backup_dir(&self) -> PathBuf {
        self.device_root.join("backups")
    }

    /// Backup files, newest name first, with their sizes in bytes.
    pub(crate) fn backups(&self) -> Vec<(String, u64)> {
        let mut list: Vec<_> = fs::read_dir(self.backup_dir())
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|entry| {
                let name = entry.file_name().into_string().ok()?;
                let size = entry.metadata().ok()?.len();
                valid_name(&name).then_some((name, size))
            })
            .collect();
        list.sort_by(|a, b| b.0.cmp(&a.0));
        list
    }

    /// Writes a full backup of the live records, standalone GST bills and shared settings.
    pub(crate) fn backup_create(&mut self) -> Result<String, String> {
        let role = self.require_owner()?;
        let records = self
            .authority
            .archive_records(role, &self.directory)
            .map_err(debug)?;
        let ledger = self.authority.gst_bill_ledger(role).map_err(debug)?;
        let bytes = self
            .shop
            .backup_with(role, records, Some(ledger))
            .map_err(debug)?;
        let dir = self.backup_dir();
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let path = fresh(&dir, &format!("ALM-backup-{}", self.today()?));
        create_new(&path, &bytes).map_err(|e| e.to_string())?;
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_owned();
        self.message = format!("Backup saved ✓ {name}");
        Ok(name)
    }

    /// Validates a backup and opens it at a fresh path; an invalid archive changes nothing.
    pub(crate) fn backup_restore(&mut self, name: &str) -> Result<(), String> {
        let role = self.require_owner()?;
        if !valid_name(name) {
            return Err("Unknown backup.".into());
        }
        let bytes = fs::read(self.backup_dir().join(name)).map_err(|e| e.to_string())?;
        let dir = self.device_root.join("restored");
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let stem = name.trim_end_matches(".json");
        let path = fresh(&dir, &format!("{stem}-restored"));
        let store = Store::restore(&path, role, &bytes)
            .map_err(|_| "Not a valid ALM backup — nothing was changed.".to_owned())?;
        self.restored = Some((name.to_owned(), store));
        self.message = format!("Restored {name} to a fresh archive ✓");
        Ok(())
    }
}

#[tauri::command(async)]
pub fn backup_create(state: State<'_, AppState>) -> Result<String, String> {
    with(&state, "settings", |trial| {
        trial.backup_create().map(|_| ())
    })
}

#[tauri::command(async)]
pub fn backup_restore(state: State<'_, AppState>, q: String) -> Result<String, String> {
    with(&state, "settings", |trial| trial.backup_restore(&q))
}

pub(crate) fn summary(store: &Store) -> Result<crate::views::RestoredSummary, String> {
    let role = Role::Owner;
    let records = store.records(role).map_err(debug)?;
    let profile = store.business_profile(role).map_err(debug)?;
    let gst = store.standalone_gst_ledger(role).map_err(debug)?;
    Ok(crate::views::RestoredSummary {
        products: records.products.len(),
        customers: records.customers.len(),
        sales: records.sales.len(),
        receipts: records.receipts.len(),
        gst_bills: gst.bills.len(),
        reorder_default: records.reorder_default,
        profile_version: profile.version,
        gstin: profile.gstin.clone(),
    })
}
