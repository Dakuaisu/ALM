//! DEC-034: the SQL matrix in the migrations (and, with `ALM_PG_URL`, in the live schema) is the
//! core's `alm_core::permissions::sql_functions()`, and every `SalesAuthority` command, walked
//! for both roles against the simulation, is allowed or denied exactly as the matrix says.
#![forbid(unsafe_code)]

use std::num::NonZeroU32;

use alm_core::confirmation::demo_fixture::{directory, fixture, seed_purchasing};
use alm_core::confirmation::manual_cashbook::CashBookAmounts;
use alm_core::permissions::{self, Capability};
use alm_core::{
    BillDate, DraftCustomer, DraftId, DraftLine, Money, ProductId, ProvisionalDraft, Role,
};
use alm_sync::{
    AuthorityError, CrateSale, CrateTerms, Intent, LocalAuthority, ProductTerms, PurchaseLineTerms,
    PurchaseOrderTerms, RequestId, SaleIntent, SalesAuthority,
};

const CLOCK_MS: u64 = 1_700_000_000_000;

/// The `(function, manager)` rows the migrations insert into `alm_private.permissions`, in
/// order (a later row for a function replaces an earlier one).
fn migration_rows() -> Vec<(String, bool)> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../supabase/migrations");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|e| e == "sql"))
        .collect();
    files.sort();
    let mut rows: Vec<(String, bool)> = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(&file).unwrap();
        for statement in text.split(';') {
            let Some(at) = statement.find("insert into alm_private.permissions") else {
                continue;
            };
            let values = &statement[at..];
            for tuple in values.split('(').skip(2) {
                let mut parts = tuple.split(')').next().unwrap().split(',');
                let name = parts.next().unwrap().trim().trim_matches('\'').to_owned();
                let manager = match parts.next().unwrap().trim() {
                    "true" => true,
                    "false" => false,
                    other => panic!("{}: {other}", file.display()),
                };
                rows.retain(|(n, _)| *n != name);
                rows.push((name, manager));
            }
        }
    }
    rows.sort();
    rows
}

fn core_rows() -> Vec<(String, bool)> {
    permissions::sql_functions()
        .into_iter()
        .map(|(name, manager)| (name.to_owned(), manager))
        .collect()
}

#[test]
fn the_sql_matrix_in_the_migrations_is_the_core_table() {
    let rows = migration_rows();
    assert!(rows.len() >= 30, "{rows:?}");
    assert_eq!(rows, core_rows());
}

#[test]
fn the_live_schema_holds_the_core_matrix() {
    let Ok(url) = std::env::var("ALM_PG_URL") else {
        return;
    };
    let mut client = postgres::Client::connect(&url, postgres::NoTls).unwrap();
    let rows: Vec<(String, bool)> = client
        .query(
            "SELECT function_name, manager FROM alm_private.permissions ORDER BY 1",
            &[],
        )
        .unwrap()
        .iter()
        .map(|row| (row.get(0), row.get(1)))
        .collect();
    assert_eq!(rows, core_rows());
}

fn day() -> BillDate {
    BillDate::new(2025, 4, 30).unwrap()
}

fn today() -> BillDate {
    alm_core::projection::shop_today(CLOCK_MS).unwrap()
}

fn money(paise: i64) -> Money {
    Money::from_minor_units(paise)
}

fn authority() -> LocalAuthority {
    let mut shop = fixture().unwrap();
    seed_purchasing(&mut shop).unwrap();
    let mut authority = LocalAuthority::new(shop, directory()).with_clock(|| CLOCK_MS);
    authority.set_role_caller(Role::Owner, "matrix-device", "matrix-owner");
    authority.set_role_caller(Role::Manager, "matrix-device", "matrix-manager");
    authority
}

