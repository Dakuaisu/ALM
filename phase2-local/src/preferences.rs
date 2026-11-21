//! Local presentation preferences, separate from the synthetic business archive.
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::{Money, confirmation::BalanceMode};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Theme {
    Light,
    Dark,
}

impl Theme {
    /// Value used by the legacy data-theme presentation attribute.
    pub const fn data_theme(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Preference {
    Theme(Theme),
    Privacy(bool),
    Balance(BalanceMode),
}

/// A session's local preferences; storage failure never changes authorization.
pub struct Preferences {
    directory: PathBuf,
    theme: Theme,
    privacy: bool,
    balance: BalanceMode,
}

impl Preferences {
    /// Missing, unreadable, or unrecognized keys retain their defaults.
    pub fn load(directory: &Path) -> Self {
        let theme = match fs::read(directory.join("theme")).as_deref() {
            Ok(b"dark") => Theme::Dark,
            _ => Theme::Light,
        };
        let privacy = matches!(fs::read(directory.join("privacy")).as_deref(), Ok(b"on"));
        let balance = match fs::read(directory.join("balance")).as_deref() {
            Ok(b"latest") => BalanceMode::Latest,
            _ => BalanceMode::AsOnBillDate,
        };
        Self {
            directory: directory.into(),
            theme,
            privacy,
            balance,
        }
    }

    pub const fn theme(&self) -> Theme {
        self.theme
    }
    pub const fn balance_mode(&self) -> BalanceMode {
        self.balance
    }

    /// Returns only a presentation projection, not an access-control decision.
    pub const fn present_money(&self, amount: Money) -> Option<Money> {
        if self.privacy { None } else { Some(amount) }
    }

    /// Applies immediately in this session even if saving the key fails.
    pub fn set(&mut self, preference: Preference) {
        let (key, value): (&str, &[u8]) = match preference {
            Preference::Theme(theme) => {
                self.theme = theme;
                (
                    "theme",
                    match theme {
                        Theme::Light => b"light",
                        Theme::Dark => b"dark",
                    },
                )
            }
            Preference::Privacy(privacy) => {
                self.privacy = privacy;
                ("privacy", if privacy { b"on" } else { b"off" })
            }
            Preference::Balance(balance) => {
                self.balance = balance;
                (
                    "balance",
                    match balance {
                        BalanceMode::AsOnBillDate => b"bill-date",
                        BalanceMode::Latest => b"latest",
                    },
                )
            }
        };
        let stage = self.directory.join(format!("{key}.stage"));
        if let Ok(mut file) = OpenOptions::new().write(true).create_new(true).open(&stage) {
            let saved = file
                .write_all(value)
                .and_then(|()| file.sync_all())
                .and_then(|()| fs::rename(&stage, self.directory.join(key)));
            if saved.is_err() {
                let _ = fs::remove_file(stage);
            }
        }
    }
}
