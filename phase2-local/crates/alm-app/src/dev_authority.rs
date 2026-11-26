//! DEVELOPMENT ONLY: an Owner may point this device at a REST authority (a local PostgREST over
//! the migrated schema, with its GoTrue auth service) instead of the built-in simulation. The
//! REST and auth URLs and the public key are kept in this device's data folder; each person
//! then signs in with their own email and password and the role comes only from their token.
//! Access tokens stay in memory; only the refresh token is kept, in the platform credential
//! store (`credentials`). Never put real credentials in the repository.
use std::path::Path;

use alm_core::Role;
use alm_sync::{AuthError, GoTrueSession, RestAuthority, SalesAuthority};
use tauri::State;

use crate::gst_print::esc;
use crate::{AppState, Trial, text, with};

/// Three lines: REST URL, auth URL, public key. No token is ever written.
const FILE: &str = "rest-authority.txt";

/// The REST authority this device signs in to.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RestConfig {
    pub(crate) url: String,
    pub(crate) auth: String,
    pub(crate) apikey: String,
}

pub(crate) fn read(device_root: &Path) -> Option<RestConfig> {
    let text = std::fs::read_to_string(device_root.join(FILE)).ok()?;
    let mut lines = text.lines().map(str::to_owned);
    Some(RestConfig {
        url: lines.next()?,
        auth: lines.next()?,
        apikey: lines.next().unwrap_or_default(),
    })
}

/// This device install's identity: in its request IDs and the authority's activity log. Made
/// once, kept in the device data folder (`device-id.txt`) across restarts and restores; an ID
/// made by an earlier build is kept as it is.
pub(crate) fn device_id(device_root: &Path) -> Result<String, String> {
    let path = device_root.join("device-id.txt");
    if let Ok(id) = std::fs::read_to_string(&path) {
        let id = id.trim();
        if alm_sync::RequestId::new(id).is_ok() && id.len() <= 32 && !id.contains(':') {
            return Ok(id.to_owned());
        }
    }
    let id = format!("dev-{:016x}", random_u64());
    std::fs::write(&path, format!("{id}\n")).map_err(text)?;
    Ok(id)
}

/// 64 random bits from std only: the process's randomly keyed `RandomState` hashers over the
/// time, the process ID and a stack address.
fn random_u64() -> u64 {
    use std::hash::{BuildHasher, Hasher};
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let marker = 0u8;
    let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
    hasher.write_u128(nanos);
    hasher.write_u32(std::process::id());
    hasher.write_usize(std::ptr::addr_of!(marker) as usize);
    let first = hasher.finish();
    let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
    hasher.write_u64(first);
    hasher.write_u128(nanos);
    first ^ hasher.finish().rotate_left(29)
}

/// The Settings card (Owner only).
pub(crate) fn card(device_root: &Path, signed_in: Option<(&str, Role)>) -> String {
    let config = read(device_root);
    let status = match (&config, signed_in) {
        (None, _) => "Using the built-in authority simulation.".to_owned(),
        (Some(c), Some((email, role))) => format!(
            "Using the REST authority at <b>{}</b>, signed in as <b>{}</b> ({}).",
            esc(&c.url),
            esc(email),
            role_name(role)
        ),
        (Some(c), None) => format!("Using the REST authority at <b>{}</b>.", esc(&c.url)),
    };
    let (url, auth) = config.map(|c| (c.url, c.auth)).unwrap_or_default();
    format!(
        r#"<div class="card" style="margin-bottom:14px"><div class="sectit">🧪 Development authority (REST)</div><p class="help">Development only, this device only: send commands and pulls to a PostgREST URL instead of the built-in simulation; each person then signs in with their own account and their role comes from the authority. Never enter real credentials. {status}</p><form data-action="settings_rest" data-payload="terms"><div class="f2"><div><label>REST URL <input class="inp" name="url" placeholder="http://127.0.0.1:3000" value="{}"></label></div><div><label>Sign-in (auth) URL <input class="inp" name="auth" placeholder="http://127.0.0.1:9999" value="{}"></label></div></div><div class="field"><label>API key <input class="inp" name="apikey" placeholder="public (anon) key"></label></div><div class="row"><button class="btn primary" type="submit">Use this REST authority</button><button class="btn" type="button" data-action="settings_rest_off">Use the built-in simulation</button></div></form></div>"#,
        esc(&url),
        esc(&auth)
    )
}

