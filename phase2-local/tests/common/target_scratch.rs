use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicUsize, Ordering};

pub(crate) fn target_dir() -> PathBuf {
    let target = Path::new(env!("CARGO_MANIFEST_DIR"))
        .canonicalize()
        .unwrap()
        .join("target");
    assert_eq!(
        target.canonicalize().expect("existing Cargo target"),
        target
    );
    target
}

pub(crate) struct Scratch(PathBuf);

impl Scratch {
    pub(crate) fn new(prefix: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let target = target_dir();
        loop {
            let seq = NEXT.fetch_add(1, Ordering::Relaxed);
            let path = target.join(format!("{prefix}{}-{seq}", process::id()));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(err) if err.kind() == ErrorKind::AlreadyExists => continue,
                Err(err) => panic!("create scratch directory {}: {err}", path.display()),
            }
        }
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
