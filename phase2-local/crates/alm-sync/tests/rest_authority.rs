//! Opt-in: the conformance suite and the Manager checks through the REST transport against a
//! running PostgREST over the migrated local schema. Set `ALM_REST_URL` (the PostgREST root),
//! `ALM_PG_URL` (to reset and seed the database before each case) and either `ALM_JWT_OWNER`
//! and `ALM_JWT_MANAGER` or `ALM_JWT_SECRET` (the tests then mint both tokens themselves);
//! `ALM_REST_APIKEY` is sent as the `apikey` header when set. Without them every test returns
//! early. The cases share one database, so they run one at a time.
#![forbid(unsafe_code)]

mod conformance;
#[path = "rest/jwt.rs"]
mod jwt;
mod pg_authority;

use std::sync::{Mutex, MutexGuard};

use alm_core::Role;
use alm_core::confirmation::demo_fixture::{directory, fixture, seed_purchasing};
use alm_core::projection::ProjectionBatch;
use alm_sync::{
    AuthorityError, Intent, LocalAuthority, Lookup, Outcome, RequestId, RestAuthority,
    SalesAuthority,
};
use postgres::{Client, NoTls};
use serde_json::json;

static SHARED_DATABASE: Mutex<()> = Mutex::new(());

/// The REST authority holding the database for one case.
pub struct Rest {
    inner: RestAuthority,
    _database: MutexGuard<'static, ()>,
}

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.is_empty())
}

fn tokens() -> Option<(String, String)> {
    match (env("ALM_JWT_OWNER"), env("ALM_JWT_MANAGER")) {
        (Some(owner), Some(manager)) => Some((owner, manager)),
        _ => env("ALM_JWT_SECRET").map(|s| (jwt::mint(&s, "owner"), jwt::mint(&s, "manager"))),
    }
}

/// Resets the database to the seeded shop and connects as both roles.
fn rest() -> Option<Rest> {
    let (url, pg) = (env("ALM_REST_URL")?, env("ALM_PG_URL")?);
    let (owner, manager) = tokens()?;
    let guard = SHARED_DATABASE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut client = Client::connect(&pg, NoTls).expect("connect to the test database");
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
        .expect("reset");
    let mut shop = fixture().unwrap();
    seed_purchasing(&mut shop).unwrap();
    pg_authority::seed_shop(&mut client, &LocalAuthority::new(shop, directory()));
    let inner = RestAuthority::new(&url, &env("ALM_REST_APIKEY").unwrap_or_default())
        .with_token(Role::Owner, &owner)
        .with_token(Role::Manager, &manager);
    Some(Rest {
        inner,
        _database: guard,
    })
}

impl SalesAuthority for Rest {
    fn submit(
        &mut self,
        role: Role,
        request: &RequestId,
        intent: &Intent,
    ) -> Result<Outcome, AuthorityError> {
        self.inner.submit(role, request, intent)
    }

    fn status(&self, role: Role, request: &RequestId) -> Result<Option<Outcome>, AuthorityError> {
        self.inner.status(role, request)
    }

    fn lookup(
        &self,
        role: Role,
        request: &RequestId,
        intent: &Intent,
    ) -> Result<Lookup, AuthorityError> {
        self.inner.lookup(role, request, intent)
    }

    fn allocate_gst_number(&mut self, role: Role, bill_id: &str) -> Result<u64, AuthorityError> {
        self.inner.allocate_gst_number(role, bill_id)
    }

    fn pull(&self, role: Role, checkpoint: Option<u64>) -> Result<ProjectionBatch, AuthorityError> {
        self.inner.pull(role, checkpoint)
    }
}

conformance_tests!(rest());

/// Through REST a Manager is denied every command, and its pull response (the raw body) holds
/// only the catalog: no cost, buy-rate, FIFO, margin, profit, quote, stock or balance field.
#[test]
fn manager_is_denied_and_receives_no_owner_fields() {
    let Some(mut a) = rest() else { return };
    let w = conformance::world(&a);
    let sale = conformance::sale(&w, 161, 1, w.sell);
    let request = RequestId::new("rest-manager-1").unwrap();
    assert_eq!(
        a.submit(Role::Manager, &request, &sale),
        Err(AuthorityError::Denied)
    );
    assert_eq!(
        a.lookup(Role::Manager, &request, &sale),
        Err(AuthorityError::Denied)
    );
    a.submit(Role::Owner, &RequestId::new("rest-owner-1").unwrap(), &sale)
        .unwrap();
    let body = a
        .inner
        .rpc(
            Role::Manager,
            "pull_projection",
            &json!({"p_checkpoint": null}),
        )
        .unwrap();
    let text = body.to_string().to_lowercase();
    for word in [
        "buy",
        "cost",
        "fifo",
        "margin",
        "profit",
        "lot",
        "stock_value",
        "supplier",
        "purchase",
        "\"stock\"",
        "on_hand",
        "balance",
        "dev_state",
        "book",
        "allocations",
        "7000",
        "3800",
    ] {
        assert!(
            !text.contains(word),
            "a Manager response carries {word}: {text}"
        );
    }
    assert_eq!(
        body["rows"].as_object().unwrap().keys().collect::<Vec<_>>(),
        ["catalog"]
    );
    let owner = a
        .inner
        .rpc(
            Role::Owner,
            "pull_projection",
            &json!({"p_checkpoint": null}),
        )
        .unwrap();
    assert!(owner["rows"]["owner"]["sale_costs"].is_array());
    // A token without a role claim is refused like a revoked account.
    let anonymous = RestAuthority::new(&env("ALM_REST_URL").unwrap(), "").with_token(
        Role::Owner,
        &jwt::mint(&env("ALM_JWT_SECRET").unwrap_or_default(), ""),
    );
    if env("ALM_JWT_SECRET").is_some() {
        assert_eq!(
            anonymous.pull(Role::Owner, None),
            Err(AuthorityError::Denied)
        );
    }
}

/// Leaves the database reset and seeded with the app's shop, for the app's opt-in REST end to
/// end test. Runs only with `ALM_RESET_ONLY` set.
#[test]
fn reset_and_seed_for_the_app() {
    if env("ALM_RESET_ONLY").is_some() {
        assert!(
            rest().is_some(),
            "ALM_REST_URL, ALM_PG_URL and tokens are needed"
        );
    }
}
