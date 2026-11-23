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

pub(crate) struct Root(pub std::path::PathBuf);

impl Root {
    pub(crate) fn new(label: &str) -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let serial = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("alm-{label}-{}-{serial}", std::process::id()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub(crate) fn entry(id: &str, date: &str, cash: &str, intent: &str) -> crate::cashbook::Entry {
    crate::cashbook::Entry {
        id: id.into(),
        date: date.into(),
        cash: cash.into(),
        paytm: String::new(),
        phonepe: "".into(),
        debt: "".into(),
        salary: "150".into(),
        goods: "".into(),
        misc: "0.50".into(),
        intent: intent.into(),
    }
}

#[test]
fn cash_book_renders_core_system_figures_differences_and_month_totals() {
    let root = Root::new("cashbook");
    let mut trial = crate::Trial::open(&root.0, Some(day())).unwrap();
    trial.authority = books();
    trial
        .cash_entry(entry("", "2025-04-30", "18", "check"))
        .unwrap();
    let html = trial.render("expenses").unwrap();
    assert!(
        html.contains("System total for 2025-04-30: Cash +₹20 · Online ₹0 — from 1 bills/receipts")
    );
    assert!(html.contains(r#"Cash difference: <b class="neg">−₹2</b>"#));
    trial
        .cash_entry(entry("", "2025-04-30", "18", "system"))
        .unwrap();
    let html = trial.render("expenses").unwrap();
    assert!(html.contains(r#"name="cash" inputmode="decimal" value="20.00""#));
    assert!(html.contains(r#"Cash difference: <b class="pos">₹0</b>"#));
    trial
        .cash_entry(entry("", "2025-04-30", "20.00", "save"))
        .unwrap();
    let month = trial.cashbook.month(Role::Owner, day()).unwrap();
    assert_eq!(
        (
            month.income.minor_units(),
            month.expenses.minor_units(),
            month.net.minor_units()
        ),
        (2_000, 15_050, -13_050)
    );
    let html = trial.render("expenses").unwrap();
    assert!(html.contains(&format!(
        r#"<div class="label">Net (month)</div><div class="val">{}</div>"#,
        money_r(month.net)
    )));
    assert!(html.contains("<td>2025-04-30</td><td class=\"num pos\">₹20</td>"));
    trial
        .cash_entry(entry("0", "2025-04-30", "20", "delete"))
        .unwrap();
    assert!(trial.cashbook.rows(Role::Owner).unwrap().is_empty());

    trial.owner = false;
    assert!(
        trial
            .cash_entry(entry("", "2025-04-30", "1", "save"))
            .is_err()
    );
    let html = trial.render("expenses").unwrap();
    assert!(html.contains("role=\"alert\""));
    assert!(!html.contains('₹'));
}

#[test]
fn dashboard_renders_core_window_totals_profit_receivables_and_ageing() {
    use alm_core::confirmation::{DashboardPeriod, SalesPeriod};
    let fixture = books();
    let core = fixture
        .dashboard(Role::Owner, day(), DashboardPeriod::Month)
        .expect("dashboard");
    assert_eq!(
        (
            core.current.sales.minor_units(),
            core.current.collected.minor_units()
        ),
        (12_050, 2_000)
    );
    let profit = fixture
        .sales_history(
            Role::Owner,
            day(),
            SalesPeriod::Custom((Some(core.current.window.start), Some(day()))),
            "",
        )
        .expect("history")
        .profit
        .expect("profit")
        .profit;
    assert_eq!(profit.minor_units(), 5_050);
    let ageing = fixture.customer_ageing(Role::Owner, day()).expect("ageing");
    let html = views::dashboard_screen(
        &fixture,
        &directory(),
        Role::Owner,
        day(),
        DashboardPeriod::Month,
    )
    .expect("dashboard");
    assert!(html.contains(
        r#"<div class="hl">Sales · this month</div><div class="hv"><span class="priv">₹121</span>"#
    ));
    assert!(
        html.contains(
            "1 bill · 2025-04-01 to 2025-04-30 · last month <span class=\"priv\">₹0</span>"
        )
    );
    assert!(html.contains(&format!(
        r#"<div class="sl">Profit</div><div class="sv"><span class="priv">{}</span>"#,
        money_r(profit)
    )));
    assert!(html.contains(
        r#"<div class="sl">Collected</div><div class="sv"><span class="priv">₹20</span>"#
    ));
    assert!(html.contains(r#"<div class="sl">Receivables</div><div class="sv"><span class="priv">₹321</span></div><div class="ss">2 customers owe</div>"#));
    assert!(html.contains(&format!(
        r#"<span class="al">0–15 days</span><span class="av priv">{}</span>"#,
        money_r(ageing.d15)
    )));
    assert!(html.find("Synthetic Market").unwrap() < html.find("Synthetic Customer").unwrap());

    let next = alm_core::BillDate::new(2025, 5, 1).unwrap();
    let html = views::dashboard_screen(
        &fixture,
        &directory(),
        Role::Owner,
        next,
        DashboardPeriod::Today,
    )
    .expect("today");
    assert!(html.contains("bills ₹0 · receipts ₹31"));
    assert!(html.contains(r#"<span>Online</span><b class="priv">₹31</b>"#));

    let manager = views::dashboard_screen(
        &fixture,
        &directory(),
        Role::Manager,
        day(),
        DashboardPeriod::Month,
    )
    .expect("denial");
    assert!(manager.contains("role=\"alert\""));
    assert!(!manager.contains("Profit"));
    assert!(!manager.contains('₹'));
}
