use std::fs;
use std::path::{Path, PathBuf};

mod scratch_retry {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../alm-store/tests/common/scratch_retry.rs"
    ));
}

use super::sink::Action;

pub struct Sandbox {
    path: PathBuf,
    retain: bool,
}

impl Sandbox {
    pub fn new() -> Self {
        let target = Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
        let path = scratch_retry::create(&target, "gst-preview-replay-")
            .expect("create GST preview sandbox");
        Self {
            path,
            retain: retain_artifacts(std::env::var("ALM_GST_PREVIEW_ARTIFACTS").ok().as_deref()),
        }
    }

    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    pub fn readback(&self, count: usize, action: Action, html: &str) -> String {
        let name = match action {
            Action::Print => "print",
            Action::SavePdf => "save-pdf",
        };
        let path = self.path.join(format!("invoice-{count}-{name}.html"));
        fs::write(&path, html).unwrap();
        fs::read_to_string(path).unwrap()
    }
}

#[cfg(test)]
fn create_sandbox(target: &Path, process_id: u32, mut next: impl FnMut() -> usize) -> PathBuf {
    scratch_retry::create_with(
        |_| target.join(format!("gst-preview-replay-{process_id}-{}", next())),
        |path| fs::create_dir(path),
        cfg!(windows),
    )
    .expect("create GST preview sandbox")
}

fn retain_artifacts(value: Option<&str>) -> bool {
    value == Some("1")
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        if self.retain {
            println!(
                "Synthetic inert GST preview artifacts: {}",
                self.path.display()
            );
        } else {
            fs::remove_dir_all(&self.path).unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Sandbox, create_sandbox};
    use std::fs;

    #[test]
    fn sandbox_retries_collision_and_cleans_only_its_created_path() {
        let mut parent = Sandbox::new();
        parent.retain = false;
        let test_parent = parent.path();
        let collision = test_parent.join("gst-preview-replay-17-0");
        fs::create_dir(&collision).unwrap();
        let sentinel = collision.join("sentinel");
        fs::write(&sentinel, "preserve").unwrap();

        let mut sequence = 0;
        let created = create_sandbox(test_parent, 17, || {
            let candidate = sequence;
            sequence += 1;
            candidate
        });
        assert_eq!(sequence, 2);
        assert_eq!(created, test_parent.join("gst-preview-replay-17-1"));
        assert_ne!(created, collision);

        drop(Sandbox {
            path: created.clone(),
            retain: false,
        });
        assert!(!created.exists());
        assert_eq!(fs::read_to_string(&sentinel).unwrap(), "preserve");
    }

    #[test]
    fn sandbox_candidates_are_distinct_for_a_shared_atomic_sequence() {
        let mut first = Sandbox::new();
        let mut second = Sandbox::new();
        first.retain = false;
        second.retain = false;
        assert_ne!(first.path(), second.path());
        assert!(first.path().is_dir() && second.path().is_dir());
    }

    #[test]
    fn retention_keeps_only_the_existing_exact_one_setting() {
        assert!(super::retain_artifacts(Some("1")));
        assert!(!super::retain_artifacts(Some("true")));
        assert!(!super::retain_artifacts(Some("0")));
        assert!(!super::retain_artifacts(None));
    }
}