const fn role_name(role: Role) -> &'static str {
    match role {
        Role::Owner => "Owner",
        Role::Manager => "Manager",
    }
}

/// The sign-in screen shown for every tab while this device uses a REST authority and nobody
/// is signed in. The role is never chosen here: it comes from the account's token.
pub(crate) fn login_screen(config: &RestConfig, message: &str) -> String {
    format!(
        r#"<div class="card" style="max-width:420px;margin:24px auto"><div class="sectit">Sign in</div><p class="help">REST authority <b>{}</b> (development). Sign in with your own account; what you may do comes from the role the Owner gave it. This device keeps you signed in (in the system's credential store) until you sign out.</p>{}<form data-action="rest_login" data-payload="terms"><div class="field"><label>Email <input class="inp" name="email" type="email" autocomplete="username" required></label></div><div class="field"><label>Password <input class="inp" name="password" type="password" autocomplete="current-password" required></label></div><div class="row"><button class="btn primary" type="submit">Sign in</button><button class="btn" type="button" data-action="settings_rest_off_signed_out">Use the built-in simulation</button></div></form></div>"#,
        esc(&config.url),
        if message.is_empty() {
            String::new()
        } else {
            format!(r#"<div class="help" role="status">{}</div>"#, esc(message))
        }
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
    auth: String,
    #[serde(default)]
    apikey: String,
}

#[cfg(test)]
impl RestForm {
    pub(crate) fn new(url: &str, auth: &str, apikey: &str) -> Self {
        Self {
            url: url.into(),
            auth: auth.into(),
            apikey: apikey.into(),
        }
    }
}

#[derive(serde::Deserialize)]
pub struct LoginForm {
    email: String,
    password: String,
}

#[cfg(test)]
impl LoginForm {
    pub(crate) fn new(email: &str, password: &str) -> Self {
        Self {
            email: email.into(),
            password: password.into(),
        }
    }
}

const SESSION_ENDED: &str = "Your session ended: sign in again.";

/// The account signed in last (its user ID; not a secret): whose refresh token to resume with.
const SIGNED_IN: &str = "signed-in-account.txt";

impl Trial {
    /// The authority commands and pulls go to.
    pub(crate) fn sales_authority(&mut self) -> &mut dyn SalesAuthority {
        match &mut self.rest {
            Some(rest) => rest,
            None => &mut self.authority,
        }
    }

    /// Where commands go, for confirmation messages.
    pub(crate) const fn authority_name(&self) -> &'static str {
        if self.rest.is_some() || self.rest_config.is_some() {
            "by the REST authority (development)"
        } else {
            "in the local simulation"
        }
    }

    /// This device uses a REST authority and nobody is signed in: only the sign-in screen.
    pub(crate) const fn signed_out(&self) -> bool {
        self.rest_config.is_some() && self.session.is_none()
    }

    /// The signed-in account's email and role, if a person signed in.
    pub(crate) fn signed_in(&self) -> Option<(&str, Role)> {
        self.session.as_ref().map(|s| (s.email(), self.role()))
    }

    /// Before talking to the REST authority: refuses while signed out, and presents a fresh
    /// token (refreshed near expiry). A refresh the auth service refuses ends the session.
    pub(crate) fn session_token(&mut self) -> Result<(), String> {
        if self.signed_out() {
            return Err("Sign in first.".into());
        }
        let Some(session) = self.session.as_mut() else {
            return Ok(());
        };
        match session.access_token() {
            Ok(token) => {
                let token = token.to_owned();
                let role = self.role();
                if let Some(rest) = self.rest.as_mut() {
                    rest.set_token(role, &token);
                }
                self.keep_refresh_token()
            }
            // Offline: the request will find no answer either.
            Err(AuthError::Unavailable) => Ok(()),
            Err(AuthError::Refused(_) | AuthError::Invalid) => {
                self.end_session(SESSION_ENDED)?;
                Err(SESSION_ENDED.into())
            }
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
        let auth = form.auth.trim();
        let http = |u: &str| u.starts_with("http://") || u.starts_with("https://");
        if !http(url) || !http(auth) {
            return Err("Enter the http(s) REST URL and sign-in URL.".into());
        }
        let fields = [url, auth, form.apikey.trim()];
        if fields.iter().any(|f| f.contains(['\n', '\r'])) {
            return Err("Enter each value on one line.".into());
        }
        std::fs::write(self.device_root.join(FILE), fields.join("\n") + "\n").map_err(text)?;
        self.end_session("This device now uses the REST authority (development): sign in.")?;
        self.rest_config = read(&self.device_root);
        self.rest = None;
        self.projection.clear().map_err(text)?;
        self.posted = crate::device::Posted::empty()?;
        Ok(())
    }

    pub(crate) fn settings_rest_off(&mut self) -> Result<(), String> {
        if !self.signed_out() {
            self.require_owner()?;
        }
        match std::fs::remove_file(self.device_root.join(FILE)) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(text(error)),
        }
        self.end_session("")?;
        self.rest_config = None;
        if self.account != crate::trial::ACCOUNT {
            self.switch_account(crate::trial::ACCOUNT)?;
        }
        self.owner = true;
        self.use_authority(None, "This device uses the built-in authority simulation.")
    }

    /// Signs a person in: their token's role decides what this device may do, and their
    /// account's own device tables open (another account's stay closed).
    pub(crate) fn rest_login(&mut self, form: &LoginForm) -> Result<(), String> {
        let config = self
            .rest_config
            .clone()
            .ok_or("This device uses the built-in simulation.")?;
        let session = GoTrueSession::sign_in(
            &config.auth,
            &config.apikey,
            form.email.trim(),
            &form.password,
        )
        .map_err(|error| match error {
            AuthError::Refused(reason) => format!("Sign-in refused: {reason}."),
            AuthError::Unavailable => "No answer from the sign-in service.".to_owned(),
            AuthError::Invalid => "The sign-in service sent no session.".to_owned(),
        })?;
        self.start_session(session)
    }

    /// The session's token decides the role; the account's own device tables open and its
    /// refresh token is kept in the platform credential store for the next launch.
    fn start_session(&mut self, mut session: GoTrueSession) -> Result<(), String> {
        let config = self
            .rest_config
            .clone()
            .ok_or("This device uses the built-in simulation.")?;
        let Some(role) = session.role() else {
            let _ = session.sign_out();
            return Err("This account has no role at the authority: ask the Owner.".into());
        };
        let token = session
            .access_token()
            .map_err(|_| SESSION_ENDED)?
            .to_owned();
        let device = device_id(&self.device_root)?;
        let account = session.user_id().to_owned();
        self.rest = Some(
            RestAuthority::new(&config.url, &config.apikey)
                .with_token(role, &token)
                .with_device(&device),
        );
        self.owner = role == Role::Owner;
        self.session = Some(session);
        self.switch_account(&account)?;
        self.saved_refresh = Some(String::new());
        self.keep_refresh_token()?;
        std::fs::write(self.device_root.join(SIGNED_IN), &account).map_err(text)?;
        self.message = format!(
            "Signed in as {} ({}).",
            self.session.as_ref().map_or("", |s| s.email()),
            role_name(role)
        );
        Ok(())
    }

    /// At launch: the account signed in last resumes with its stored refresh token, without a
    /// password. A refused token (signed out elsewhere, revoked or used up) is deleted and the
    /// sign-in screen shows; with no answer the token stays for the next attempt.
    pub(crate) fn resume_session(&mut self) -> Result<(), String> {
        let Some(config) = self.rest_config.clone() else {
            return Ok(());
        };
        let Ok(account) = std::fs::read_to_string(self.device_root.join(SIGNED_IN)) else {
            return Ok(());
        };
        let Ok(Some(token)) = self.credentials.load(&account) else {
            return self.forget_credentials(&account);
        };
        match GoTrueSession::resume(&config.auth, &config.apikey, &token) {
            Ok(session) if session.user_id() == account => match self.start_session(session) {
                Ok(()) => Ok(()),
                Err(error) => {
                    self.forget_credentials(&account)?;
                    self.message = error;
                    Ok(())
                }
            },
            Ok(session) => {
                let _ = session.sign_out();
                self.forget_credentials(&account)?;
                SESSION_ENDED.clone_into(&mut self.message);
                Ok(())
            }
            Err(AuthError::Unavailable) => {
                "No answer from the sign-in service: sign in again when it answers."
                    .clone_into(&mut self.message);
                Ok(())
            }
            Err(AuthError::Refused(_) | AuthError::Invalid) => {
                self.forget_credentials(&account)?;
                SESSION_ENDED.clone_into(&mut self.message);
                Ok(())
            }
        }
    }

    /// Writes the session's refresh token to the credential store when it changed.
    fn keep_refresh_token(&mut self) -> Result<(), String> {
        let (Some(session), Some(saved)) = (self.session.as_ref(), self.saved_refresh.as_mut())
        else {
            return Ok(());
        };
        if session.refresh_token() != saved {
            self.credentials
                .save(session.user_id(), session.refresh_token())?;
            session.refresh_token().clone_into(saved);
        }
        Ok(())
    }

    /// Deletes `account`'s stored refresh token and the signed-in marker.
    pub(crate) fn forget_credentials(&mut self, account: &str) -> Result<(), String> {
        self.credentials.delete(account)?;
        self.saved_refresh = None;
        match std::fs::remove_file(self.device_root.join(SIGNED_IN)) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(text(error)),
        }
    }

    /// Signs out (the auth service ends the session) and removes the account's posted data
    /// from this device, as a revocation does; its drafts and queued actions stay in its own
    /// tables for its next sign-in.
    pub(crate) fn rest_logout(&mut self) -> Result<(), String> {
        self.end_session("Signed out. Sign in to continue.")
    }

    fn end_session(&mut self, message: &str) -> Result<(), String> {
        if let Some(session) = self.session.take() {
            self.forget_credentials(session.user_id())?;
            let _ = session.sign_out();
            // The open file is purged in place (Windows refuses to delete an open file); the
            // other role's file is deleted.
            let other = match self.projection.role() {
                Role::Owner => Role::Manager,
                Role::Manager => Role::Owner,
            };
            alm_store::ProjectionStore::remove_scoped(&self.device_root, &self.account, other)
                .map_err(text)?;
            self.projection.purge().map_err(text)?;
            self.projection = alm_store::ProjectionStore::open_scoped(
                &self.device_root,
                &self.account,
                self.role(),
            )
            .map_err(text)?;
            self.posted = crate::device::Posted::empty()?;
            self.current = None;
            self.reviewed = None;
            self.posting.invalidate();
            self.legacy = None;
        }
        self.rest = None;
        message.clone_into(&mut self.message);
        Ok(())
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

#[tauri::command(async)]
pub fn settings_rest_off_signed_out(state: State<'_, AppState>) -> Result<String, String> {
    with(&state, "review", Trial::settings_rest_off)
}

#[tauri::command(async)]
pub fn rest_login(state: State<'_, AppState>, terms: LoginForm) -> Result<String, String> {
    with(&state, "review", |trial| trial.rest_login(&terms))
}

#[tauri::command(async)]
pub fn rest_logout(state: State<'_, AppState>) -> Result<String, String> {
    with(&state, "review", Trial::rest_logout)
}
