//! A person's session with the hosted auth service (GoTrue, as Supabase serves it under
//! `/auth/v1`): password sign-in, refresh and sign-out. The access token is the JWT the REST
//! authority presents; the role the device acts as is read from that token's `app_metadata`,
//! which only an admin or service call can set. Tokens live in memory only.
use std::time::Duration;

use alm_core::Role;
use serde_json::{Value, json};

/// Seconds before expiry at which [`GoTrueSession::access_token`] refreshes.
const REFRESH_MARGIN: u64 = 2;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthError {
    /// The service refused: wrong email or password, or a refresh token that was revoked or
    /// used up (sign in again). Carries the service's message.
    Refused(String),
    /// No answer from the service.
    Unavailable,
    /// The answer was not a session.
    Invalid,
}

pub struct GoTrueSession {
    base: String,
    apikey: String,
    access: String,
    refresh: String,
    expires_at: u64,
    user: String,
    email: String,
    agent: ureq::Agent,
}

impl std::fmt::Debug for GoTrueSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GoTrueSession")
            .field("base", &self.base)
            .field("user", &self.user)
            .field("expires_at", &self.expires_at)
            .finish_non_exhaustive()
    }
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_global(Some(Duration::from_secs(20)))
        .build()
        .into()
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

impl GoTrueSession {
    /// Signs in with email and password. `base` is the auth root (`http://127.0.0.1:9999`, or a
    /// Supabase project's `https://<ref>.supabase.co/auth/v1`); `apikey` its public key.
    ///
    /// # Errors
    /// [`AuthError`].
    pub fn sign_in(
        base: &str,
        apikey: &str,
        email: &str,
        password: &str,
    ) -> Result<Self, AuthError> {
        let mut session = Self {
            base: base.trim_end_matches('/').to_owned(),
            apikey: apikey.to_owned(),
            access: String::new(),
            refresh: String::new(),
            expires_at: 0,
            user: String::new(),
            email: String::new(),
            agent: agent(),
        };
        let answer = session.post(
            "/token?grant_type=password",
            None,
            &json!({"email": email, "password": password}),
        )?;
        session.take(&answer)?;
        Ok(session)
    }

    /// Resumes a session from a refresh token kept on the device (no password): a refresh
    /// exchanges it for a new access token and a new refresh token.
    ///
    /// # Errors
    /// [`AuthError`]; `Refused` when the refresh token was revoked or used up.
    pub fn resume(base: &str, apikey: &str, refresh_token: &str) -> Result<Self, AuthError> {
        let mut session = Self {
            base: base.trim_end_matches('/').to_owned(),
            apikey: apikey.to_owned(),
            access: String::new(),
            refresh: refresh_token.to_owned(),
            expires_at: 0,
            user: String::new(),
            email: String::new(),
            agent: agent(),
        };
        session.refresh()?;
        Ok(session)
    }

    /// The current refresh token, for the platform credential store (it changes on every
    /// refresh).
    pub fn refresh_token(&self) -> &str {
        &self.refresh
    }

    /// Exchanges the refresh token for a new access token (and refresh token).
    ///
    /// # Errors
    /// [`AuthError`]; `Refused` when the refresh token was revoked (signed out elsewhere).
    pub fn refresh(&mut self) -> Result<(), AuthError> {
        let answer = self.post(
            "/token?grant_type=refresh_token",
            None,
            &json!({"refresh_token": self.refresh}),
        )?;
        self.take(&answer)
    }

    /// The access token, refreshed first when it expires within two seconds of `now`
    /// (seconds since the epoch).
    ///
    /// # Errors
    /// As [`Self::refresh`].
    pub fn access_token_at(&mut self, now: u64) -> Result<&str, AuthError> {
        if now + REFRESH_MARGIN >= self.expires_at {
            self.refresh()?;
        }
        Ok(&self.access)
    }

    /// [`Self::access_token_at`] with the wall clock.
    ///
    /// # Errors
    /// As [`Self::refresh`].
    pub fn access_token(&mut self) -> Result<&str, AuthError> {
        self.access_token_at(now())
    }

    /// Ends the session at the service (its refresh tokens are revoked).
    ///
    /// # Errors
    /// [`AuthError`]; the session is dropped either way.
    pub fn sign_out(self) -> Result<(), AuthError> {
        self.post("/logout", Some(&self.access), &json!({}))
            .map(|_| ())
    }

    /// The role the token grants (`app_metadata.role`), if any.
    pub fn role(&self) -> Option<Role> {
        token_role(&self.access)
    }

    /// The signed-in user's ID (the token's subject).
    pub fn user_id(&self) -> &str {
        &self.user
    }