/// One intent of every kind; each is sent to a fresh authority, so none depends on another.
fn every_intent() -> Vec<Intent> {
    let customer = DraftCustomer::Selected {
        id: alm_core::confirmation::demo_fixture::customer_id(),
        name: alm_core::confirmation::demo_fixture::CUSTOMER.into(),
    };
    let draft = ProvisionalDraft::new(
        DraftId::new(1),
        vec![DraftLine::new(
            ProductId::new(1),
            NonZeroU32::new(1).unwrap(),
        )],
    )
    .with_customer(customer)
    .with_bill_date(Some(day()));
    let amounts = CashBookAmounts {
        cash: money(2_500),
        ..CashBookAmounts::ZERO
    };
    let plan: alm_core::legacy::import::ImportPlan = serde_json::from_value(serde_json::json!({
        "digest": "0", "version": "x", "records": [], "quarantine": []}))
    .unwrap();
    let decision: alm_core::legacy::resolve::QuarantineDecision =
        serde_json::from_value(serde_json::json!({"manifest": 0, "entry": 0, "kind": "x",
            "legacy_id": "x", "decision": "keep_excluded", "reason": "x"}))
        .unwrap();
    vec![
        Intent::PostSale(SaleIntent {
            draft,
            date: day(),
            paid: money(2_000),
            prices: vec![money(12_050)],
        }),
        Intent::SellCrate(CrateSale {
            draft: DraftId::new(2),
            template: 0,
            quantity: 1,
            customer: ("batch1-customer".into(), "Synthetic Customer".into()),
            date: day(),
            paid: money(0),
            rate: money(1),
            components: vec![],
        }),
        Intent::CancelSale {
            draft: DraftId::new(9),
        },
        Intent::RestoreSale {
            draft: DraftId::new(9),
        },
        Intent::EditSale {
            draft: DraftId::new(9),
            lines: vec![(1, 100)],
        },
        Intent::RecordReceipt {
            customer: "batch1-customer".into(),
            date: day(),
            amount: money(100),
            online: false,
        },
        Intent::SaveCashDay {
            id: None,
            date: today(),
            amounts,
        },
        Intent::SaveCashDay {
            id: Some(0),
            date: today(),
            amounts,
        },
        Intent::DeleteCashDay { id: 0 },
        Intent::SaveProduct(ProductTerms {
            id: 90,
            new: true,
            item: "Matrix Juice".into(),
            brand: String::new(),
            buy: money(100),
            sell: money(200),
            reorder: None,
            reviewed: None,
        }),
        Intent::DeleteProduct { id: 90 },
        Intent::AddSupplier {
            name: "Matrix Traders".into(),
        },
        Intent::SetSupplierPrices {
            product: 1,
            rates: vec![(0, Some(money(6_000)))],
        },
        Intent::SavePurchaseOrder(PurchaseOrderTerms {
            supplier: Some((None, "Matrix Depot".into())),
            date: Some(day()),
            note: String::new(),
            lines: vec![PurchaseLineTerms {
                item: "Synthetic Soda".into(),
                product: Some(2),
                ordered: 5,
                rate: money(3_600),
            }],
        }),
        Intent::ReceivePurchaseOrder {
            po: 0,
            date: None,
            today: day(),
            lines: vec![(1, money(3_600))],
            remaining: vec![5],
        },
        Intent::ClosePurchaseOrder {
            po: 0,
            remaining: vec![5],
        },
        Intent::SaveCrate(CrateTerms {
            id: None,
            name: "Matrix Crate".into(),
            price: money(40_000),
            components: vec![(1, 2, false)],
        }),
        Intent::DeleteCrate { id: 0 },
        Intent::SetReorderDefault { level: 4 },
        Intent::RecordRestore,
        Intent::ImportLegacy(Box::new(plan)),
        Intent::ResolveLegacyQuarantine(Box::new(decision)),
        Intent::SaveOpening {
            customer: "batch1-customer".into(),
            amount: Some(money(100)),
            date: None,
        },
        Intent::SaveGodown {
            item: "Matrix Soda".into(),
            cases: "3".into(),
            note: String::new(),
        },
        Intent::CountStock {
            product: 1,
            counted: 49,
            fridge: None,
            date: day(),
            reason: None,
            note: None,
            on_hand: 50,
        },
        Intent::SaveGstBill { bill: "{}".into() },
        Intent::CancelGstBill { id: 1 },
        Intent::DeleteGstBill { id: 1 },
        Intent::ImportGstBill {
            bill: None,
            deleted: Some(5),
        },
    ]
}

