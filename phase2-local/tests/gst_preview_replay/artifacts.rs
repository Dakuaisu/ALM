use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::sink::Action;

pub struct Sandbox {
    path: PathBuf,
    retain: bool,
}

impl Sandbox {
    pub fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join(format!("gst-preview-replay-{}-{nonce}", std::process::id()));
        fs::create_dir(&path).unwrap();
        Self {
            path,
            retain: std::env::var("ALM_GST_PREVIEW_ARTIFACTS").as_deref() == Ok("1"),
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
