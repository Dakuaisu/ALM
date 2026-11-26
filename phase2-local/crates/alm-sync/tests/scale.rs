//! Scale (development evidence, synthetic data only): a generated legacy file of shop size
//! (`alm-core/tests/legacy/scale.rs`: 20,000 bills incl. mixed and crate bills, 1,000 customers,
//! 500 products, 2,000 purchases, 300 GST bills) is parsed, planned, imported into the
//! LocalAuthority (and, with `ALM_PG_URL`, into Postgres), reconciled and pulled in full; with
//! `ALM_SCALE_ONE_TRANSACTION` too, 20,000 bills are posted in one Postgres transaction; with
//! PostgREST, 20,000 bills are posted and pulled through REST. Each step's time is printed and
//! must stay within 5x the time recorded on the development machine (`BASELINE_MS`, debug
//! build); set `ALM_SKIP_PERF` to skip that assertion.
#![forbid(unsafe_code)]

#[path = "rest/jwt.rs"]
mod jwt;
mod pg_authority;
#[path = "../../alm-core/tests/legacy/scale.rs"]
mod scale;
#[path = "scale/shop.rs"]
mod shop;

use std::time::Instant;

use alm_core::Role;
use alm_core::confirmation::demo_fixture::{directory, fixture, seed_purchasing};
use alm_core::legacy::import::plan_file;
use alm_core::legacy::{LegacyBackup, live_numbers, reconcile};
use alm_sync::{Intent, LocalAuthority, Outcome, RequestId, SalesAuthority};

/// Milliseconds recorded on the development machine (debug build), per step.
const BASELINE_MS: &[(&str, u128)] = &[
    ("generate", 62),
    ("parse", 2_077),
    ("plan", 3_288),
    ("import local", 7_305),
    ("reconcile", 1_360),
    ("pull local", 2_398),
    ("import postgres", 4_171),
    ("pull postgres", 2_794),
    ("post 20000 bills postgres", 758_269),
    ("pull 20000 bills postgres", 14_320),
    ("post 20000 bills rest", 60_859),
    ("pull 20000 bills rest", 7_070),
    ("pull changes after one bill rest", 10_281),
    ("apply changes on the device", 1_520),
];

fn check(step: &str, started: Instant, times: &mut Vec<(String, u128)>) {
    let ms = started.elapsed().as_millis();
    eprintln!("scale: {step} {ms} ms");
    times.push((step.to_owned(), ms));
    if std::env::var_os("ALM_SKIP_PERF").is_some() {
        return;
    }
    let base = BASELINE_MS
        .iter()
        .find(|(s, _)| *s == step)
        .map_or(0, |(_, ms)| *ms);
    assert!(
        ms <= (base * 5).max(50),
        "{step}: {ms} ms is over 5x the recorded {base} ms"
    );
}

#[test]
fn a_shop_sized_legacy_file_imports_reconciles_and_pulls_within_its_recorded_times() {
    let mut times = Vec::new();
    let started = Instant::now();
    let file = scale::legacy_file(&scale::SHOP);
    check("generate", started, &mut times);

    let started = Instant::now();
    let backup = LegacyBackup::parse_for_import(file.as_bytes()).unwrap();
    check("parse", started, &mut times);
    assert_eq!(backup.collection("sales").len(), scale::SHOP.bills);

    let started = Instant::now();
    let (_, plan) = plan_file(file.as_bytes()).unwrap();
    check("plan", started, &mut times);
    assert!(plan.records.len() > scale::SHOP.bills);

    let mut shop = fixture().unwrap();
    seed_purchasing(&mut shop).unwrap();
    let mut local = LocalAuthority::new(shop, directory());
    let intent = Intent::ImportLegacy(Box::new(plan));
    let request = RequestId::new("scale-import").unwrap();
    let started = Instant::now();
    let outcome = local.submit(Role::Owner, &request, &intent).unwrap();
    check("import local", started, &mut times);
    assert_eq!(outcome, Outcome::Created(1));

    let book = local
        .pull(Role::Owner, None)
        .unwrap()
        .snapshot
        .unwrap()
        .owner
        .unwrap()
        .book;
    let started = Instant::now();
    let report = reconcile(&backup, &book.legacy, 1, live_numbers(&book)).unwrap();
    check("reconcile", started, &mut times);
    assert_eq!(report.imported, book.legacy.records.len());

    let started = Instant::now();
    let batch = local.pull(Role::Owner, None).unwrap();
    check("pull local", started, &mut times);
    assert!(batch.snapshot.is_some());

    if let Some(pg) = pg_authority::PgAuthority::connect(&local) {
        let mut pg = pg;
        let started = Instant::now();
        assert_eq!(
            pg.submit(Role::Owner, &request, &intent).unwrap(),
            Outcome::Created(1)
        );
        check("import postgres", started, &mut times);
        let started = Instant::now();
        let pulled = pg.pull(Role::Owner, None).unwrap();
        check("pull postgres", started, &mut times);
        let legacy = pulled.snapshot.unwrap().owner.unwrap().book.legacy;
        assert_eq!(legacy, book.legacy, "the same rows as the core");
    }
}

