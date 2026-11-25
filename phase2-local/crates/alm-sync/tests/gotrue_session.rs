//! Opt-in: real accounts through GoTrue (R-5). Set `ALM_GOTRUE_URL` (the auth root),
//! `ALM_REST_URL`, `ALM_PG_URL` and `ALM_JWT_SECRET` (the secret GoTrue and PostgREST share, to
//! mint the service token the admin API needs). The owner and manager users are created through
//! the admin API inside the test, with their role in `app_metadata`, and removed at the end.
//! Without the variables the test returns early.
#![forbid(unsafe_code)]

#[path = "rest/jwt.rs"]
mod jwt;
mod pg_authority;

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use alm_core::Role;
use alm_core::confirmation::demo_fixture::{directory, fixture, seed_purchasing};
use alm_sync::{
    AuthError, AuthorityError, GoTrueSession, Intent, LocalAuthority, RequestId, RestAuthority,
    SalesAuthority, token_role,
};
use serde_json::{Value, json};

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.is_empty())
}

struct Stack {
    auth: String,
    rest: String,
    secret: String,
    agent: ureq::Agent,
}

impl Stack {
    fn service(&self) -> String {
        jwt::mint_claims(
            &self.secret,
            &json!({"role": "service_role", "exp": now() + 600}),
        )
    }

    /// An admin call (`/admin/...`) with the service token; returns the JSON answer.
    fn admin(&self, method: &str, path: &str, body: &Value) -> Value {
        let url = format!("{}{path}", self.auth);
        let token = format!("Bearer {}", self.service());
        let mut response = match method {
            "POST" => self
                .agent
                .post(&url)
                .header("Authorization", &token)
                .header("apikey", "")
                .send(body.to_string()),
            "PUT" => self
                .agent
                .put(&url)
                .header("Authorization", &token)
                .header("apikey", "")
                .send(body.to_string()),
            _ => self
                .agent
                .delete(&url)
                .header("Authorization", &token)
                .header("apikey", "")
                .call(),
        }
        .unwrap();
        let status = response.status().as_u16();
        let text = response.body_mut().read_to_string().unwrap();
        assert!(
            (200..300).contains(&status),
            "{method} {path}: {status} {text}"
        );
        serde_json::from_str(&text).unwrap_or(Value::Null)
    }

    /// A confirmed user with `role` in `app_metadata`; returns its ID.
    fn user(&self, email: &str, password: &str, role: &str) -> String {
        self.admin(
            "POST",
            "/admin/users",
            &json!({"email": email, "password": password, "email_confirm": true,
                "app_metadata": {"role": role}}),
        )["id"]
            .as_str()
            .unwrap()
            .to_owned()
    }

