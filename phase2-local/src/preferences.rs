//! Local presentation preferences stored as one file per key under a directory.
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use alm_core::Money;
use alm_core::confirmation::BalanceMode;
use alm_core::preferences::PreferenceState;
pub use alm_core::preferences::{Preference, Theme};

/// A session's local preferences; storage failure never changes authorization.
pub struct Preferences {
    directory: PathBuf,
    state: PreferenceState,
}

impl Preferences {
    /// Missing, unreadable, or unrecognized keys retain their defaults.
    pub fn load(directory: &Path) -> Self {
        Self {
            directory: directory.into(),
            state: PreferenceState::load(|key| fs::read(directory.join(key)).ok()),
        }
    }

    pub const fn theme(&self) -> Theme {
        self.state.theme()
    }
    pub const fn balance_mode(&self) -> BalanceMode {
        self.state.balance_mode()
    }

    /// Returns only a presentation projection, not an access-control decision.
    pub const fn present_money(&self, amount: Money) -> Option<Money> {
        self.state.present_money(amount)
    }

    /// Applies immediately in this session even if saving the key fails.
    pub fn set(&mut self, preference: Preference) {
        let directory = &self.directory;
        self.state.set(preference, |key, value| {
            let stage = directory.join(format!("{key}.stage"));
            if let Ok(mut file) = OpenOptions::new().write(true).create_new(true).open(&stage) {
                let saved = file
                    .write_all(value)
                    .and_then(|()| file.sync_all())
                    .and_then(|()| fs::rename(&stage, directory.join(key)));
                if saved.is_err() {
                    let _ = fs::remove_file(stage);
                }
            }
        });
    }
}
