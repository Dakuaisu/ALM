use std::fs;
use std::path::{Path, PathBuf};

#[path = "scratch_retry.rs"]
mod scratch_retry;

pub(crate) fn target_dir() -> PathBuf {
    let target = Path::new(env!("CARGO_MANIFEST_DIR"))
        .canonicalize()
        .unwrap()
        .join("target");
    fs::create_dir_all(&target).expect("create crate target directory");
    assert_eq!(
        target.canonicalize().expect("existing Cargo target"),
        target
    );
    target
}

pub(crate) struct Scratch(PathBuf);

impl Scratch {
    pub(crate) fn new(prefix: &str) -> Self {
        let target = target_dir();
        Self(scratch_retry::create(&target, prefix).expect("create scratch directory"))
    }

    pub(crate) fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.0) {
            eprintln!("remove synthetic device directory: {error}");
        }
    }
}
