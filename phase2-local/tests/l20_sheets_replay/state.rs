use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct Stamp {
    #[serde(rename = "lastPushDevice")]
    pub device: String,
    #[serde(rename = "lastPushAt")]
    pub at: u64,
}

impl Stamp {
    pub fn new(device: &str, at: u64) -> Self {
        Self {
            device: device.into(),
            at,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Meta {
    pub rev: i64,
    pub schema: u32,
    pub sync_url: String,
    pub last_sync: u64,
    #[serde(flatten)]
    pub push: Stamp,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub(super) enum Sales {
    Rows(Vec<String>),
    Malformed(String),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) enum Activity {
    Force(String),
    Pull(String),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct Db {
    pub meta: Meta,
    pub products: Vec<String>,
    pub customers: Vec<String>,
    pub sales: Sales,
    pub log: Vec<Activity>,
}

impl Db {
    pub fn fixture(url: &str, product: &str, rev: i64) -> Self {
        Self {
            meta: Meta {
                rev,
                schema: 2,
                sync_url: url.into(),
                last_sync: 77,
                push: Stamp::default(),
            },
            products: vec![product.into()],
            customers: vec!["synthetic customer".into()],
            sales: Sales::Rows(vec!["synthetic sale".into()]),
            log: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum WriteMode {
    Now,
    Hold,
    Reject,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum WriteIntent {
    Seen(Stamp),
    MigrationBackup(Db),
    Save(Db),
}

pub(super) struct Disk {
    path: PathBuf,
    pub intents: Vec<WriteIntent>,
    pub seen_writable: bool,
    pub mirror_writable: bool,
    pub primary: WriteMode,
    held: Option<(Db, bool)>,
}

impl Disk {
    pub fn new(db: &Db) -> Self {
        static SEQ: AtomicUsize = AtomicUsize::new(0);
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join(format!(
                "l20-sheets-{}-{nonce}-{}",
                std::process::id(),
                SEQ.fetch_add(1, Ordering::Relaxed)
            ));
        fs::create_dir(&path).unwrap();
        let disk = Self {
            path,
            intents: Vec::new(),
            seen_writable: true,
            mirror_writable: true,
            primary: WriteMode::Now,
            held: None,
        };
        durable(&disk.path.join("mirror"), db);
        durable(&disk.path.join("primary"), db);
        disk
    }

    pub fn save(&mut self, db: &mut Db) -> bool {
        db.meta.rev += 1;
        self.intents.push(WriteIntent::Save(db.clone()));
        let mirror_saved = self.mirror_writable;
        if mirror_saved {
            durable(&self.path.join("mirror"), db);
        }
        match self.primary {
            WriteMode::Now => durable(&self.path.join("primary"), db),
            WriteMode::Hold => {
                assert!(self.held.replace((db.clone(), mirror_saved)).is_none());
            }
            WriteMode::Reject => return !mirror_saved,
        }
        false
    }

    pub fn complete_primary(&mut self, accepted: bool) -> bool {
        let (db, mirror_saved) = self.held.take().expect("no pending primary write");
        if accepted {
            durable(&self.path.join("primary"), &db);
        }
        !accepted && !mirror_saved
    }

    pub fn migration_backup(&mut self, db: &Db) {
        self.intents.push(WriteIntent::MigrationBackup(db.clone()));
        durable(&self.path.join("pre_v2_backup"), db);
    }

    pub fn set_seen(&mut self, stamp: &Stamp) {
        self.intents.push(WriteIntent::Seen(stamp.clone()));
        if self.seen_writable {
            durable(&self.path.join("seen"), stamp);
        }
    }

    pub fn seen(&self) -> Stamp {
        match fs::read(self.path.join("seen")) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Stamp::default(),
            Err(error) => panic!("seen read failed: {error}"),
        }
    }

    pub fn copies(&self) -> [Db; 2] {
        ["mirror", "primary"]
            .map(|slot| serde_json::from_slice(&fs::read(self.path.join(slot)).unwrap()).unwrap())
    }

    pub fn reload_selection(&self) -> Db {
        let [mirror, primary] = self.copies();
        if mirror.meta.rev >= primary.meta.rev {
            mirror
        } else {
            primary
        }
    }
}

impl Drop for Disk {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn durable(path: &Path, value: &impl Serialize) {
    let staging = path.with_extension("stage");
    let mut file = File::create(&staging).unwrap();
    file.write_all(&serde_json::to_vec(value).unwrap()).unwrap();
    file.sync_all().unwrap();
    fs::rename(staging, path).unwrap();
}
