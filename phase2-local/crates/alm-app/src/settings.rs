use alm_core::confirmation::BalanceMode;
use alm_core::preferences::{Preference, Theme};
use alm_core::synthetic_read_model::{BusinessProfile, parse_reorder_default};
use tauri::State;

use crate::{AppState, Trial, debug, with};

/// The device name recorded with shared profile changes.
pub const DEVICE: &str = "this-device";

#[derive(serde::Deserialize)]
pub struct Profile {
    pub gstin: String,
    pub fssai: String,
    #[serde(default)]
    pub show_fssai: String,
    #[serde(default)]
    pub show_gstin: String,
    pub reorder: String,
}

impl Trial {
    pub(crate) fn settings_profile(&mut self, form: &Profile, now: u64) -> Result<(), String> {
        let role = self.require_owner()?;
        let reorder = parse_reorder_default(&form.reorder)
            .ok_or("Default Reorder Level must be a whole number.")?;
        let profile = BusinessProfile {
            gstin: form.gstin.trim().to_owned(),
            fssai: form.fssai.trim().to_owned(),
            show_fssai: Some(form.show_fssai == "on"),
            show_gstin: Some(form.show_gstin == "on"),
            ..self.shop.business_profile(role).map_err(debug)?.clone()
        };
        self.shop
            .save_business_profile(role, &profile, now, DEVICE)
            .map_err(debug)?;
        self.shop
            .set_reorder_default(role, reorder)
            .map_err(debug)?;
        self.shop
            .apply_product_settings(role, &mut self.authority)
            .map_err(debug)?;
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
    let theme = if q == "dark" {
        Theme::Dark
    } else {
        Theme::Light
    };
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