    fn rest(&self, role: Role, token: &str) -> RestAuthority {
        RestAuthority::new(&self.rest, "").with_token(role, token)
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn stack() -> Option<Stack> {
    let stack = Stack {
        auth: env("ALM_GOTRUE_URL")?.trim_end_matches('/').to_owned(),
        rest: env("ALM_REST_URL")?,
        secret: env("ALM_JWT_SECRET")?,
        agent: ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(Duration::from_secs(20)))
            .build()
            .into(),
    };
    let mut client = postgres::Client::connect(&env("ALM_PG_URL")?, postgres::NoTls).unwrap();
    client
        .batch_execute(
            "DO $$ DECLARE t text; BEGIN
                 SELECT string_agg(format('public.%I', tablename), ', ') INTO t
                     FROM pg_tables WHERE schemaname = 'public';
                 EXECUTE 'TRUNCATE ' || t || ' RESTART IDENTITY CASCADE';
             END $$;
             INSERT INTO public.authority_state (id, revision) VALUES (1, 0);
             INSERT INTO public.invoice_counters (series, next_no) VALUES ('sale', 1);",
        )
        .unwrap();
    let mut shop = fixture().unwrap();
    seed_purchasing(&mut shop).unwrap();
    pg_authority::seed_shop(&mut client, &LocalAuthority::new(shop, directory()));
    Some(stack)
}

fn request(tag: &str) -> RequestId {
    RequestId::new(&format!("gotrue-{tag}-{}", now())).unwrap()
}

#[test]
fn owner_and_manager_sign_in_refresh_expire_and_sign_out_with_roles_from_app_metadata() {
    let Some(stack) = stack() else {
        return;
    };
    let run = format!("{}-{}", std::process::id(), now());
    let password = format!("synthetic-{run}");
    let owner_email = format!("owner-{run}@example.test");
    let manager_email = format!("manager-{run}@example.test");
    let owner_id = stack.user(&owner_email, &password, "owner");
    let manager_id = stack.user(&manager_email, &password, "manager");

    // Owner: signed in, the role from the token, commands and the Owner pull work.
    assert_eq!(
        GoTrueSession::sign_in(&stack.auth, "", &owner_email, "wrong").unwrap_err(),
        AuthError::Refused("Invalid login credentials".into())
    );
    let mut owner = GoTrueSession::sign_in(&stack.auth, "", &owner_email, &password).unwrap();
    assert_eq!(
        (owner.role(), owner.user_id()),
        (Some(Role::Owner), owner_id.as_str())
    );
    let token = owner.access_token().unwrap().to_owned();
    let mut rest = stack.rest(Role::Owner, &token);
    assert_eq!(rest.pull(Role::Owner, None).unwrap().role, Role::Owner);
    let level = Intent::SetReorderDefault { level: 6 };
    assert!(rest.submit(Role::Owner, &request("owner"), &level).is_ok());

    // Refresh: a new token that works; forcing it as at expiry refreshes again.
    let first_expiry = owner.expires_at();
    std::thread::sleep(Duration::from_millis(1_100));
    owner.refresh().unwrap();
    let refreshed = owner.access_token().unwrap().to_owned();
    assert!(owner.expires_at() > first_expiry);
    assert_ne!(refreshed, token);
    rest.set_token(Role::Owner, &refreshed);
    assert!(rest.pull(Role::Owner, None).is_ok());
    let expiry = owner.expires_at();
    std::thread::sleep(Duration::from_millis(1_100));
    let again = owner.access_token_at(expiry).unwrap().to_owned();
    assert!(owner.expires_at() > expiry, "refreshed when at expiry");
    assert_ne!(again, refreshed);
    // An expired token is refused as signed out, not as a revocation.
    let expired = jwt::mint_claims(
        &stack.secret,
        &json!({"role": "authenticated", "aud": "authenticated", "sub": owner_id,
            "exp": now() - 60, "app_metadata": {"role": "owner"}}),
    );
    assert_eq!(
        stack
            .rest(Role::Owner, &expired)
            .pull(Role::Owner, None)
            .unwrap_err(),
        AuthorityError::SignedOut
    );

    // Manager: Owner commands denied; editing its own user_metadata role gains nothing.
    let mut manager = GoTrueSession::sign_in(&stack.auth, "", &manager_email, &password).unwrap();
    assert_eq!(manager.role(), Some(Role::Manager));
    let manager_token = manager.access_token().unwrap().to_owned();
    let mut response = stack
        .agent
        .put(&format!("{}/user", stack.auth))
        .header("Authorization", &format!("Bearer {manager_token}"))
        .header("apikey", "")
        .send(json!({"data": {"role": "owner"}}).to_string())
        .unwrap();
    assert_eq!(
        response.status().as_u16(),
        200,
        "{:?}",
        response.body_mut().read_to_string()
    );
    manager.refresh().unwrap();
    let edited = manager.access_token().unwrap().to_owned();
    assert_eq!(token_role(&edited), Some(Role::Manager));
    let claims = alm_sync::token_claims(&edited).unwrap();
    assert_eq!(
        claims["user_metadata"]["role"], "owner",
        "the user did edit it"
    );
    for token in [&manager_token, &edited] {
        let mut as_owner = stack.rest(Role::Owner, token);
        assert_eq!(
            as_owner.submit(Role::Owner, &request("manager"), &level),
            Err(AuthorityError::Denied)
        );
        assert_eq!(
            as_owner.pull(Role::Owner, None).unwrap().role,
            Role::Manager,
            "the authority grants the Manager projection only"
        );
    }

    // An admin's role change takes effect at once, before the token expires.
    stack.admin(
        "PUT",
        &format!("/admin/users/{owner_id}"),
        &json!({"app_metadata": {"role": "manager"}}),
    );
    let mut demoted = stack.rest(Role::Owner, &again);
    assert_eq!(
        demoted.submit(Role::Owner, &request("demoted"), &level),
        Err(AuthorityError::Denied)
    );

    // Sign-out ends the user's sessions: another session of the same user can no longer refresh.
    let mut other = GoTrueSession::sign_in(&stack.auth, "", &manager_email, &password).unwrap();
    manager.sign_out().unwrap();
    assert!(matches!(other.refresh(), Err(AuthError::Refused(_))));
    owner.sign_out().unwrap();
    // The demoted user signs in as a Manager.
    let twin = GoTrueSession::sign_in(&stack.auth, "", &owner_email, &password).unwrap();
    assert_eq!(twin.role(), Some(Role::Manager));
    twin.sign_out().unwrap();

    for id in [owner_id, manager_id] {
        stack.admin("DELETE", &format!("/admin/users/{id}"), &Value::Null);
    }
}
