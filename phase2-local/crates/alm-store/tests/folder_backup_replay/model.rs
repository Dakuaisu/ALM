use std::fs::{self, File};
use std::io::{self, Write};
use std::path::PathBuf;
mod scratch_retry {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../alm-store/tests/common/scratch_retry.rs"
    ));
}

use serde::{Deserialize, Serialize};

pub struct Sandbox(pub PathBuf);

impl Sandbox {
    pub fn new() -> Self {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
        let path = scratch_retry::create(&base, "folder-backup-").unwrap();
        for name in ["old", "new"] {
            fs::create_dir(path.join(name)).unwrap();
        }
        Self(path)
    }
    pub fn file(&self, folder: &str, name: &str) -> PathBuf {
        self.0.join(folder).join(name)
    }
    pub fn read(&self, folder: &str, name: &str) -> Database {
        serde_json::from_slice(&fs::read(self.file(folder, name)).unwrap()).unwrap()
    }
}
impl Drop for Sandbox {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Database {
    pub records: Vec<String>,
    pub meta: Meta,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Meta {
    pub backup_folder: Option<String>,
    pub last_file_backup: u64,
}
impl Database {
    pub fn new(record: &str) -> Self {
        Self {
            records: vec![record.into()],
            meta: Meta {
                backup_folder: None,
                last_file_backup: 7,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Permission {
    Granted,
    Prompt,
    Denied,
}
#[derive(Clone, Copy)]
pub enum Picker {
    Select(&'static str),
    Cancel,
    Fail,
}
#[derive(Default)]
pub struct TokenStore {
    pub token: Option<&'static str>,
    pub accepts: bool,
}

#[derive(Default)]
pub struct CurrentWrite {
    pub reject: bool,
    pub attempts: Vec<Vec<u8>>,
}

impl CurrentWrite {
    fn write(&mut self, staged: &mut File, bytes: &[u8]) -> io::Result<()> {
        self.attempts.push(bytes.to_vec());
        if self.reject {
            return Err(io::Error::other("synthetic current write rejection"));
        }
        staged.write_all(bytes)
    }
}

pub struct Replay<'a> {
    pub sandbox: &'a Sandbox,
    pub db: Database,
    pub selected: Option<&'static str>,
    pub needs_grant: bool,
    pub permission: Permission,
    pub requested: usize,
    pub request_result: Permission,
    pub now: u64,
    pub day: &'static str,
    pub daily: Option<&'static str>,
    pub deadline: Option<u64>,
    pub current_write: CurrentWrite,
    pub fail_dated: bool,
    pub feedback: Vec<&'static str>,
    pub trace: Vec<&'static str>,
}
impl<'a> Replay<'a> {
    pub fn new(sandbox: &'a Sandbox, record: &str) -> Self {
        Self {
            sandbox,
            db: Database::new(record),
            selected: None,
            needs_grant: false,
            permission: Permission::Granted,
            requested: 0,
            request_result: Permission::Granted,
            now: 100,
            day: "2024-05-01",
            daily: None,
            deadline: None,
            current_write: CurrentWrite::default(),
            fail_dated: false,
            feedback: vec![],
            trace: vec![],
        }
    }
    pub fn restore(&mut self, store: &TokenStore) {
        self.selected = store.token;
        if let Some(name) = self.selected {
            self.db.meta.backup_folder = Some(name.into());
            self.trace.push("restore-query");
            self.needs_grant = self.permission != Permission::Granted;
        }
    }
    pub fn connect(&mut self, picker: Picker, store: &mut TokenStore) {
        match picker {
            Picker::Cancel => {}
            Picker::Fail => self.feedback.push("Could not connect folder"),
            Picker::Select(name) => {
                self.selected = Some(name);
                self.needs_grant = false;
                if store.accepts {
                    store.token = Some(name);
                }
                self.db.meta.backup_folder = Some(name.into());
                self.write(true);
                self.feedback.push("Backup folder set");
            }
        }
    }
    pub fn schedule(&mut self) {
        if self.selected.is_some() {
            self.deadline = Some(self.now + 1500);
        }
    }
    pub fn advance(&mut self, now: u64) {
        self.now = now;
        if self.deadline.is_some_and(|due| due <= now) {
            self.deadline = None;
            self.trace.push("callback");
            self.write(false);
        }
    }
    pub fn manual(&mut self, reconnect: bool) {
        let ok = self.write(true);
        self.feedback.push(match (reconnect, ok) {
            (true, true) => "Reconnected & backed up",
            (true, false) => "Permission denied",
            (false, true) => "Saved to folder",
            (false, false) => "Could not write",
        });
    }
    pub fn write(&mut self, force: bool) -> bool {
        let Some(folder) = self.selected else {
            return false;
        };
        self.trace.push("query");
        if self.permission != Permission::Granted {
            if force {
                self.requested += 1;
                self.trace.push("request");
                self.permission = self.request_result;
            }
            if self.permission != Permission::Granted {
                self.needs_grant = true;
                return false;
            }
            self.needs_grant = false;
        }
        let bytes = serde_json::to_vec(&self.db).unwrap();
        if self.write_files(folder, &bytes).is_err() {
            return false;
        }
        self.db.meta.last_file_backup = self.now;
        true
    }
    fn write_files(&mut self, folder: &str, bytes: &[u8]) -> io::Result<()> {
        let destination = self.sandbox.file(folder, "ALM_data.json");
        let staged = destination.with_extension("json.stage");
        let mut current = File::create(&staged)?;
        let result = self.current_write.write(&mut current, bytes);
        drop(current);
        if let Err(error) = result {
            self.trace.push("current-write-failed");
            fs::remove_file(staged)?;
            return Err(error);
        }
        fs::rename(staged, destination)?;
        self.trace.push("current-closed");
        if self.daily != Some(self.day) {
            let destination = self
                .sandbox
                .file(folder, &format!("ALM_backup_{}.json", self.day));
            let staged = destination.with_extension("json.stage");
            let mut daily = File::create(&staged)?;
            daily.write_all(bytes)?;
            drop(daily);
            self.trace.push("dated-staged");
            if self.fail_dated {
                fs::remove_file(&staged)?;
                self.trace.push("dated-failed");
                return Err(io::Error::other("synthetic dated failure"));
            }
            fs::rename(staged, destination)?;
            self.trace.push("dated-closed");
            self.daily = Some(self.day);
        }
        Ok(())
    }
}
