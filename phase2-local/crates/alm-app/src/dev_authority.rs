//! DEVELOPMENT ONLY: an Owner may point this device at a REST authority (a local PostgREST over
//! the migrated schema) instead of the built-in simulation. The URL, public key and token are
//! kept in this device's data folder only; never put real credentials in the repository.
use std::path::Path;

use alm_core::Role;
use alm_sync::{RestAuthority, SalesAuthority};
use tauri::State;

use crate::gst_print::esc;
use crate::{AppState, Trial, text, with};

/// Three lines: URL, public key, token.
const FILE: &str = "dev-authority.txt";

fn read(device_root: &Path) -> Option<[String; 3]> {
    let text = std::fs::read_to_string(device_root.join(FILE)).ok()?;
    let mut lines = text.lines().map(str::to_owned);
    Some([lines.next()?, lines.next()?, lines.next()?])
}

/// The REST authority saved on this device, if any. The device presents its one token whatever
/// role it acts as; the authority decides from the token.
pub(crate) fn load(device_root: &Path) -> Option<RestAuthority> {
    let [url, apikey, token] = read(device_root)?;
    Some(
        RestAuthority::new(&url, &apikey)
            .with_token(Role::Owner, &token)
            .with_token(Role::Manager, &token),
    )
}

/// The Settings card (Owner only).
pub(crate) fn card(device_root: &Path) -> String {
    let url = read(device_root).map(|[url, _, _]| url).unwrap_or_default();
    let status = if url.is_empty() {
        "Using the built-in authority simulation.".to_owned()
    } else {
        format!("Using the REST authority at <b>{}</b>.", esc(&url))
    };
    format!(
        r#"<div class="card" style="margin-bottom:14px"><div class="sectit">🧪 Development authority (REST)</div><p class="help">Development only, this device only: send commands and pulls to a PostgREST URL instead of the built-in simulation. Never enter real credentials. {status}</p><form data-action="settings_rest" data-payload="terms"><div class="f2"><div><label>REST URL <input class="inp" name="url" placeholder="http://127.0.0.1:3000" value="{}"></label></div><div><label>API key <input class="inp" name="apikey" placeholder="public (anon) key"></label></div></div><div class="field"><label>Token (JWT) <input class="inp" name="token" type="password" autocomplete="off"></label></div><div class="row"><button class="btn primary" type="submit">Use this REST authority</button><button class="btn" type="button" data-action="settings_rest_off">Use the built-in simulation</button></div></form></div>"#,
        esc(&url)
    )
}

/// A new standalone GST bill's number from the authority in use (the fields apart, so a form
/// borrowed from the trial can stay open).
pub(crate) fn allocate_gst(
    rest: &mut Option<RestAuthority>,
    local: &mut alm_sync::LocalAuthority,
    bill: u64,
) -> Result<u64, String> {
    let key = alm_sync::gst_bill_key(bill);
    match rest {
        Some(rest) => rest.allocate_gst_number(Role::Owner, &key),
        None => local.allocate_gst_number(Role::Owner, &key),
    }
    .map_err(|e| crate::device::refused(&e))
}

#[derive(serde::Deserialize)]
pub struct RestForm {
    url: String,
    #[serde(default)]
    apikey: String,
    #[serde(default)]
    token: String,
}

#[cfg(test)]
impl RestForm {
    pub(crate) fn new(url: &str, apikey: &str, token: &str) -> Self {
        Self {
            url: url.into(),
            apikey: apikey.into(),
            token: token.into(),
        }
    }
}

impl Trial {
    /// The authority commands and pulls go to.
    pub(crate) fn sales_authority(&mut self) -> &mut dyn SalesAuthority {
        match &mut self.rest {
            Some(rest) => rest,
            None => &mut self.authority,
        }
    }

    /// Switches authorities: the stored projection belongs to the previous one, so it is
    /// rebuilt from the new one.
    fn use_authority(&mut self, rest: Option<RestAuthority>, message: &str) -> Result<(), String> {
        self.rest = rest;
        self.projection.clear().map_err(text)?;
        self.message = message.into();
        self.refresh()
    }

    pub(crate) fn settings_rest(&mut self, form: &RestForm) -> Result<(), String> {
        self.require_owner()?;
        let url = form.url.trim();
        if !(url.starts_with("http://") || url.starts_with("https://")) || form.token.is_empty() {
            return Err("Enter an http(s) REST URL and a token.".into());
        }
        let fields = [url, form.apikey.trim(), form.token.trim()];
        if fields.iter().any(|f| f.contains(['\n', '\r'])) {
            return Err("Enter each value on one line.".into());
        }
        std::fs::write(self.device_root.join(FILE), fields.join("\n") + "\n").map_err(text)?;
        let rest = load(&self.device_root);
        self.use_authority(
            rest,
            "This device now uses the REST authority (development).",
        )
    }

    pub(crate) fn settings_rest_off(&mut self) -> Result<(), String> {
        self.require_owner()?;
        match std::fs::remove_file(self.device_root.join(FILE)) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(text(error)),
        }
        self.use_authority(None, "This device uses the built-in authority simulation.")
    }
}

#[tauri::command(async)]
pub fn settings_rest(state: State<'_, AppState>, terms: RestForm) -> Result<String, String> {
    with(&state, "settings", |trial| trial.settings_rest(&terms))
}

#[tauri::command(async)]
pub fn settings_rest_off(state: State<'_, AppState>) -> Result<String, String> {
    with(&state, "settings", Trial::settings_rest_off)
}
