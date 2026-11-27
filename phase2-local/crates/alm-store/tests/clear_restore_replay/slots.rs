use std::fs;
use std::path::{Path, PathBuf};
mod scratch_retry {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../alm-store/tests/common/scratch_retry.rs"
    ));
}

use serde::{Deserialize, Serialize};

pub(super) struct Sandbox(pub(super) PathBuf);

impl Sandbox {
    pub(super) fn new() -> Self {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
        fs::create_dir_all(&root).unwrap();
        let path = scratch_retry::create(&root, "clear-restore-").unwrap();
        Self(path)
    }
    pub(super) fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct Snapshot {
    pub revision: u64,
    pub archive: Vec<u8>,
    pub activity: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Outcome {
    Accepted,
    Rejected,
}

pub(super) fn write_slot(path: &Path, snapshot: &Snapshot) {
    fs::write(path, serde_json::to_vec(snapshot).unwrap()).unwrap();
}
pub(super) fn read_slot(path: &Path) -> Snapshot {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
pub(super) fn attempt(path: &Path, snapshot: &Snapshot, outcome: Outcome) -> Outcome {
    match outcome {
        Outcome::Accepted => write_slot(path, snapshot),
        Outcome::Rejected => (),
    }
    outcome
}

pub(super) fn select(dir: &Sandbox) -> (&'static str, Snapshot) {
    let mirror = read_slot(&dir.path("mirror"));
    let primary = read_slot(&dir.path("primary"));
    if mirror.revision >= primary.revision {
        ("mirror", mirror)
    } else {
        ("primary", primary)
    }
}

pub(super) fn follow_up(dir: &Sandbox, selected: &Snapshot) -> Snapshot {
    let mut saved = selected.clone();
    saved.revision += 1;
    write_slot(&dir.path("mirror"), &saved);
    write_slot(&dir.path("primary"), &saved);
    saved
}
