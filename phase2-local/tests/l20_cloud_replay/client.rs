use super::records::{Db, Device, Key, Row, Stamp, Table};
use super::rpc::{Op, Rejection, Reply, Request, Script};
use super::storage::{Completed, Files, Progress};

pub const NOW: u64 = 90_000_000;
const MIRROR_INTERVAL: u64 = 900_000;
const SNAP_INTERVAL: u64 = 86_400_000;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Screen {
    #[default]
    Products,
    Bill,
    GstSales,
    Dashboard,
}

#[derive(Clone, Copy, Default)]
pub enum Focus {
    #[default]
    Other,
    Input,
    Select,
    Textarea,
}

#[derive(Clone, Copy, Default)]
pub struct View {
    pub screen: Screen,
    pub modal: bool,
    pub draft: bool,
    pub gst_draft: bool,
    pub focus: Focus,
}

impl View {
    fn busy(self) -> bool {
        self.modal
            || (self.screen == Screen::Bill && self.draft)
            || (self.screen == Screen::GstSales && self.gst_draft)
            || matches!(self.focus, Focus::Input | Focus::Select | Focus::Textarea)
    }
}

pub enum Live {
    Ready(Db),
    Assigned(serde_json::Value),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Notice {
    Synced,
    Stopped(Rejection),
    Retrying(Rejection),
    EnableFirst,
    NoSheetsUrl,
    Uploading,
    Uploaded,
    UploadIssue(Rejection),
    NoSnapshot,
    RebuildRpcFailed(Rejection),
    MalformedSnapshot,
    MigrationFailed,
    Rebuilt,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Event {
    Rpc(Op),
    Progress(Progress, bool),
    Save { rev: u64, mirror: bool },
    SafetyStarted(u64),
    Complete(Completed),
    Redraw(Screen),
    CloudKick,
    Audit(i64),
    Notice(Notice),
}

pub struct Client {
    pub live: Live,
    pub painted: Db,
    pub files: Files,
    pub rpc: Script,
    pub trace: Vec<Event>,
    pub view: View,
    pub present: [bool; 3],
    pub busy: bool,
    pub now: u64,
    pub err: Option<Rejection>,
    pub failures: u32,
}

impl Client {
    pub fn new(db: Db, progress: Progress, enabled: bool) -> Self {
        let files = Files::new(&db, progress, enabled);
        Self {
            painted: db.clone(),
            live: Live::Ready(db),
            files,
            rpc: Script::default(),
            trace: Vec::new(),
            view: View::default(),
            present: [true; 3],
            busy: false,
            now: NOW,
            err: None,
            failures: 0,
        }
    }

    pub fn db(&self) -> &Db {
        let Live::Ready(db) = &self.live else {
            panic!("database has not migrated")
        };
        db
    }

    pub fn db_mut(&mut self) -> &mut Db {
        let Live::Ready(db) = &mut self.live else {
            panic!("database has not migrated")
        };
        db
    }

    pub fn enabled(&self) -> bool {
        self.files.cloud_on() && self.present.into_iter().all(|field| field)
    }

    pub fn call(&mut self, request: Request) -> Reply {
        self.trace.push(Event::Rpc(request.op()));
        self.rpc.call(request)
    }

    pub fn note(&mut self, notice: Notice) {
        self.trace.push(Event::Notice(notice));
    }

    pub fn checkpoint(&mut self, progress: Progress) {
        self.trace
            .push(Event::Progress(progress, self.files.checkpoint(progress)));
    }

    pub fn save(&mut self) {
        self.db_mut().rev += 1;
        let db = self.db().clone();
        let mirror = self.files.begin_save(&db);
        self.trace.push(Event::Save {
            rev: db.rev,
            mirror,
        });
    }

    pub fn persist(&mut self) {
        self.save();
        if self.enabled() {
            self.trace.push(Event::CloudKick);
        }
    }

    pub fn sheets_without_url(&mut self) {
        self.note(Notice::NoSheetsUrl);
    }

    pub fn settle(&mut self) {
        self.trace
            .extend(self.files.settle().into_iter().map(Event::Complete));
    }

    pub fn reload(&mut self) {
        let (primary, mirror) = self.files.copies();
        if mirror.rev >= primary.rev {
            self.live = Live::Ready(mirror);
        } else {
            self.live = Live::Ready(primary);
            self.save();
        }
        self.painted = self.db().clone();
    }

    pub fn audit(&mut self, value: i64) {
        let stamp = Stamp {
            version: 1,
            time: self.now,
            device: Device::Local,
        };
        let id = 1 + self
            .db()
            .rows
            .iter()
            .filter(|row| row.key.0 == Table::Log)
            .map(|row| row.key.1)
            .max()
            .unwrap_or(0);
        self.db_mut()
            .rows
            .push(Row::new(Key(Table::Log, id), value, Some(stamp)));
        self.trace.push(Event::Audit(value));
    }

    pub fn redraw(&mut self, screen: Screen) {
        self.painted = self.db().clone();
        self.trace.push(Event::Redraw(screen));
    }

    pub fn sync(&mut self, silent: bool) {
        if !self.enabled() || self.busy || !matches!(self.live, Live::Ready(_)) {
            return;
        }
        self.busy = true;
        let result = self.sync_records();
        match result {
            Ok(()) => {
                self.err = None;
                self.failures = 0;
                self.refresh_optional();
                if !silent {
                    self.note(Notice::Synced);
                }
            }
            Err(error) => {
                self.err = Some(error);
                self.failures += 1;
                match error {
                    Rejection::Revoked | Rejection::Http401 | Rejection::Http403 => {
                        self.files.set_cloud(false);
                        self.note(Notice::Stopped(error));
                        self.audit(-1);
                    }
                    Rejection::Ordinary if !silent || self.failures == 3 => {
                        self.note(Notice::Retrying(error))
                    }
                    Rejection::Ordinary => {}
                }
            }
        }
        self.busy = false;
    }
}

#[path = "sync.rs"]
mod sync;