    pub fn email(&self) -> &str {
        &self.email
    }

    /// When the access token expires (seconds since the epoch).
    pub const fn expires_at(&self) -> u64 {
        self.expires_at
    }

    fn post(&self, path: &str, bearer: Option<&str>, body: &Value) -> Result<Value, AuthError> {
        let mut request = self
            .agent
            .post(&format!("{}{path}", self.base))
            .header("apikey", &self.apikey)
            .header("Content-Type", "application/json");
        if let Some(token) = bearer {
            request = request.header("Authorization", &format!("Bearer {token}"));
        }
        let mut response = request
            .send(body.to_string())
            .map_err(|_| AuthError::Unavailable)?;
        let status = response.status().as_u16();
        let text = response
            .body_mut()
            .read_to_string()
            .map_err(|_| AuthError::Unavailable)?;
        let value = if text.trim().is_empty() {
            Value::Null
        } else {
            serde_json::from_str(&text).map_err(|_| AuthError::Invalid)?
        };
        match status {
            200..=299 => Ok(value),
            400..=499 => Err(AuthError::Refused(
                ["error_description", "msg", "message", "error"]
                    .iter()
                    .find_map(|k| value[*k].as_str())
                    .unwrap_or("refused")
                    .to_owned(),
            )),
            _ => Err(AuthError::Unavailable),
        }
    }

    fn take(&mut self, answer: &Value) -> Result<(), AuthError> {
        let (Some(access), Some(refresh)) = (
            answer["access_token"].as_str(),
            answer["refresh_token"].as_str(),
        ) else {
            return Err(AuthError::Invalid);
        };
        let claims = token_claims(access).ok_or(AuthError::Invalid)?;
        self.expires_at = answer["expires_at"]
            .as_u64()
            .or_else(|| claims["exp"].as_u64())
            .ok_or(AuthError::Invalid)?;
        access.clone_into(&mut self.access);
        refresh.clone_into(&mut self.refresh);
        self.user = claims["sub"].as_str().unwrap_or_default().to_owned();
        self.email = claims["email"].as_str().unwrap_or_default().to_owned();
        Ok(())
    }
}

/// The role a JWT's `app_metadata.role` grants; `user_metadata` is never read.
pub fn token_role(jwt: &str) -> Option<Role> {
    match token_claims(jwt)?["app_metadata"]["role"].as_str()? {
        "owner" => Some(Role::Owner),
        "manager" => Some(Role::Manager),
        _ => None,
    }
}

/// A JWT's claims (its payload), unverified: the authority verifies the signature.
pub fn token_claims(jwt: &str) -> Option<Value> {
    let payload = jwt.split('.').nth(1)?;
    serde_json::from_slice(&base64url_decode(payload)?).ok()
}

fn base64url_decode(text: &str) -> Option<Vec<u8>> {
    let mut bits = 0_u32;
    let mut count = 0;
    let mut out = Vec::with_capacity(text.len() * 3 / 4);
    for byte in text.bytes().filter(|b| *b != b'=') {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'-' | b'+' => 62,
            b'_' | b'/' => 63,
            _ => return None,
        };
        bits = (bits << 6) | u32::from(value);
        count += 6;
        if count >= 8 {
            count -= 8;
            out.push(u8::try_from((bits >> count) & 0xff).ok()?);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_role_comes_from_app_metadata_only() {
        let token = |claims: &str| {
            let encode = |bytes: &[u8]| {
                const A: &[u8; 64] =
                    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
                let mut out = String::new();
                for chunk in bytes.chunks(3) {
                    let n = chunk.iter().fold(0_u32, |n, b| (n << 8) | u32::from(*b))
                        << (8 * (3 - chunk.len()));
                    for i in 0..=chunk.len() {
                        out.push(char::from(A[((n >> (18 - 6 * i)) & 63) as usize]));
                    }
                }
                out
            };
            format!("e30.{}.sig", encode(claims.as_bytes()))
        };
        assert_eq!(
            token_role(&token(r#"{"app_metadata":{"role":"owner"}}"#)),
            Some(Role::Owner)
        );
        assert_eq!(
            token_role(&token(
                r#"{"app_metadata":{"role":"manager"},"user_metadata":{"role":"owner"}}"#
            )),
            Some(Role::Manager)
        );
        assert_eq!(
            token_role(&token(r#"{"user_metadata":{"role":"owner"}}"#)),
            None
        );
        assert_eq!(token_role("not a token"), None);
        assert_eq!(base64url_decode("aGk"), Some(b"hi".to_vec()));
    }
}
