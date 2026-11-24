use alm_core::confirmation::BalanceMode;
use alm_core::preferences::{Preference, Theme};
use alm_core::synthetic_read_model::{ProfileInput, parse_reorder_default};
use tauri::State;

use crate::{AppState, Trial, debug, with};

/// The device name recorded with shared profile changes.
pub const DEVICE: &str = "this-device";

#[derive(serde::Deserialize, Default)]
pub struct Profile {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub tagline: String,
    #[serde(default)]
    pub owner: String,
    #[serde(default)]
    pub mobile: String,
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub loading: String,
    pub gstin: String,
    pub fssai: String,
    #[serde(default)]
    pub show_fssai: String,
    #[serde(default)]
    pub show_gstin: String,
    pub reorder: String,
}

impl Trial {
    /// The authority's default reorder level, from this account's book rows.
    pub(crate) fn reorder_default(&mut self) -> Result<u32, String> {
        Ok(self
            .projected()?
            .snapshot
            .owner
            .map_or(5, |owner| owner.book.settings.reorder_default))
    }

    /// Sends a changed default reorder level to the authority, like every other posted setting.
    pub(crate) fn send_reorder_default(&mut self, level: u32) -> Result<(), String> {
        if self.reorder_default()? != level {
            self.send(&alm_sync::Intent::SetReorderDefault { level })?;
        }
        Ok(())
    }

    pub(crate) fn settings_profile(&mut self, form: &Profile, now: u64) -> Result<(), String> {
        let role = self.require_owner()?;
        let reorder = parse_reorder_default(&form.reorder)
            .ok_or("Default Reorder Level must be a whole number.")?;
        let profile = self
            .shop
            .business_profile(role)
            .map_err(debug)?
            .with_input(&ProfileInput {
                name: &form.name,
                tagline: &form.tagline,
                owner: &form.owner,
                mobile: &form.mobile,
                address: &form.address,
                gstin: &form.gstin,
                fssai: &form.fssai,
                show_fssai: &form.show_fssai,
                show_gstin: &form.show_gstin,
                loading: &form.loading,
            })
            .map_err(|_| "Default Loading must be a non-negative amount.".to_owned())?;
        self.shop
            .save_business_profile(role, &profile, now, DEVICE)
            .map_err(debug)?;
        self.shop
            .set_reorder_default(role, reorder)
            .map_err(debug)?;
        self.send_reorder_default(reorder)?;
        self.refresh()?;
        self.message = "Profile saved ✓".into();
        Ok(())
    }

    pub(crate) fn settings_preference(&mut self, preference: Preference) -> Result<(), String> {
        self.require_owner()?;
        self.prefs.set(preference);
        Ok(())
    }
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| {
            u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
        })
}

#[tauri::command(async)]
pub fn settings_profile(state: State<'_, AppState>, terms: Profile) -> Result<String, String> {
    with(&state, "settings", |trial| {
        trial.settings_profile(&terms, now())
    })
}

#[tauri::command(async)]
pub fn settings_theme(state: State<'_, AppState>, q: String) -> Result<String, String> {
    let theme = Theme::parse(&q);
    with(&state, "settings", |trial| {
        trial.settings_preference(Preference::Theme(theme))
    })
}

#[tauri::command(async)]
pub fn settings_balance(state: State<'_, AppState>, q: String) -> Result<String, String> {
    let mode = if q == "latest" {
        BalanceMode::Latest
    } else {
        BalanceMode::AsOnBillDate
    };
    with(&state, "settings", |trial| {
        trial.settings_preference(Preference::Balance(mode))
    })
}

#[tauri::command(async)]
pub fn settings_privacy(state: State<'_, AppState>, q: String) -> Result<String, String> {
    with(&state, "settings", |trial| {
        trial.settings_preference(Preference::Privacy(q == "on"))
    })
}
