//! The development authority simulation against the `SalesAuthority` contract.
use std::num::NonZeroU32;

use alm_core::confirmation::demo_fixture::{customer_id, directory, fixture};
use alm_core::confirmation::owner_billing::priced_draft;
use alm_core::{
    BillDate, DraftCustomer, DraftId, DraftLine, Money, ProductId, ProvisionalDraft, Role,
};
use alm_sync::{
    AuthorityError, Intent, LocalAuthority, Outcome, RequestId, SaleIntent, SalesAuthority,
};

fn day() -> BillDate {
    BillDate::new(2025, 4, 30).unwrap()
}

fn draft(id: u64, qty: u32) -> ProvisionalDraft {
    ProvisionalDraft::new(
        DraftId::new(id),
        vec![DraftLine::new(
            ProductId::new(1),
            NonZeroU32::new(qty).unwrap(),
        )],
    )
    .with_customer(DraftCustomer::Selected {
        id: customer_id(),
        name: "Synthetic Customer".into(),
    })
    .with_bill_date(Some(day()))
}

fn sale(authority: &LocalAuthority, id: u64, qty: u32) -> SaleIntent {
    let draft = draft(id, qty);
    let prices = priced_draft(authority.fixture(), Role::Owner, &draft)
        .unwrap()
        .lines
        .iter()
        .map(|line| line.unit_price)
        .collect();
    SaleIntent {
        draft,
        date: day(),
        paid: Money::from_minor_units(2_000),
        prices,
    }
}

fn request(text: &str) -> RequestId {
    RequestId::new(text).unwrap()
}

fn seeded() -> LocalAuthority {
    LocalAuthority::new(fixture().unwrap(), directory()).with_clock(|| 1_700_000_000_000)
}

#[test]
fn identical_retry_returns_the_original_receipt_and_reuse_conflicts() {
    let mut authority = seeded();
    let intent = sale(&authority, 1, 2);
    let first = authority
        .post_sale(Role::Owner, &request("trial-1"), &intent)
        .unwrap();
    assert_eq!((first.invoice, first.total.minor_units()), (1, 24_100));
    let checkpoint = authority.checkpoint();
    let again = authority
        .post_sale(Role::Owner, &request("trial-1"), &intent)
        .unwrap();
    assert_eq!(again, first);
    assert_eq!(authority.checkpoint(), checkpoint, "no second effect");
    let other = sale(&authority, 1, 3);
    assert_eq!(
        authority.post_sale(Role::Owner, &request("trial-1"), &other),
        Err(AuthorityError::Conflict)
    );
    assert_eq!(
        authority.post_sale(Role::Owner, &request("trial-2"), &intent),
        Err(AuthorityError::AlreadyPosted)
    );
    assert_eq!(
        authority.post_sale(Role::Manager, &request("trial-3"), &sale(&authority, 2, 1)),
        Err(AuthorityError::Denied)
    );
}

#[test]
fn stale_terms_and_failures_leave_no_effect() {
    let mut authority = seeded();
    let mut intent = sale(&authority, 1, 2);
    intent.prices = vec![Money::from_minor_units(1)];
    assert_eq!(
        authority.post_sale(Role::Owner, &request("trial-1"), &intent),
        Err(AuthorityError::StaleReview)
    );
    let too_many = sale(&authority, 1, 500);
    assert!(matches!(
        authority.post_sale(Role::Owner, &request("trial-1"), &too_many),
        Err(AuthorityError::Rejected(_))
    ));
    assert_eq!(authority.checkpoint(), 0);
    assert_eq!(authority.status(Role::Owner, &request("trial-1")), Ok(None));
    let ok = sale(&authority, 1, 2);
    assert!(
        authority
            .post_sale(Role::Owner, &request("trial-1"), &ok)
            .is_ok()
    );
}

