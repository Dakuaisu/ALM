use std::fs;
use std::io;
use std::path::PathBuf;

mod scratch_retry {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../alm-store/tests/common/scratch_retry.rs"
    ));
}

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct State {
    pub(super) seq: u64,
    pub(super) pushed_m: u64,
    pub(super) mirror_at: u64,
    pub(super) snap_at: u64,
}

pub(super) struct Store(PathBuf);

impl Store {
    pub(super) fn seed(state: State) -> Self {
        let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let temp = std::env::temp_dir().canonicalize().unwrap();
        let base = if temp.starts_with(&project) {
            temp
        } else {
            project.join("target")
        };
        fs::create_dir_all(&base).unwrap();
        let dir =
            scratch_retry::create(&base, "l20-mirror-").expect("create replay state directory");
        let store = Self(dir);
        store.write(state).unwrap();
        store
    }

    pub(super) fn read(&self) -> State {
        let bytes = fs::read(self.0.join("alm_sbstate_v1.json")).unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    pub(super) fn write(&self, state: State) -> io::Result<()> {
        fs::write(
            self.0.join("alm_sbstate_v1.json"),
            serde_json::to_vec(&state).unwrap(),
        )
    }
}

impl Drop for Store {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.0) {
            eprintln!("could not remove mirror replay state: {error}");
        }
    }
}
