//! Scale (development evidence, synthetic data only): a shop of 500 products, 1,000 customers
//! and 20,000 posted bills (one in ten mixes a crate row with its items) is posted through the
//! LocalAuthority, pulled in full, and the Sales, Customers, Stock and Dashboard screens are
//! rendered from it. Each step's time is printed and must stay within 5x the time recorded on
//! the development machine (`BASELINE_MS`, debug build); `ALM_SKIP_PERF` skips that assertion.
use std::time::Instant;

use alm_core::Role;
use alm_sync::{LocalAuthority, RequestId, SalesAuthority};

use crate::screen_tests::Root;

#[path = "../../alm-sync/tests/scale/shop.rs"]
mod shop;

/// Milliseconds recorded on the development machine (debug build), per step.
const BASELINE_MS: &[(&str, u128)] = &[
    ("post 20000 bills", 8_217),
    ("pull", 4_995),
    ("render sales", 3_337),
    ("render customers", 2_682),
    ("render stock", 2_839),
    ("render dashboard", 6_437),
];

fn check(step: &str, started: Instant) {
    let ms = started.elapsed().as_millis();
    eprintln!("scale: {step} {ms} ms");
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
fn a_shop_of_twenty_thousand_bills_posts_pulls_and_renders_within_its_recorded_times() {
    let (shop, book) = shop::shop();
    let mut authority = LocalAuthority::new(shop, book.clone()).with_clock(|| 1_700_000_000_000);
    let snapshot = authority.pull(Role::Owner, None).unwrap().snapshot.unwrap();

    let started = Instant::now();
    for bill in 1..=shop::BILLS {
        let sale = shop::bill(bill, &book, &snapshot);
        authority
            .submit(
                Role::Owner,
                &RequestId::new(&format!("scale-{bill}")).unwrap(),
                &sale,
            )
            .unwrap();
    }
    check("post 20000 bills", started);

    let started = Instant::now();
    let batch = authority.pull(Role::Owner, None).unwrap();
    check("pull", started);
    assert_eq!(
        batch.snapshot.unwrap().sales.len(),
        usize::try_from(shop::BILLS).unwrap()
    );

    let root = Root::new("scale-render");
    let mut trial = crate::Trial::open(&root.0, Some(super::billing_tests::day())).unwrap();
    trial.authority = authority;
    trial.directory = book;
    trial.projection.clear().unwrap();
    trial.refresh().unwrap();
    for tab in ["sales", "customers", "stock", "dashboard"] {
        let started = Instant::now();
        let html = trial.render(tab).unwrap();
        check(&format!("render {tab}"), started);
        assert!(!html.contains("role=\"alert\""), "{tab} renders");
    }
}