#[test]
fn lost_response_commits_once_and_status_or_retry_resolves_it() {
    let mut authority = seeded();
    let intent = sale(&authority, 1, 2);
    authority.lose_next_response();
    assert_eq!(
        authority.post_sale(Role::Owner, &request("trial-1"), &intent),
        Err(AuthorityError::Unavailable)
    );
    let status = authority.status(Role::Owner, &request("trial-1")).unwrap();
    let Some(Outcome::Sale(receipt)) = status else {
        panic!("committed")
    };
    assert_eq!(
        authority.post_sale(Role::Owner, &request("trial-1"), &intent),
        Ok(receipt)
    );
    assert_eq!(authority.checkpoint(), 1);
    assert_eq!(
        authority.status(Role::Manager, &request("trial-1")),
        Err(AuthorityError::Denied)
    );
}

#[test]
fn journal_replay_restores_state_requests_and_gst_numbers() {
    let dir = std::env::temp_dir().join(format!("alm-sync-journal-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("journal.txt");
    let _ = std::fs::remove_file(&path);
    let mut authority = LocalAuthority::open(&path, fixture().unwrap(), directory()).unwrap();
    let intent = sale(&authority, 1, 2);
    let receipt = authority
        .post_sale(Role::Owner, &request("trial-1"), &intent)
        .unwrap();
    authority
        .record_receipt(
            Role::Owner,
            &request("trial-2"),
            "batch1-customer",
            day(),
            Money::from_minor_units(1_000),
            true,
        )
        .unwrap();
    authority
        .set_cancelled(
            Role::Owner,
            &request("trial-3"),
            alm_core::DraftId::new(1),
            true,
        )
        .unwrap();
    authority
        .set_cancelled(
            Role::Owner,
            &request("trial-4"),
            alm_core::DraftId::new(1),
            false,
        )
        .unwrap();
    let edited = authority
        .edit_sale(
            Role::Owner,
            &request("trial-5"),
            alm_core::DraftId::new(1),
            vec![(3, 11_800)],
        )
        .unwrap();
    assert_eq!(edited.total.minor_units(), 35_400);
    let gst = authority
        .allocate_gst_number(Role::Owner, "bill-a")
        .unwrap();
    assert_eq!(
        authority.allocate_gst_number(Role::Owner, "bill-a"),
        Ok(gst)
    );
    assert_eq!(
        authority.allocate_gst_number(Role::Owner, "bill-b"),
        Ok(gst + 1)
    );
    assert_eq!(
        authority.allocate_gst_number(Role::Manager, "bill-c"),
        Err(AuthorityError::Denied)
    );
    let before = authority.pull(Role::Owner, None).unwrap();
    drop(authority);

    let mut reopened = LocalAuthority::open(&path, fixture().unwrap(), directory()).unwrap();
    let after = reopened.pull(Role::Owner, None).unwrap();
    assert_eq!(after.checkpoint, before.checkpoint);
    assert_eq!(after.snapshot, before.snapshot);
    assert_eq!(
        reopened.post_sale(Role::Owner, &request("trial-1"), &intent),
        Ok(receipt)
    );
    assert_eq!(
        reopened.allocate_gst_number(Role::Owner, "bill-b"),
        Ok(gst + 1)
    );
    assert_eq!(
        reopened
            .pull(Role::Owner, Some(after.checkpoint))
            .unwrap()
            .snapshot,
        None
    );
    std::fs::write(&path, "R\ttrial-9\tcancel draft=99\n").unwrap();
    assert!(LocalAuthority::open(&path, fixture().unwrap(), directory()).is_err());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn the_manager_pull_holds_only_the_catalog() {
    let mut authority = seeded();
    let intent = sale(&authority, 1, 2);
    authority
        .post_sale(Role::Owner, &request("trial-1"), &intent)
        .unwrap();
    let owner = authority.pull(Role::Owner, None).unwrap().snapshot.unwrap();
    assert_eq!(owner.sales.len(), 1);
    assert!(owner.owner.is_some());
    let manager = authority.pull(Role::Manager, None).unwrap();
    assert_eq!(manager.role, Role::Manager);
    let manager = manager.snapshot.unwrap();
    assert!(manager.owner.is_none());
    assert!(manager.sales.is_empty() && manager.customers.is_empty() && manager.stock.is_empty());
    assert_eq!(manager.catalog.len(), 2);
    let _ = Intent::CancelSale {
        draft: alm_core::DraftId::new(1),
    };
}

/// The simulation's pull carries the `pull_projection` rows: the same sections, and only the
/// catalog for a Manager; the Owner's other screens are computed from its `book` rows.
#[test]
fn pull_uses_the_hosted_row_format() {
    let authority = LocalAuthority::new(fixture().unwrap(), directory());
    let keys = |role: Role| {
        let batch = authority.pull(role, None).unwrap();
        let wire = alm_sync::wire::encode_batch(&batch);
        assert_eq!(alm_sync::wire::decode_batch(&wire, role), Ok(batch));
        let mut keys: Vec<String> = wire["rows"]
            .as_object()
            .unwrap()
            .iter()
            .filter(|(_, v)| !v.is_null() && v.as_array().is_none_or(|a| !a.is_empty()))
            .map(|(k, _)| k.clone())
            .collect();
        keys.sort();
        (keys, wire)
    };
    let (manager, wire) = keys(Role::Manager);
    assert_eq!(manager, ["catalog"]);
    assert!(!wire.to_string().contains("dev_state"));
    let (owner, wire) = keys(Role::Owner);
    assert_eq!(
        owner,
        ["catalog", "customers", "owner", "products", "stock"]
    );
    let owner_keys: Vec<&String> = wire["rows"]["owner"].as_object().unwrap().keys().collect();
    assert!(owner_keys.iter().any(|k| *k == "book"));
    assert!(!wire.to_string().contains("dev_state"));
}

/// The reconciliation report for each synthetic legacy file, against what the authority holds
/// after the import (read back through a pull): every difference is explained by a quarantined
/// record except the numbering, where the authority's live series would reissue legacy numbers
/// (OPEN-003); the report computed before the import (dry run) is the same.
#[test]
fn legacy_imports_reconcile_against_the_authority_rows() {
    use alm_core::legacy::{
        LegacyBackup, LineStatus, MigrateClock, dry_run, live_numbers, reconcile,
    };
    let clock = MigrateClock {
        now_ms: 1_767_900_000_000,
        today: "2026-01-08",
    };
    for (file, bytes, quarantined, unexplained) in [
        (
            "rehearsal-a",
            &include_bytes!("../../alm-core/tests/legacy/rehearsal-a.json")[..],
            4,
            2,
        ),
        (
            "rehearsal-b",
            &include_bytes!("../../alm-core/tests/legacy/rehearsal-b.json")[..],
            2,
            1,
        ),
    ] {
        let mut authority = seeded();
        let backup = LegacyBackup::parse(bytes, clock).unwrap();
        let book = |a: &LocalAuthority| {
            a.pull(Role::Owner, None)
                .unwrap()
                .snapshot
                .unwrap()
                .owner
                .unwrap()
                .book
        };
        let before = book(&authority);
        let dry = dry_run(&backup, &before.legacy, live_numbers(&before));
        let mut plan = alm_core::legacy::import::plan(&backup);
        plan.source = String::from_utf8(bytes.to_vec()).unwrap();
        let Ok(Outcome::Created(id)) = authority.submit(
            Role::Owner,
            &request(&format!("legacy-{file}")),
            &Intent::ImportLegacy(Box::new(plan)),
        ) else {
            panic!("{file}: imported")
        };
        let after = book(&authority);
        assert_eq!(
            after.settings, before.settings,
            "{file}: no live series moved"
        );
        assert_eq!(after.sales, before.sales, "{file}: nothing posted");
        let report = reconcile(&backup, &after.legacy, id, live_numbers(&after)).unwrap();
        assert_eq!(report, dry, "{file}: the dry run predicted the import");
        assert_eq!(report.quarantined.len(), quarantined, "{file}");
        let open: Vec<_> = report
            .sections
            .iter()
            .flat_map(|s| s.lines.iter().map(move |l| (s.name, l)))
            .filter(|(_, l)| matches!(l.status, LineStatus::Unexplained(_)))
            .map(|(s, l)| (s, l.key.clone()))
            .collect();
        assert_eq!(open.len(), unexplained, "{file}: {open:?}");
        assert!(
            open.iter()
                .all(|(s, _)| *s == "Numbering (shown, not issued)"),
            "{file}: {open:?}"
        );
    }
}
