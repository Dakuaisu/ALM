use super::script::{Read, Request};
use super::state::{Activity, Db, Sales, Stamp};
use super::{DEVICE, Notice, Replay, Status};

const TEN_MINUTES: u64 = 600_000;

impl Replay {
    pub fn update_status(&mut self) {
        self.status = if self.cloud.enabled() {
            self.conflict_click = false;
            self.status_pointer = false;
            if self.cloud_error {
                Status::CloudRetry
            } else if self.cloud_last > 0 {
                Status::CloudSynced(self.cloud_last)
            } else {
                Status::CloudReady
            }
        } else if self.db().meta.sync_url.is_empty() {
            Status::Local
        } else if self.blocked.is_some() {
            self.conflict_click = true;
            self.status_pointer = true;
            Status::Paused
        } else {
            self.conflict_click = false;
            self.status_pointer = false;
            let last = self.db().meta.last_sync;
            if last == 0 {
                Status::SheetsReady
            } else {
                Status::SheetsSynced(last)
            }
        };
    }

    pub fn click_status(&mut self) {
        if self.conflict_click {
            self.dialog = true;
        }
    }

    pub fn open_sheet(&mut self) -> bool {
        let url = &self.db().meta.sync_url;
        if url.is_empty() {
            return false;
        }
        self.script.requests.push(Request::Open {
            url: format!("{url}?action=open"),
            target: "_blank",
        });
        true
    }

    pub fn jsonp(&mut self, read: Read) -> usize {
        let url = &self.db.as_ref().unwrap().meta.sync_url;
        self.script.jsonp(url, self.timers.now, read)
    }

    pub fn push(&mut self, silent: bool) {
        if self.db().meta.sync_url.is_empty() {
            self.notice(silent, Notice::MissingUrl);
        } else if self.blocked.is_some() {
            if silent {
                self.update_status();
            } else {
                self.dialog = true;
            }
        } else if self.timers.now - self.last_remote <= TEN_MINUTES {
            self.do_push(silent);
        } else {
            self.jsonp(Read::Precheck(silent));
        }
    }

    fn do_push(&mut self, silent: bool) {
        let at = self.timers.now;
        self.db_mut().meta.push = Stamp::new(DEVICE, at);
        self.script.post(self.db().clone(), silent);
    }

    pub fn settle_post(&mut self, id: usize, opaque_resolved: bool) {
        let silent = self.script.take_post(id);
        if opaque_resolved {
            self.confirm_push(silent);
        } else {
            self.notice(silent, Notice::PostFailed);
        }
    }

    pub fn confirm_push(&mut self, silent: bool) {
        self.timers.confirm = None;
        let due = self.checked_at + TEN_MINUTES;
        if silent && self.timers.now < due {
            self.timers.confirm = Some(due);
            return;
        }
        self.checked_at = self.timers.now;
        self.jsonp(Read::Confirm {
            silent,
            at: self.db().meta.push.at,
        });
    }

    pub fn reply(&mut self, id: usize, result: Result<Option<Db>, ()>) {
        let read = self.script.begin_reply(id, result.is_ok());
        let loaded = result.is_ok();
        match read {
            Read::Precheck(silent) => self.precheck(silent, result),
            Read::Confirm { silent, at } => self.confirmed(silent, at, result),
            Read::Pull => self.import(result),
            Read::Probe => {
                if let Ok(payload) = result {
                    self.script.probe_payloads.push(payload);
                }
            }
        }
        if loaded {
            self.script.cleanup(id);
        }
    }

    fn precheck(&mut self, silent: bool, result: Result<Option<Db>, ()>) {
        if let Ok(db) = result {
            let remote = db.map(|db| db.meta.push).unwrap_or_default();
            if !remote.device.is_empty()
                && remote.device != DEVICE
                && remote.at > self.disk.seen().at
            {
                self.blocked = Some(remote);
                self.update_status();
                self.notices.push(Notice::Conflict);
                if !silent {
                    self.dialog = true;
                }
                return;
            }
            self.last_remote = self.timers.now;
        }
        self.do_push(silent);
    }

    fn confirmed(&mut self, silent: bool, at: u64, result: Result<Option<Db>, ()>) {
        match result {
            Ok(Some(db)) if db.meta.push == Stamp::new(DEVICE, at) => {
                self.db_mut().meta.last_sync = self.timers.now;
                self.disk.set_seen(&db.meta.push);
                self.last_remote = self.timers.now;
                self.save_db();
                self.update_status();
                self.notice(silent, Notice::Confirmed);
            }
            Ok(_) => self.notice(silent, Notice::Mismatch),
            Err(()) => self.notice(silent, Notice::ReadbackFailed),
        }
    }

    pub fn later(&mut self) {
        self.dialog = false;
    }

    pub fn force(&mut self, confirmed: bool) {
        assert!(self.dialog);
        if confirmed {
            let device = self
                .blocked
                .take()
                .map_or_else(|| "?".into(), |stamp| stamp.device);
            self.db_mut().log.push(Activity::Force(device));
            self.do_push(false);
            self.dialog = false;
        }
    }

    pub fn pull_latest(&mut self) {
        self.dialog = false;
        self.pull(true);
    }

    pub fn pull(&mut self, confirmed: bool) {
        if self.db().meta.sync_url.is_empty() {
            self.notices.push(Notice::MissingUrl);
        } else if confirmed {
            self.notices.push(Notice::Pulling);
            self.jsonp(Read::Pull);
        } else {
            self.notices.push(Notice::ConfirmReplace);
        }
    }

    fn import(&mut self, result: Result<Option<Db>, ()>) {
        let db = match result {
            Ok(Some(db)) => db,
            Ok(None) => {
                self.notices.push(Notice::NoData);
                return;
            }
            Err(()) => {
                self.notices.push(Notice::PullLoadFailed);
                return;
            }
        };
        self.db = Some(db);
        self.db_mut().meta.rev = self.db().meta.rev.max(0);
        if self.db().meta.schema < 2 {
            self.disk.migration_backup(self.db.as_ref().unwrap());
            if matches!(self.db().sales, Sales::Malformed(_)) {
                self.notices.push(Notice::PullError);
                return;
            }
            self.db_mut().meta.schema = 2;
        }
        let stamp = self.db().meta.push.clone();
        self.disk.set_seen(&stamp);
        self.blocked = None;
        self.last_remote = self.timers.now;
        self.db_mut().log.push(Activity::Pull(stamp.device));
        self.save_db();
        self.notices.push(Notice::Pulled);
        self.redraws.push(self.current);
        self.update_status();
    }

    fn notice(&mut self, silent: bool, notice: Notice) {
        if !silent {
            self.notices.push(notice);
        }
    }
}