#[test]
fn every_command_is_allowed_or_denied_for_both_roles_exactly_as_the_matrix_says() {
    let intents = every_intent();
    let mut seen = std::collections::BTreeSet::new();
    for intent in &intents {
        let capability = intent.capability();
        seen.insert(capability);
        for role in [Role::Owner, Role::Manager] {
            let mut a = authority();
            let request = RequestId::new("matrix-1").unwrap();
            let result = a.submit(role, &request, intent);
            let allowed = permissions::allows(role, capability);
            assert_eq!(
                result != Err(AuthorityError::Denied),
                allowed,
                "{role:?} {capability:?}: {result:?}"
            );
        }
    }
    // Every command capability of the matrix is walked.
    for row in permissions::MATRIX {
        let command = !row.sql.is_empty();
        let other = matches!(
            row.capability,
            Capability::AllocateGstNumber
                | Capability::RequestStatus
                | Capability::RequestLookup
                | Capability::Pull
                | Capability::SaveCustomer
                | Capability::ReceiveGoods
        );
        assert!(
            !command || other || seen.contains(&row.capability),
            "{:?} not walked",
            row.capability
        );
    }
    for role in [Role::Owner, Role::Manager] {
        let mut a = authority();
        let request = RequestId::new("matrix-2").unwrap();
        assert_eq!(
            a.allocate_gst_number(role, "gst-matrix").is_ok(),
            permissions::allows(role, Capability::AllocateGstNumber),
            "{role:?} GST number"
        );
        assert_eq!(a.status(role, &request), Ok(None), "{role:?} status");
        assert_eq!(
            a.lookup(role, &request, &intents[0]),
            Ok(alm_sync::Lookup::NotCommitted),
            "{role:?} lookup"
        );
        assert_eq!(
            a.pull(role, None).map(|b| b.role),
            Ok(role),
            "{role:?} pull"
        );
    }
}

#[test]
fn the_manager_limits_hold_inside_the_allowed_commands() {
    let amounts = CashBookAmounts {
        cash: money(2_500),
        ..CashBookAmounts::ZERO
    };
    let close = |date| Intent::SaveCashDay {
        id: None,
        date,
        amounts,
    };
    let mut a = authority();
    let request = |tag: &str| RequestId::new(&format!("limit-{tag}")).unwrap();
    assert_eq!(
        a.submit(Role::Manager, &request("old"), &close(day())),
        Err(AuthorityError::Denied),
        "an earlier day"
    );
    a.submit(Role::Manager, &request("today"), &close(today()))
        .unwrap();
    assert!(
        a.submit(Role::Manager, &request("today"), &close(today()))
            .is_ok(),
        "its own retry replays"
    );
    assert_eq!(
        a.submit(Role::Manager, &request("again"), &close(today())),
        Err(AuthorityError::Denied),
        "today once"
    );
    assert!(
        a.status(Role::Manager, &request("today"))
            .unwrap()
            .is_some()
    );
    let owner = request("owner");
    a.submit(Role::Owner, &owner, &close(day())).unwrap();
    assert_eq!(
        a.status(Role::Manager, &owner),
        Err(AuthorityError::Denied),
        "another account's request"
    );
    assert_eq!(
        a.submit(Role::Manager, &owner, &close(day())),
        Err(AuthorityError::Denied),
        "nor replay it"
    );
}
