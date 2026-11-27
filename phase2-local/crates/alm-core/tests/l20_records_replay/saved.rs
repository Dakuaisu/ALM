use std::fs;
use std::path::{Path, PathBuf};

mod scratch_retry {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../alm-store/tests/common/scratch_retry.rs"
    ));
}

use serde::{Serialize, de::DeserializeOwned};

pub(super) struct Saved(PathBuf);

impl Saved {
    pub fn new() -> Self {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .canonicalize()
            .unwrap();
        let target = root.join("target");
        fs::create_dir_all(&target).unwrap();
        assert_eq!(target.canonicalize().unwrap(), target);
        Self(scratch_retry::create(&target, "l20-records-").expect("create records scratch"))
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