/// Opt-in (`ALM_PG_URL` and `ALM_SCALE_ONE_TRANSACTION`, 13 to 21 minutes): 20,000 bills posted
/// through the dev Postgres authority, all in its one test transaction, which ran out of the
/// shared lock table (53200) on a default postgres:17 before the request and draft locks were
/// bounded; then pulled in full.
#[test]
fn twenty_thousand_bills_post_in_one_postgres_transaction_within_their_recorded_times() {
    if std::env::var_os("ALM_SCALE_ONE_TRANSACTION").is_none() {
        return;
    }
    let (shop, book) = shop::shop();
    let local = LocalAuthority::new(shop, book.clone());
    let Some(mut pg) = pg_authority::PgAuthority::connect(&local) else {
        return;
    };
    let snapshot = local.pull(Role::Owner, None).unwrap().snapshot.unwrap();
    let mut times = Vec::new();
    let started = Instant::now();
    for bill in 1..=shop::BILLS {
        let request = RequestId::new(&format!("scale-pg-{bill}")).unwrap();
        pg.submit(Role::Owner, &request, &shop::bill(bill, &book, &snapshot))
            .unwrap();
    }
    check("post 20000 bills postgres", started, &mut times);
    let started = Instant::now();
    let pulled = pg.pull(Role::Owner, None).unwrap().snapshot.unwrap();
    check("pull 20000 bills postgres", started, &mut times);
    assert_eq!(pulled.sales.len(), usize::try_from(shop::BILLS).unwrap());
}

/// Opt-in (`ALM_PG_URL`, `ALM_REST_URL`, `ALM_JWT_SECRET`): the database is reset and seeded
/// with the shop, then 20,000 bills (one in ten mixed) are posted through PostgREST, one
/// transaction per call as a device sends them, and pulled in full (Sales list, owner book and
/// the crate rows of every bill). Runs alone against the shared database.
#[test]
fn twenty_thousand_bills_post_and_pull_through_rest_within_their_recorded_times() {
    let (Ok(url), Ok(rest_url), Ok(secret)) = (
        std::env::var("ALM_PG_URL"),
        std::env::var("ALM_REST_URL"),
        std::env::var("ALM_JWT_SECRET"),
    ) else {
        return;
    };
    let (shop, book) = shop::shop();
    let local = LocalAuthority::new(shop, book.clone());
    let mut client = postgres::Client::connect(&url, postgres::NoTls).unwrap();
    reset(&mut client);
    pg_authority::seed_shop(&mut client, &local);
    let mut rest = alm_sync::RestAuthority::new(&rest_url, "")
        .with_token(Role::Owner, &jwt::mint(&secret, "owner"));
    let snapshot = local.pull(Role::Owner, None).unwrap().snapshot.unwrap();
    let mut times = Vec::new();
    let started = Instant::now();
    for bill in 1..=shop::BILLS {
        let request = RequestId::new(&format!("scale-rest-{bill}")).unwrap();
        rest.submit(Role::Owner, &request, &shop::bill(bill, &book, &snapshot))
            .unwrap();
    }
    check("post 20000 bills rest", started, &mut times);
    let started = Instant::now();
    let full = rest.pull(Role::Owner, None).unwrap();
    check("pull 20000 bills rest", started, &mut times);
    let pulled = full.snapshot.clone().unwrap();
    assert_eq!(pulled.sales.len(), usize::try_from(shop::BILLS).unwrap());
    assert_eq!(
        pulled.sales.iter().filter(|s| s.lines == 2).count(),
        usize::try_from(shop::BILLS / 10).unwrap()
    );

    // One more bill, then the device's pull from its checkpoint: changes only.
    use alm_core::projection::PulledRows;
    let full_bytes = match &full.rows {
        Some(PulledRows::Full { rows, hashes }) => {
            rows.to_string().len() + hashes.to_string().len()
        }
        other => panic!("a full pull: {other:?}"),
    };
    let held = alm_sync::delta::device_view(None, &full)
        .unwrap()
        .unwrap()
        .1;
    let manager = alm_sync::RestAuthority::new(&rest_url, "")
        .with_token(Role::Manager, &jwt::mint(&secret, "manager"));
    let manager_full = manager.pull(Role::Manager, None).unwrap();
    let request = RequestId::new("scale-rest-one-more").unwrap();
    rest.submit(
        Role::Owner,
        &request,
        &shop::bill(shop::BILLS + 1, &book, &snapshot),
    )
    .unwrap();
    let started = Instant::now();
    let changes = rest.pull(Role::Owner, Some(full.checkpoint)).unwrap();
    check("pull changes after one bill rest", started, &mut times);
    let Some(PulledRows::Changes(delta)) = &changes.rows else {
        panic!("changes only: {:?}", changes.rows.is_some())
    };
    let delta_bytes = delta.to_string().len();
    let started = Instant::now();
    let (merged, _, all) = alm_sync::delta::device_view(Some(&held), &changes)
        .unwrap()
        .unwrap();
    check("apply changes on the device", started, &mut times);
    assert!(!all);
    eprintln!(
        "scale: full pull {full_bytes} bytes, changes after one bill {delta_bytes} bytes \
         ({} rows added)",
        delta["added"].as_object().map_or(0, serde_json::Map::len)
    );
    assert!(
        delta_bytes * 10 < full_bytes,
        "{delta_bytes} vs {full_bytes}"
    );
    let fresh = rest.pull(Role::Owner, None).unwrap().snapshot.unwrap();
    assert_eq!(merged, fresh, "the device's rows equal a full pull");
    // A Manager's pull from its checkpoint carries no costs.
    let manager_changes = manager
        .pull(Role::Manager, Some(manager_full.checkpoint))
        .unwrap();
    if let Some(PulledRows::Changes(delta)) = &manager_changes.rows {
        let text = delta.to_string();
        assert!(
            !text.contains("\"owner\"") && !text.contains("cost"),
            "{text}"
        );
    }
    assert!(manager_changes.snapshot.is_none_or(|s| s.owner.is_none()));
    reset(&mut client);
}

/// Empties the public tables to the freshly migrated state the other opt-in suites expect.
fn reset(client: &mut postgres::Client) {
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
}
