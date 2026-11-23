//! Rendered-HTML checks for the batch 1 book screens against core values.
use alm_core::confirmation::demo_fixture::{directory, fixture};
use alm_core::confirmation::owner_billing::{CustomerFilter, ReceiptTerms, save_receipt};
use alm_core::confirmation::{ConfirmationFixture, OpeningBalance, ReceiptMode};
use alm_core::{DraftCustomerId, Money, Role};

use crate::billing_tests::{day, posted};
use crate::views::{self, CustomersState, money_r};

pub(crate) fn books() -> ConfirmationFixture {
    let mut fixture = fixture().expect("synthetic fixture");
    posted(&mut fixture);
    save_receipt(
        &mut fixture,
        &directory(),
        Role::Owner,
        "batch1-customer",
        ReceiptTerms {
            date: "2025-05-01",
            amount: "30.50",
            mode: ReceiptMode::Online,
        },
    )
    .expect("receipt");
    fixture
        .save_customer_opening(
            Role::Owner,
            &DraftCustomerId::new("batch1-market".into()),
            OpeningBalance {
                amount: Some(Money::from_minor_units(25_050)),
                date: None,
            },
        )
        .expect("opening");
    fixture
}

#[test]
fn customers_render_core_balances_totals_and_ledger() {
    let fixture = books();
    let core = directory()
        .customer_book(&fixture, Role::Owner, &CustomerFilter::All)
        .expect("book");
    assert_eq!(core.receivable.minor_units(), 7_000 + 25_050);
    let state = CustomersState {
        open: Some("batch1-customer".into()),
        ..CustomersState::default()
    };
    let html =
        views::customers_screen(&fixture, &directory(), Role::Owner, &state).expect("customers");
    assert!(html.contains(&format!(
        r#"<div class="label">Total Receivable</div><div class="val">{}</div><div class="sub">2 customers</div>"#,
        money_r(core.receivable)
    )));
    assert!(html.contains("₹321"));
    assert!(html.contains(r#"<td class="num neg">₹70</td>"#));
    assert!(html.contains(r#"<td class="num neg">₹251</td>"#));
    assert!(html.contains(
        "<td>Bill #1 (1 items)</td><td class=\"num\">₹121</td><td class=\"num pos\">₹20</td>"
    ));
    assert!(html.contains("<td>Payment (Online)</td><td class=\"num\"></td><td class=\"num pos\">₹31</td><td class=\"num neg\">₹70</td>"));
    let due = CustomersState {
        due_only: true,
        ..CustomersState::default()
    };
    let html = views::customers_screen(&fixture, &directory(), Role::Owner, &due).expect("due");
    assert_eq!(html.matches("data-action=\"customer_open\"").count(), 4);
}

#[test]
fn manager_sees_no_customer_balances() {
    let fixture = books();
    let state = CustomersState {
        open: Some("batch1-customer".into()),
        ..CustomersState::default()
    };
    let html =
        views::customers_screen(&fixture, &directory(), Role::Manager, &state).expect("denial");
    assert!(html.contains("role=\"alert\""));
    assert!(!html.contains('₹'));
    assert!(!html.contains("Synthetic Customer"));
}

#[test]
fn opening_balance_input_uses_the_core_parser_and_owner_gate() {
    let root = std::env::temp_dir().join(format!("alm-opening-{}", std::process::id()));
    std::fs::create_dir(&root).unwrap();
    let mut trial = crate::Trial::open(&root, Some(day())).unwrap();
    let terms = |amount: &str| crate::customers::Opening {
        customer: "batch1-market".into(),
        amount: amount.into(),
        date: "2025-04-01".into(),
    };
    assert!(crate::customers::save_opening(&mut trial, terms("-5")).is_err());
    trial.owner = false;
    assert!(crate::customers::save_opening(&mut trial, terms("5")).is_err());
    trial.owner = true;
    crate::customers::save_opening(&mut trial, terms("250.50")).unwrap();
    let due = trial.authority.customer_due("batch1-market");
    drop(trial);
    std::fs::remove_dir_all(root).unwrap();
    assert_eq!(due.minor_units(), 25_050);
}
