use super::{
    Notice, Replay,
    script::{CloudTrigger, Request},
};

pub(super) const START: u64 = 1_000_000;

#[derive(Clone, Copy, Default)]
pub(super) struct Cloud {
    pub on: bool,
    pub url: &'static str,
    pub key: &'static str,
    pub token: &'static str,
}

impl Cloud {
    pub const fn enabled(self) -> bool {
        self.on && !self.url.is_empty() && !self.key.is_empty() && !self.token.is_empty()
    }
}

#[derive(Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
enum Timer {
    Sheets,
    Cloud,
    Confirm,
    Interval,
}

pub(super) struct Timers {
    pub now: u64,
    pub sheets: Option<u64>,
    pub cloud: Option<u64>,
    pub confirm: Option<u64>,
    interval: u64,
}

impl Default for Timers {
    fn default() -> Self {
        Self {
            now: START,
            sheets: None,
            cloud: None,
            confirm: None,
            interval: START + 15_000,
        }
    }
}

impl Replay {
    pub fn save_product(&mut self, name: &str) {
        self.db_mut().products.push(name.into());
        self.persist();
    }

    pub fn save_url(&mut self, url: &str) {
        self.db_mut().meta.sync_url = url.trim().into();
        self.persist();
    }

    fn persist(&mut self) {
        self.save_db();
        self.kick();
        self.timers.sheets = Some(self.timers.now + 2_500);
        self.update_status();
    }

    pub fn save_db(&mut self) {
        if self.disk.save(self.db.as_mut().unwrap()) {
            self.notices.push(Notice::SaveFailed);
        }
    }

    pub fn complete_primary(&mut self, accepted: bool) {
        if self.disk.complete_primary(accepted) {
            self.notices.push(Notice::SaveFailed);
        }
    }

    pub fn kick(&mut self) {
        if self.cloud.enabled() {
            self.timers.cloud = Some(self.timers.now + 3_000);
        }
    }

    pub fn visibility(&mut self, hidden: bool) {
        self.hidden = hidden;
        if !hidden {
            self.kick();
        }
    }

    pub fn cloud_sync(&mut self, trigger: CloudTrigger) {
        if self.cloud.enabled() && !self.cloud_busy && self.db.is_some() {
            self.cloud_busy = true;
            self.script.requests.push(Request::Cloud {
                trigger,
                at: self.timers.now,
            });
        }
    }

    pub fn advance(&mut self, to: u64) {
        assert!(to >= self.timers.now);
        while let Some((at, kind)) = [
            (self.timers.sheets, Timer::Sheets),
            (self.timers.cloud, Timer::Cloud),
            (self.timers.confirm, Timer::Confirm),
            (Some(self.timers.interval), Timer::Interval),
        ]
        .into_iter()
        .filter_map(|(at, kind)| at.map(|at| (at, kind)))
        .filter(|(at, _)| *at <= to)
        .min()
        {
            self.timers.now = at;
            match kind {
                Timer::Sheets => {
                    self.timers.sheets = None;
                    if !self.db().meta.sync_url.is_empty() {
                        self.push(true);
                    }
                }
                Timer::Cloud => {
                    self.timers.cloud = None;
                    self.cloud_sync(CloudTrigger::Kick);
                }
                Timer::Confirm => self.confirm_push(true),
                Timer::Interval => {
                    self.timers.interval += 15_000;
                    if !self.hidden && at - self.cloud_last > 55_000 {
                        self.cloud_sync(CloudTrigger::Interval);
                    }
                }
            }
        }
        self.timers.now = to;
    }
}
