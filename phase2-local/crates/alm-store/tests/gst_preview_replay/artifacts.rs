use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::sink::Action;

pub struct Sandbox {
    path: PathBuf,
    retain: bool,
}

impl Sandbox {
    pub fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let target = Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
        let path = create_sandbox(&target, process::id(), || {
            NEXT.fetch_add(1, Ordering::Relaxed)
        });
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

fn create_sandbox(target: &Path, process_id: u32, mut next: impl FnMut() -> usize) -> PathBuf {
    loop {
        let sequence = next();
        let path = target.join(format!("gst-preview-replay-{process_id}-{sequence}"));
        match fs::create_dir(&path) {
            Ok(()) => return path,
            Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
            Err(error) => panic!("create GST preview sandbox {}: {error}", path.display()),
        }
    }
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
