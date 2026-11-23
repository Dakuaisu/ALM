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
        html.contains("System total for 2025-04-30: Cash ₹20 · Online ₹0 — from 1 bills/receipts")
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

#[test]
fn reorder_renders_core_rows_and_hides_costs_from_the_manager() {
    use alm_core::ProductId;
    use alm_core::confirmation::ProductPriceInput;
    let mut fixture = fixture().expect("synthetic fixture");
    let mut form = fixture
        .open_product_entry(Role::Owner, ProductId::new(1))
        .unwrap();
    form.reorder_level = Some(60);
    form.apply_price(ProductPriceInput::Buy(Money::from_minor_units(7_000)))
        .unwrap();
    fixture.save_product_entry(Role::Owner, &form).unwrap();
    let rows = fixture.open_reorder_planner(Role::Owner).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].suggested_quantity.get(), 130);
    assert_eq!(rows[0].estimated_cost.minor_units(), 910_000);
    let html = views::reorder_screen(&fixture, Role::Owner, None).unwrap();
    assert!(html.contains("<b>⚠️ 1 item(s) at or below reorder level.</b>"));
    assert!(html.contains(r#"<td class="num">50</td><td class="num">60</td><td class="num" style="font-weight:700;color:var(--accent)">130 pkt</td><td><span class="muted">buy ₹70.00</span></td><td class="num">₹9,100</td><td><span class="tag low">LOW</span>"#), "{html}");
    let manager = views::reorder_screen(&fixture, Role::Manager, None).unwrap();
    assert!(manager.contains("role=\"alert\""));
    assert!(!manager.contains('₹'));
    assert!(!manager.contains("Synthetic Lime"));
}

#[test]
fn stock_renders_core_status_and_fifo_value_for_the_owner_only() {
    use alm_core::ProductId;
    let fixture = books();
    let list = fixture.stock_list(Role::Owner, "", false).unwrap();
    let value = list.value.unwrap();
    assert_eq!(value.minor_units(), 49 * 7_000 + 40 * 3_800);
    let state = views::StockState {
        lots: Some(ProductId::new(1)),
        ..views::StockState::default()
    };
    let html = views::stock_screen(&fixture, Role::Owner, &state).unwrap();
    assert!(html.contains(&format!(
        r#"<div class="label">Stock Value</div><div class="val">{}</div>"#,
        money_r(value)
    )));
    assert!(html.contains(r#"<td class="num" style="font-weight:800">49</td><td class="num">₹70</td><td class="num">₹3,430</td>"#), "{html}");
    let manager = views::stock_screen(&fixture, Role::Manager, &state).unwrap();
    assert!(manager.contains("role=\"alert\""));
    for hidden in ['₹', '4'] {
        assert!(!manager.contains(hidden), "{hidden}");
    }
    assert!(!manager.contains("FIFO"));
}

#[test]
fn godown_renders_the_core_manual_list_and_leaves_billing_stock_alone() {
    use alm_core::ProductId;
    let root = Root::new("godown");
    let mut trial = crate::Trial::open(&root.0, Some(day())).unwrap();
    let row = |item: &str, cases: &str| crate::godown::Row {
        item: item.into(),
        cases: cases.into(),
        note: "back rack".into(),
    };
    assert!(crate::godown::save(&mut trial, &row("Synthetic Soda", "1.5")).is_err());
    crate::godown::save(&mut trial, &row("Synthetic Soda", "12")).unwrap();
    crate::godown::save(&mut trial, &row("Synthetic Lime", "1000")).unwrap();
    let list = trial.authority.manual_godown_list(Role::Owner, "").unwrap();
    assert_eq!(list.total_cases, 1_012);
    let html = trial.render("godown").unwrap();
    assert!(html.contains("2 items · 1,012 cases"));
    assert!(html.contains(r#"<td><b>Synthetic Lime</b></td><td class="num" style="font-weight:800">1,000</td><td class="muted">back rack</td>"#));
    assert_eq!(trial.authority.product_stock(ProductId::new(2)), Some(40));
    trial.owner = false;
    assert!(crate::godown::save(&mut trial, &row("X", "1")).is_err());
    let html = trial.render("godown").unwrap();
    assert!(html.contains("role=\"alert\""));
    assert!(!html.contains("1,012"));
}

#[test]
fn gst_sales_list_form_cancel_delete_persist_and_deny_the_manager() {
    use alm_core::confirmation::{GstBillId, GstBillNumber};
    use std::collections::HashMap;
    let root = Root::new("gstsales");
    let mut trial = crate::Trial::open(&root.0, Some(day())).unwrap();
    let ordinary = trial.authority.without_gst_bills();
    let next = trial.authority.next_gst_bill_number(Role::Owner).unwrap();
    assert_eq!(next, GstBillNumber(2));
    trial.gst_form = Some(
        trial
            .authority
            .new_gst_bill_fields(Role::Owner, day())
            .unwrap(),
    );
    let form = trial.render("gstsales").unwrap();
    assert!(form.contains(r#"<label>GST Invoice No <input class="inp" value="2" readonly>"#));
    assert!(form.contains(r#"name="cust_state" value="Uttar Pradesh""#));
    let terms = |intent: &str, qty: &str| -> HashMap<String, String> {
        [
            ("customer", "Synthetic GST Buyer B"),
            ("gstin", "09ABCDE1234F1Z5"),
            ("date", "2025-04-30"),
            ("pay_mode", "UPI"),
            ("reverse_charge", "No"),
            ("eway_number", "123456789012"),
            ("transporter", "Self Delivery"),
            ("item_0", "Synthetic Soda"),
            ("unit_0", "Case"),
            ("qty_0", qty),
            ("rate_0", ""),
            ("gst_0", ""),
            ("pack_0", ""),
            ("intent", intent),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect()
    };
    trial.gst_submit(terms("fill", "2"), 50).unwrap();
    let form = trial.render("gstsales").unwrap();
    assert!(
        form.contains(r#"name="rate_0" inputmode="decimal" aria-label="Rate 1" value="62.50""#)
    );
    assert!(form.contains("<span class=\"num\">₹125.00</span>"));
    assert!(
        form.contains(r#"<span class="muted">Total</span><b>₹125.00</b>"#),
        "{form}"
    );
    let mut save = terms("save", "2");
    save.insert("rate_0".into(), "62.50".into());
    save.insert("gst_0".into(), "18".into());
    trial.gst_submit(save, 50).unwrap();
    let list = trial.render("gstsales").unwrap();
    let rows = trial.authority.gst_bill_list(Role::Owner).unwrap();
    assert_eq!(rows[0].number, "ALM/25-26/2");
    assert!(list.find("ALM/25-26/2").unwrap() < list.find("ALM/25-26/1").unwrap());
    assert!(list.contains(
        r#"<td>Synthetic GST Buyer B</td><td class="num">1</td><td class="num">₹125</td>"#
    ));

    trial
        .authority
        .cancel_gst_bill_to(Role::Owner, GstBillId(2), &mut trial.gst_db)
        .unwrap();
    let list = trial.render("gstsales").unwrap();
    assert!(list.contains(
        r#"<tr style="opacity:.5"><td>2025-04-30</td><td><b>ALM/25-26/2</b><span class="tag""#
    ));
    assert_eq!(
        trial.authority.next_gst_bill_number(Role::Owner).unwrap(),
        GstBillNumber(3)
    );
    trial
        .authority
        .delete_gst_bill_to(Role::Owner, GstBillId(2), &mut trial.gst_db)
        .unwrap();
    assert_eq!(trial.authority.without_gst_bills(), ordinary);
    drop(trial);

    let mut trial = crate::Trial::open(&root.0, Some(day())).unwrap();
    let ledger = trial.authority.gst_bill_ledger(Role::Owner).unwrap();
    assert_eq!(ledger.deleted, [GstBillId(2)]);
    assert_eq!(
        trial.authority.next_gst_bill_id(Role::Owner).unwrap(),
        GstBillId(3)
    );
    trial
        .authority
        .delete_gst_bill_to(Role::Owner, GstBillId(1), &mut trial.gst_db)
        .unwrap();
    drop(trial);
    let mut trial = crate::Trial::open(&root.0, Some(day())).unwrap();
    assert!(
        trial
            .authority
            .gst_bill_list(Role::Owner)
            .unwrap()
            .is_empty()
    );
    trial.owner = false;
    let html = trial.render("gstsales").unwrap();
    assert!(html.contains("role=\"alert\""));
    assert!(!html.contains('₹'));
    assert!(trial.gst_submit(HashMap::new(), 1).is_err());
}

#[test]
fn products_render_core_prices_margins_and_hide_them_from_the_manager() {
    use alm_core::ProductId;
    let root = Root::new("products");
    let mut trial = crate::Trial::open(&root.0, Some(day())).unwrap();
    trial.products.form = Some(
        trial
            .authority
            .open_product_entry(Role::Owner, ProductId::new(1))
            .unwrap(),
    );
    let form = crate::products::Form {
        item: "Synthetic Lime".into(),
        brand: "Synthetic Co".into(),
        buy: "70".into(),
        sell: "".into(),
        reorder: "60".into(),
    };
    trial.product_save(&form).unwrap();
    let row = trial
        .authority
        .product_list(Role::Owner, "lime", None)
        .unwrap()
        .rows
        .remove(0);
    assert_eq!(
        (row.buy.minor_units(), row.sell.minor_units()),
        (7_000, 19_050)
    );
    assert_eq!(row.margin_percent.as_deref(), Some("172.1%"));
    let html = trial.render("products").unwrap();
    assert!(html.contains(r#"<td class="num">₹70</td><td class="num">₹191</td><td class="num">172.1%</td><td class="num"><span>50 pkt</span> <span class="tag low">low</span>"#), "{html}");
    assert!(html.contains(r#"value="Synthetic Co">Synthetic Co</button>"#));
    trial.products.form = Some(
        trial
            .authority
            .open_product_entry(Role::Owner, ProductId::new(2))
            .unwrap(),
    );
    let dup = crate::products::Form {
        item: "synthetic lime".into(),
        ..form
    };
    assert!(
        trial
            .product_save(&dup)
            .unwrap_err()
            .contains("already exists")
    );
    trial.owner = false;
    trial.products.form = None;
    let html = trial.render("products").unwrap();
    assert!(html.contains("role=\"alert\""));
    for hidden in ["₹", "%", "Synthetic Lime"] {
        assert!(!html.contains(hidden), "{hidden}");
    }
}
