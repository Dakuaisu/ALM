use std::collections::VecDeque;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use serde::{Deserialize, Serialize, de::DeserializeOwned};

use super::records::Db;

static NEXT: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Progress {
    pub pushed: u64,
    pub seq: u64,
    pub mirror_at: u64,
    pub snap_at: u64,
}

#[derive(Clone, Copy)]
pub struct Writes {
    pub primary: bool,
    pub mirror: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Completed {
    Primary { rev: u64, written: bool },
    Safety { rev: u64, written: bool },
}

pub struct Files {
    dir: PathBuf,
    pending: VecDeque<(Completed, Db)>,
    pub save_policy: VecDeque<Writes>,
    pub progress_policy: VecDeque<bool>,
    pub safety_writes: bool,
}

impl Files {
    pub fn new(db: &Db, progress: Progress, on: bool) -> Self {
        let name = format!(
            "l20-cloud-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join(name);
        fs::create_dir_all(dir.parent().unwrap()).unwrap();
        fs::create_dir(&dir).unwrap();
        let files = Self {
            dir,
            pending: VecDeque::new(),
            save_policy: VecDeque::new(),
            progress_policy: VecDeque::new(),
            safety_writes: true,
        };
        files.write("primary.json", db);
        files.write("mirror.json", db);
        files.write("progress.json", &progress);
        files.set_cloud(on);
        files
    }

    fn write(&self, name: &str, data: &impl Serialize) {
        let mut file = fs::File::create(self.dir.join(name)).unwrap();
        file.write_all(&serde_json::to_vec(data).unwrap()).unwrap();
        file.sync_all().unwrap();
    }

    fn read<T: DeserializeOwned>(&self, name: &str) -> T {
        serde_json::from_slice(&fs::read(self.dir.join(name)).unwrap()).unwrap()
    }

    pub fn progress(&self) -> Progress {
        self.read("progress.json")
    }

    pub fn checkpoint(&mut self, progress: Progress) -> bool {
        let writes = self.progress_policy.pop_front().unwrap_or(true);
        if writes {
            self.write("progress.json", &progress);
        }
        writes
    }

    pub fn set_cloud(&self, on: bool) {
        self.write("cloud-on.json", &on);
    }

    pub fn cloud_on(&self) -> bool {
        self.read("cloud-on.json")
    }

    pub fn copies(&self) -> (Db, Db) {
        (self.read("primary.json"), self.read("mirror.json"))
    }

    pub fn begin_save(&mut self, db: &Db) -> bool {
        let policy = self.save_policy.pop_front().unwrap_or(Writes {
            primary: true,
            mirror: true,
        });
        let mirror =
            policy.mirror && serde_json::to_string(db).unwrap().encode_utf16().count() < 4_500_000;
        if mirror {
            self.write("mirror.json", db);
        }
        self.pending.push_back((
            Completed::Primary {
                rev: db.rev,
                written: policy.primary,
            },
            db.clone(),
        ));
        mirror
    }

    pub fn begin_safety(&mut self, db: &Db) {
        self.pending.push_back((
            Completed::Safety {
                rev: db.rev,
                written: self.safety_writes,
            },
            db.clone(),
        ));
    }

    pub fn safety(&self) -> Option<Db> {
        self.dir
            .join("pre-rebuild.json")
            .is_file()
            .then(|| self.read("pre-rebuild.json"))
    }

    pub fn settle(&mut self) -> Vec<Completed> {
        let mut completed = Vec::new();
        while let Some((result, db)) = self.pending.pop_front() {
            match result {
                Completed::Primary { written: true, .. } => self.write("primary.json", &db),
                Completed::Safety { written: true, .. } => self.write("pre-rebuild.json", &db),
                Completed::Primary { written: false, .. }
                | Completed::Safety { written: false, .. } => {}
            }
            completed.push(result);
        }
        completed
    }
}

impl Drop for Files {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.dir) {
            eprintln!("cloud replay scratch cleanup: {error}");
        }
    }
}
