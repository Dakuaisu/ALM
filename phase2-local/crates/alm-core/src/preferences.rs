//! Local presentation preferences, separate from the synthetic business archive.
use crate::{Money, confirmation::BalanceMode};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Theme {
    Light,
    Dark,
    /// "Match device": the page follows the platform colour scheme.
    System,
}

impl Theme {
    /// Value used by the legacy data-theme presentation attribute.
    pub const fn data_theme(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
            Self::System => "system",
        }
    }
}

impl Theme {
    /// The Appearance segment's value (`light`, `dark`, `system`); anything else is Light.
    pub fn parse(value: &str) -> Self {
        match value {
            "dark" => Self::Dark,
            "system" => Self::System,
            _ => Self::Light,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Preference {
    Theme(Theme),
    Privacy(bool),
    Balance(BalanceMode),
}

/// A session's local preference values; the caller persists each changed key.
pub struct PreferenceState {
    theme: Theme,
    privacy: bool,
    balance: BalanceMode,
}

impl PreferenceState {
    /// Missing, unreadable (`None`), or unrecognized keys retain their defaults.
    pub fn load(mut read: impl FnMut(&str) -> Option<Vec<u8>>) -> Self {
        let theme = match read("theme").as_deref() {
            Some(b"dark") => Theme::Dark,
            Some(b"system") => Theme::System,
            _ => Theme::Light,
        };
        let privacy = matches!(read("privacy").as_deref(), Some(b"on"));
        let balance = match read("balance").as_deref() {
            Some(b"latest") => BalanceMode::Latest,
            _ => BalanceMode::AsOnBillDate,
        };
        Self {
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

    /// Applies immediately in this session, then hands the changed key to `save`.
    pub fn set(&mut self, preference: Preference, save: impl FnOnce(&str, &[u8])) {
        let (key, value): (&str, &[u8]) = match preference {
            Preference::Theme(theme) => {
                self.theme = theme;
                (
                    "theme",
                    match theme {
                        Theme::Light => b"light",
                        Theme::Dark => b"dark",
                        Theme::System => b"system",
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
        save(key, value);
    }
}
