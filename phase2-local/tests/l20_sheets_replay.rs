//! Source-derived L20 replay; no transport, HTML execution, or browser is reachable.
//! Oracle: `docs/PHASE1_CASES.md` L20-01/05/06/09/10/14/18 and their extensions.
//! Frozen HTML anchors: 684–794 (save/reload), 2218–2243 (sidebar), 9195–9226
//! (settings), 9447–9456 and 9568–9576 (cloud guard), 9806–9997 (timers and Sheets).
#![forbid(unsafe_code)]

#[path = "l20_sheets_replay/browser_cases.rs"]
mod browser_cases;
#[path = "l20_sheets_replay/conflict_cases.rs"]
mod conflict_cases;
#[path = "l20_sheets_replay/push_cases.rs"]
mod push_cases;
#[path = "l20_sheets_replay/schedule.rs"]
mod schedule;
#[path = "l20_sheets_replay/scheduled_cases.rs"]
mod scheduled_cases;
#[path = "l20_sheets_replay/script.rs"]
mod script;
#[path = "l20_sheets_replay/sheets.rs"]
mod sheets;
#[path = "l20_sheets_replay/state.rs"]
mod state;

use schedule::{Cloud, Timers};
use script::Script;
use state::{Db, Disk, Stamp};

const DEVICE: &str = "synthetic-local";
const OTHER: &str = "synthetic-other";
const URL: &str = "https://sheets.invalid/exec";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Notice {
    MissingUrl,
    Conflict,
    PostFailed,
    Confirmed,
    Mismatch,
    ReadbackFailed,
    ConfirmReplace,
    Pulling,
    NoData,
    PullLoadFailed,
    PullError,
    Pulled,
    SaveFailed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Status {
    Local,
    Paused,
    CloudReady,
    CloudSynced(u64),
    CloudRetry,
    SheetsReady,
    SheetsSynced(u64),
}

impl Status {
    const fn ok_dot(self) -> bool {
        matches!(
            self,
            Self::CloudReady | Self::CloudSynced(_) | Self::SheetsReady | Self::SheetsSynced(_)
        )
    }
}

struct Replay {
    timers: Timers,
    disk: Disk,
    db: Option<Db>,
    script: Script,
    cloud: Cloud,
    cloud_busy: bool,
    cloud_last: u64,
    cloud_error: bool,
    hidden: bool,
    blocked: Option<Stamp>,
    last_remote: u64,
    checked_at: u64,
    status: Status,
    conflict_click: bool,
    status_pointer: bool,
    dialog: bool,
    notices: Vec<Notice>,
    current: &'static str,
    redraws: Vec<&'static str>,
    pushed_m: u64,
}

impl Replay {
    fn new(url: &str) -> Self {
        let db = Db::fixture(url, "original product", 40);
        let mut app = Self {
            timers: Timers::default(),
            disk: Disk::new(&db),
            db: Some(db),
            script: Script::default(),
            cloud: Cloud::default(),
            cloud_busy: false,
            cloud_last: 0,
            cloud_error: false,
            hidden: false,
            blocked: None,
            last_remote: 0,
            checked_at: 0,
            status: Status::Local,
            conflict_click: false,
            status_pointer: false,
            dialog: false,
            notices: Vec::new(),
            current: "stock",
            redraws: Vec::new(),
            pushed_m: 812,
        };
        app.update_status();
        app
    }

    fn db(&self) -> &Db {
        self.db.as_ref().unwrap()
    }
    fn db_mut(&mut self) -> &mut Db {
        self.db.as_mut().unwrap()
    }

    fn assert_copies(&self, expected: &Db) {
        let [mirror, primary] = self.disk.copies();
        assert_eq!(&mirror, expected, "mirror readback");
        assert_eq!(&primary, expected, "primary readback");
    }
}

fn remote(device: &str, at: u64) -> Db {
    let mut db = Db::fixture(URL, "replacement product", 3);
    db.meta.push = Stamp::new(device, at);
    db
}
