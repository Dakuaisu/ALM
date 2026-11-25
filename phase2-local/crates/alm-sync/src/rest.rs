//! The device transport: `SalesAuthority` over PostgREST RPC (`POST <base>/rpc/<function>`, as
//! Supabase serves it under `/rest/v1`), with the account's JWT as the Bearer token and the
//! project's public API key. The functions check the role claim themselves; refusals arrive as
//! the SQLSTATE codes of the migrations and map to typed errors. No credentials live here.
use std::time::Duration;

use alm_core::Role;
use alm_core::projection::ProjectionBatch;
use serde_json::{Value, json};

use crate::{AuthorityError, Intent, Lookup, Outcome, RequestId, SalesAuthority, wire};

pub struct RestAuthority {
    base: String,
    apikey: String,
    /// The JWT presented for each role this device may act as.
    tokens: Vec<(Role, String)>,
    /// This device's identity, sent as `x-alm-device` for the authority's activity log.
    device: String,
    agent: ureq::Agent,
}

impl RestAuthority {
    /// `base` is the PostgREST root (e.g. `http://127.0.0.1:3000` or a Supabase project's
    /// `https://<ref>.supabase.co/rest/v1`); `apikey` is the public (anon) key.
    pub fn new(base: &str, apikey: &str) -> Self {
        let agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(Duration::from_secs(20)))
            .build()
            .into();
        Self {
            base: base.trim_end_matches('/').to_owned(),
            apikey: apikey.to_owned(),
            tokens: Vec::new(),
            device: String::new(),
            agent,
        }
    }

    /// Presents `jwt` when acting as `role`.
    #[must_use]
    pub fn with_token(mut self, role: Role, jwt: &str) -> Self {
        self.tokens.retain(|(r, _)| *r != role);
        self.tokens.push((role, jwt.to_owned()));
        self
    }

    /// Replaces the JWT presented as `role` (a refreshed session token).
    pub fn set_token(&mut self, role: Role, jwt: &str) {
        self.tokens.retain(|(r, _)| *r != role);
        self.tokens.push((role, jwt.to_owned()));
    }

    /// Names this device in every request (`x-alm-device`).
    #[must_use]
    pub fn with_device(mut self, device: &str) -> Self {
        device.clone_into(&mut self.device);
        self
    }

    /// Calls one RPC function as `role`. A role without a token is denied before sending.
    ///
    /// # Errors
    /// The function's refusal as a typed error; `Unavailable` when no answer arrived.
    pub fn rpc(&self, role: Role, function: &str, args: &Value) -> Result<Value, AuthorityError> {
        let token = self
            .tokens
            .iter()
            .find(|(r, _)| *r == role)
            .map(|(_, t)| t)
            .ok_or(AuthorityError::Denied)?;
        let mut response = self
            .agent
            .post(&format!("{}/rpc/{function}", self.base))
            .header("apikey", &self.apikey)
            .header("Authorization", &format!("Bearer {token}"))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("x-alm-device", &self.device)
            .send(args.to_string())
            .map_err(|_| AuthorityError::Unavailable)?;
        let status = response.status().as_u16();
        let body = response
            .body_mut()
            .read_to_string()
            .map_err(|_| AuthorityError::Unavailable)?;
        let value: Value = if body.trim().is_empty() {
            Value::Null
        } else {
            serde_json::from_str(&body).map_err(|_| AuthorityError::Unavailable)?
        };
        match status {
            200..=299 => Ok(value),
            _ => Err(refusal(status, &value)),
        }
    }
}

/// A non-2xx answer: the function's SQLSTATE when there is one, else by HTTP status. A server
/// error without a code may have committed, so it stays unknown.
fn refusal(status: u16, value: &Value) -> AuthorityError {
    match value["code"].as_str() {
        Some(code) if code.starts_with("ALM") || code == "42501" => {
            wire::refusal(code, value["message"].as_str().unwrap_or_default())
        }
        // PostgREST's JWT errors: PGRST301 invalid, PGRST302 none sent, PGRST303 expired or a
        // claim not valid.
        Some("PGRST301" | "PGRST302" | "PGRST303") => AuthorityError::SignedOut,
        _ => match status {
            401 | 403 => AuthorityError::Denied,
            400 | 404 | 409 | 422 => AuthorityError::Invalid,
            _ => AuthorityError::Unavailable,
        },
    }
}

impl SalesAuthority for RestAuthority {
    fn submit(
        &mut self,
        role: Role,
        request: &RequestId,
        intent: &Intent,
    ) -> Result<Outcome, AuthorityError> {
        let (function, terms) = wire::command(intent);
        let value = self.rpc(
            role,
            function,
            &json!({"p_request_id": request.as_str(), "p_terms": terms}),
        )?;
        wire::outcome(&value)
    }

    fn status(&self, role: Role, request: &RequestId) -> Result<Option<Outcome>, AuthorityError> {
        let value = self.rpc(
            role,
            "request_status",
            &json!({"p_request_id": request.as_str()}),
        )?;
        wire::status(&value)
    }

    fn lookup(
        &self,
        role: Role,
        request: &RequestId,
        intent: &Intent,
    ) -> Result<Lookup, AuthorityError> {
        let (function, terms) = wire::command(intent);
        let value = self.rpc(
            role,
            "request_lookup",
            &json!({"p_request_id": request.as_str(), "p_command": function, "p_terms": terms}),
        )?;
        wire::lookup(&value)
    }

    fn allocate_gst_number(&mut self, role: Role, bill_id: &str) -> Result<u64, AuthorityError> {
        self.rpc(
            role,
            "allocate_gst_bill_number",
            &json!({"p_bill_id": bill_id}),
        )?
        .as_u64()
        .ok_or(AuthorityError::Invalid)
    }

    fn pull(&self, role: Role, checkpoint: Option<u64>) -> Result<ProjectionBatch, AuthorityError> {
        let mut value = self.rpc(
            role,
            "pull_projection",
            &json!({"p_checkpoint": checkpoint}),
        )?;
        if !value["rows"].is_null() && value["full"] != json!(true) {
            // Something changed: the device applies full replacements, so read everything.
            value = self.rpc(role, "pull_projection", &json!({"p_checkpoint": null}))?;
        }
        wire::decode_batch(&value, role)
    }
}
