use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use serde::{Serialize, de::DeserializeOwned};

pub(super) struct Saved(PathBuf);

impl Saved {
    pub fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .canonicalize()
            .unwrap();
        let target = root.join("target");
        fs::create_dir_all(&target).unwrap();
        assert_eq!(target.canonicalize().unwrap(), target);
        loop {
            let seq = NEXT.fetch_add(1, Ordering::Relaxed);
            let path = target.join(format!("l20-records-{}-{seq}", std::process::id()));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(err) if err.kind() == ErrorKind::AlreadyExists => continue,
                Err(err) => panic!("create {}: {err}", path.display()),
            }
        }
    }

    pub fn write(&self, name: &str, data: &impl Serialize) {
        fs::write(self.0.join(name), serde_json::to_vec(data).unwrap()).unwrap();
    }

    pub fn bytes(&self, name: &str) -> Vec<u8> {
        fs::read(self.0.join(name)).unwrap()
    }

    pub fn read<T: DeserializeOwned>(&self, name: &str) -> T {
        serde_json::from_slice(&self.bytes(name)).unwrap()
    }
}

impl Drop for Saved {
    fn drop(&mut self) {
        let result = fs::remove_dir_all(&self.0);
        if !std::thread::panicking() {
            result.expect("remove disposable replay checkpoint");
        }
    }
}
