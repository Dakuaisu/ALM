use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

#[path = "milestone_d_replay/exports.rs"]
mod exports;
#[path = "milestone_d_replay/fixture.rs"]
mod fixture;
#[path = "milestone_d_replay/outputs.rs"]
mod outputs;
#[path = "milestone_d_replay/recovery.rs"]
mod recovery;
#[path = "milestone_d_replay/sales_output.rs"]
mod sales_output;

static NEXT_REPLAY: AtomicU64 = AtomicU64::new(0);

struct ReplayDirectory {
    path: PathBuf,
    retain: bool,
}

impl ReplayDirectory {
    fn create() -> Self {
        let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let target = crate_root.join("target");
        assert!(target.starts_with(crate_root));
        fs::create_dir_all(&target).expect("create canonical crate target directory");
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is after the Unix epoch")
            .as_nanos();
        let sequence = NEXT_REPLAY.fetch_add(1, Ordering::Relaxed);
        let path = target.join(format!("milestone-d-replay-{nonce}-{sequence}"));
        fs::create_dir(&path).expect("create unique replay directory");
        Self {
            path,
            retain: std::env::var("ALM_REPLAY_QUICKLOOK_PNG").as_deref() == Ok("1"),
        }
    }
}

impl Drop for ReplayDirectory {
    fn drop(&mut self) {
        if !self.retain {
            fs::remove_dir_all(&self.path).expect("remove replay artifacts");
        }
    }
}

#[test]
fn milestone_d_replays_outputs_exports_and_read_model_recovery() {
    // Given: one synthetic fixture and an isolated directory below the crate target.
    let fixture = fixture::cancelled_fixture();
    let directory = ReplayDirectory::create();

    // When: the output, export, and scoped read-model recovery exercises run once.
    outputs::exercise(&fixture, &directory.path);
    exports::exercise(&fixture, &directory.path);
    recovery::exercise(&fixture, &directory.path);

    // Then: recovery leaves its archive and restored database as read-only projections.
    let restored = directory.path.join("d-read-model-restored.sqlite");
    assert!(restored.is_file());
    println!("Milestone D replay report: {}", directory.path.display());
}
