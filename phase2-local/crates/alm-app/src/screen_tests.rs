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
        html.contains("System total for 30-Apr-2025: Cash ₹20 · Online ₹0 — from 1 bills/receipts")
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
    assert!(html.contains("<td>30-Apr-2025</td><td class=\"num pos\">₹20</td>"));
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
    assert!(html.contains(
        "1 bill · 01-Apr-2025 to 30-Apr-2025 · last month <span class=\"priv\">₹0</span>"
    ));
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
    let html = views::reorder_screen(&fixture, Role::Owner).unwrap();
    assert!(html.contains("<b>⚠️ 1 item(s) at or below reorder level.</b>"));
    assert!(html.contains(r#"<td class="num">50</td><td class="num">60</td><td class="num" style="font-weight:700;color:var(--accent)">130 pkt</td><td><span class="muted">buy ₹70.00</span></td><td class="num">₹9,100</td><td><span class="tag low">LOW</span>"#), "{html}");
    let manager = views::reorder_screen(&fixture, Role::Manager).unwrap();
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
        r#"<tr style="opacity:.5"><td>30-Apr-2025</td><td><b>ALM/25-26/2</b><span class="tag""#
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
        margin: String::new(),
        sell: "".into(),
        reorder: "60".into(),
        changed: String::new(),
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

#[test]
fn suppliers_render_core_quotes_best_and_hide_buy_rates_from_the_manager() {
    use alm_core::ProductId;
    use std::collections::HashMap;
    let root = Root::new("suppliers");
    let mut trial = crate::Trial::open(&root.0, Some(day())).unwrap();
    let html = trial.render("suppliers").unwrap();
    assert!(
        html.contains(
            r#"<th class="num">Synthetic Wholesale</th><th class="num">Synthetic Depot</th>"#
        ),
        "{html}"
    );
    assert!(html.contains(r#"value="69.00" style="color:var(--good);font-weight:700""#));
    assert!(html.contains(r#"<b class="pos">Synthetic Wholesale</b> @ ₹69"#));
    let depot = trial.authority.purchase_suppliers(Role::Owner).unwrap()[1]
        .id
        .index();
    let terms: HashMap<String, String> = [
        ("product".to_owned(), "1".to_owned()),
        (format!("s{depot}"), "68.50".to_owned()),
    ]
    .into();
    trial.supplier_quotes(terms).unwrap();
    let row = trial
        .authority
        .compare_supplier_prices(Role::Owner, Some(ProductId::new(1)))
        .unwrap()
        .rows
        .remove(0);
    assert_eq!(row.best.unwrap().1.minor_units(), 6_850);
    let html = trial.render("suppliers").unwrap();
    assert!(html.contains(r#"<b class="pos">Synthetic Depot</b> @ ₹69"#));
    assert!(html.contains(
        "<td><b>Synthetic Depot</b></td><td class=\"num\">0</td><td class=\"num\">₹0</td>"
    ));
    let bad: HashMap<String, String> = [
        ("product".to_owned(), "1".to_owned()),
        (format!("s{depot}"), "-1".to_owned()),
    ]
    .into();
    assert!(trial.supplier_quotes(bad).is_err());
    trial.owner = false;
    let html = trial.render("suppliers").unwrap();
    assert!(html.contains("role=\"alert\""));
    for hidden in ["₹", "68.50", "Synthetic Depot"] {
        assert!(!html.contains(hidden), "{hidden}");
    }
}

#[test]
fn order_planner_cart_po_and_paste_follow_core_without_stock() {
    let root = Root::new("order");
    let mut trial = crate::Trial::open(&root.0, Some(day())).unwrap();
    let html = trial.render("order").unwrap();
    assert!(html.contains(r#"<td><b>Synthetic Lime</b></td><td class="num">50</td><td class="num">5</td><td class="num" style="font-weight:700;color:var(--accent)">1 pkt</td><td class="num">₹69</td><td>Synthetic Wholesale</td>"#), "{html}");
    let line = |product: &str, qty: &str| crate::order::Line {
        product: product.into(),
        qty: qty.into(),
    };
    trial.order_add(&line("1", "6")).unwrap();
    trial.order_add(&line("2", "")).unwrap();
    assert!(trial.order_add(&line("1", "1")).is_err());
    let total = trial.order.planner.as_ref().unwrap().cart_total().unwrap();
    assert_eq!(total.minor_units(), 6 * 6_900 + 3_600);
    let html = trial.render("order").unwrap();
    assert!(html.contains("🛒 Cart: 2 items · ₹450"));
    trial.order_checkout().unwrap();
    let draft = trial.order.draft.clone().unwrap();
    assert_eq!(
        draft.supplier, None,
        "mixed best suppliers leave the supplier blank"
    );
    let po = |supplier: &str, paste: &str, intent: &str| crate::order::Po {
        supplier: supplier.into(),
        date: "2025-04-30".into(),
        note: "".into(),
        paste: paste.into(),
        intent: intent.into(),
    };
    trial
        .order_po(&po(
            "",
            "Item\tQty\tRate\nSynthetic Soda\t4\t35\nUnknown Fizz\t1\t1\n",
            "paste",
        ))
        .unwrap();
    assert!(
        trial
            .order
            .warnings
            .iter()
            .any(|w| w.contains("Unknown Fizz")),
        "{:?}",
        trial.order.warnings
    );
    let stock = (
        trial.authority.product_stock(alm_core::ProductId::new(1)),
        trial.authority.product_stock(alm_core::ProductId::new(2)),
    );
    assert!(trial.order_po(&po("", "", "save")).is_err());
    trial.order_po(&po("Synthetic Depot", "", "save")).unwrap();
    let orders = trial.authority.purchase_orders(Role::Owner).unwrap();
    assert_eq!(orders.len(), 1);
    assert_eq!(orders[0].supplier_name, "Synthetic Depot");
    assert_eq!(
        (
            trial.authority.product_stock(alm_core::ProductId::new(1)),
            trial.authority.product_stock(alm_core::ProductId::new(2))
        ),
        stock
    );
    trial.owner = false;
    let html = trial.render("order").unwrap();
    assert!(html.contains("role=\"alert\""));
    assert!(!html.contains('₹'));
}

#[test]
fn purchases_receive_dated_partials_close_remaining_and_deny_the_manager() {
    use alm_core::ProductId;
    use std::collections::HashMap;
    let root = Root::new("purchases");
    let mut trial = crate::Trial::open(&root.0, Some(day())).unwrap();
    trial
        .order_add(&crate::order::Line {
            product: "2".into(),
            qty: "10".into(),
        })
        .unwrap();
    trial.order_checkout().unwrap();
    trial
        .order_po(&crate::order::Po {
            supplier: "Synthetic Depot".into(),
            date: "2025-04-30".into(),
            note: "".into(),
            paste: "".into(),
            intent: "save".into(),
        })
        .unwrap();
    let html = trial.render("purchases").unwrap();
    assert!(html.contains(r#"<td><b>PO-001</b></td><td>Synthetic Depot</td><td><span class="tag low">Pending</span></td><td class="num">0 / 10</td><td class="num">₹360</td>"#), "{html}");
    let receive = |date: &str, qty: &str, rate: &str| -> HashMap<String, String> {
        [
            ("po", "0"),
            ("date", date),
            ("qty_0", qty),
            ("rate_0", rate),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect()
    };
    trial
        .po_receive_save(receive("2025-05-01", "3", "36"))
        .unwrap();
    trial
        .po_receive_save(receive("2025-05-03", "4", "37.50"))
        .unwrap();
    assert!(
        trial
            .po_receive_save(receive("2025-05-03", "0", ""))
            .is_err()
    );
    let lots = trial
        .authority
        .open_stock_lots(Role::Owner, ProductId::new(2))
        .unwrap();
    let layers: Vec<_> = lots
        .rows
        .iter()
        .map(|lot| (lot.remaining.get(), lot.unit_cost.map(|c| c.minor_units())))
        .collect();
    assert_eq!(
        layers,
        [(40, Some(3_800)), (3, Some(3_600)), (4, Some(3_750))]
    );
    let html = trial.render("purchases").unwrap();
    assert!(html.contains(
        r#"<span class="tag credit">Partially Received</span></td><td class="num">7 / 10</td>"#
    ));
    assert!(html.contains("Close 3 unreceived"));
    let id = trial.purchase_id(0).unwrap();
    trial
        .authority
        .close_remaining(Role::Owner, id, true)
        .unwrap();
    let html = trial.render("purchases").unwrap();
    assert!(
        html.contains(r#"<span class="tag ok">Completed</span></td><td class="num">7 / 10</td>"#)
    );
    assert_eq!(trial.authority.product_stock(ProductId::new(2)), Some(47));
    trial.owner = false;
    let html = trial.render("purchases").unwrap();
    assert!(html.contains("role=\"alert\""));
    assert!(!html.contains('₹'));
    assert!(
        trial
            .po_receive_save(receive("2025-05-04", "1", ""))
            .is_err()
    );
}

#[test]
fn crates_render_core_contents_and_value_and_deny_the_manager() {
    use std::collections::HashMap;
    let root = Root::new("crates");
    let mut trial = crate::Trial::open(&root.0, Some(day())).unwrap();
    trial.crate_form = Some(views::CrateForm {
        rows: vec![("1".into(), "2".into(), "Case".into())],
        ..Default::default()
    });
    let terms = |intent: &str| -> HashMap<String, String> {
        [
            ("name", "Party Crate"),
            ("price", "400"),
            ("product_0", "1"),
            ("qty_0", "2"),
            ("unit_0", "Case"),
            ("product_1", "2"),
            ("qty_1", "3"),
            ("unit_1", "Case"),
            ("intent", intent),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect()
    };
    trial.crate_submit(terms("add")).unwrap();
    assert_eq!(trial.crate_form.as_ref().unwrap().rows.len(), 2);
    trial.crate_submit(terms("save")).unwrap();
    let rows = trial.authority.crate_template_list(Role::Owner).unwrap();
    assert_eq!(rows[0].value, Ok(alm_core::Money::from_minor_units(42_850)));
    let html = trial.render("crates").unwrap();
    assert!(html.contains(r#"<td><b>Party Crate</b></td><td class="muted">2cs Synthetic Lime + 3cs Synthetic Soda</td><td class="num">2</td><td class="num">₹400</td><td class="num" title="Sum of contents at today's rates">₹429</td>"#), "{html}");
    let mut bad = terms("save");
    bad.insert("qty_1".into(), "0".into());
    trial.crate_form = Some(views::CrateForm {
        rows: vec![Default::default(), Default::default()],
        ..Default::default()
    });
    assert!(trial.crate_submit(bad).is_err());
    trial.owner = false;
    let html = trial.render("crates").unwrap();
    assert!(html.contains("role=\"alert\""));
    assert!(!html.contains('₹'));
}

#[test]
fn rate_list_renders_core_rows_ticks_and_text_and_denies_the_manager() {
    let root = Root::new("ratelist");
    let mut trial = crate::Trial::open(&root.0, Some(day())).unwrap();
    let html = trial.render("ratelist").unwrap();
    assert!(
        html.contains(
            r#"<td class="muted">Other</td><td><b>Synthetic Lime</b></td><td class="num">₹121</td>"#
        ),
        "{html}"
    );
    assert!(html.contains("Synthetic Lime — ₹120.5\nSynthetic Soda — ₹62.5"));
    trial.rates.toggle(alm_core::ProductId::new(2));
    let html = trial.render("ratelist").unwrap();
    assert!(!html.contains("Synthetic Soda — ₹"));
    trial.owner = false;
    let html = trial.render("ratelist").unwrap();
    assert!(html.contains("role=\"alert\""));
    assert!(!html.contains('₹'));
}

#[test]
fn reports_render_core_months_and_dead_stock_and_deny_the_manager() {
    use alm_core::confirmation::{dead_stock_cutoff, manual_cashbook::ManualCashBook};
    let fixture = books();
    let book = ManualCashBook::default();
    let today = alm_core::BillDate::new(2025, 6, 15).unwrap();
    let months = fixture
        .monthly_profit_report(Role::Owner, &book, today)
        .unwrap();
    let april = months.iter().find(|m| m.month == 4).unwrap();
    assert_eq!(
        (april.revenue.minor_units(), april.gross.minor_units()),
        (12_050, 5_050)
    );
    let cutoff = dead_stock_cutoff(today).unwrap();
    assert_eq!(cutoff, alm_core::BillDate::new(2025, 5, 16).unwrap());
    let dead = fixture.dead_stock_report(Role::Owner, cutoff).unwrap();
    let html = views::reports_screen(&fixture, &book, Role::Owner, today).unwrap();
    assert!(html.contains(r#"<tr><td>Apr 25</td><td class="num">1</td><td class="num">₹121</td><td class="num pos">₹51</td>"#), "{html}");
    assert!(html.contains("<th class=\"num\">Other Expenses</th><th class=\"num\">Net</th>"));
    assert!(html.contains(&format!(
        r#"<td class="num neg">{}</td>"#,
        money_r(april.other_expenses().unwrap())
    )));
    assert!(html.contains("Last sold before 16-May-2025."));
    for row in dead {
        assert!(html.contains(&money_r(row.locked_value)));
    }
    assert!(html.contains("<td class=\"muted\">30-Apr-2025</td>"));
    let manager = views::reports_screen(&fixture, &book, Role::Manager, today).unwrap();
    assert!(manager.contains("role=\"alert\""));
    assert!(!manager.contains('₹'));
}

#[test]
fn products_margin_preview_comes_from_core() {
    use alm_core::ProductId;
    let root = Root::new("product-preview");
    let mut trial = crate::Trial::open(&root.0, Some(day())).unwrap();
    trial.products.form = Some(
        trial
            .authority
            .open_product_entry(Role::Owner, ProductId::new(2))
            .unwrap(),
    );
    let form = |buy: &str, margin: &str, sell: &str, changed: &str| crate::products::Form {
        item: "Synthetic Soda".into(),
        brand: String::new(),
        buy: buy.into(),
        margin: margin.into(),
        sell: sell.into(),
        reorder: String::new(),
        changed: changed.into(),
    };
    let html = trial
        .product_preview(&form("38", "24.50", "62.50", "buy"))
        .unwrap();
    assert!(html.starts_with(r#"<div class="card" id="product-form""#));
    assert!(
        html.contains(r#"name="sell" inputmode="decimal" value="62.50""#),
        "{html}"
    );
    let html = trial
        .product_preview(&form("40", "24.50", "62.50", "buy"))
        .unwrap();
    assert!(html.contains(r#"name="sell" inputmode="decimal" value="64.50""#));
    let html = trial
        .product_preview(&form("40", "", "30", "sell"))
        .unwrap();
    assert!(html.contains(r#"name="margin" inputmode="decimal" value="-10.00""#));
    assert!(trial.product_preview(&form("x", "", "", "buy")).is_err());
    trial.owner = false;
    assert!(trial.product_preview(&form("40", "", "", "buy")).is_err());
}

#[test]
fn modals_live_fragments_searches_and_clipboard_text_come_from_rust() {
    use alm_core::ProductId;
    let root = Root::new("glue");
    let mut trial = crate::Trial::open(&root.0, Some(day())).unwrap();
    trial.authority = books();
    trial.customers.open = Some("batch1-customer".into());
    let html = trial.render("customers").unwrap();
    assert!(html.contains(r#"<template id="screen-modal"><div class="modal lg" role="dialog" aria-modal="true" aria-label="Synthetic Customer"><h3>👤 Synthetic Customer<button class="x" type="button" aria-label="Close" data-action="customer_close""#));
    trial.stock.lots = Some(ProductId::new(1));
    assert!(
        trial
            .render("stock")
            .unwrap()
            .contains(r#"data-action="stock_lots_close""#)
    );
    trial
        .cash_entry(entry("", "2025-04-30", "18", "check"))
        .unwrap();
    let html = trial.render("expenses").unwrap();
    assert!(html.contains(r#"data-live="cash_live" data-target="cash-system""#));
    let live = trial.cash_live(&entry("", "2025-04-30", "25", "")).unwrap();
    assert!(live.starts_with(r#"<div class="card" id="cash-system""#));
    assert!(
        live.contains(r#"Cash difference: <b class="neg">+₹5</b>"#),
        "{live}"
    );
    let options = trial.item_options("soda", "item_0", Some("fill")).unwrap();
    assert!(options.contains(r#"data-fill="item_0=Synthetic%20Soda" data-submit="fill""#));
    crate::godown::save(
        &mut trial,
        &crate::godown::Row {
            item: "Soda".into(),
            cases: "3".into(),
            note: String::new(),
        },
    )
    .unwrap();
    assert_eq!(
        trial.godown_copy_text().unwrap(),
        "*Godown Stock* (30-Apr-2025)\n\nSoda — 3 cases\n\nTotal: 1 items"
    );
    assert!(
        trial
            .rate_copy_text()
            .unwrap()
            .contains("Synthetic Lime — ₹120.5")
    );
    assert!(
        trial
            .reorder_copy_text()
            .unwrap()
            .starts_with("*ALM Reorder List* (30-Apr-2025)")
    );
    trial.owner = false;
    assert!(trial.godown_copy_text().is_err());
    assert!(trial.rate_copy_text().is_err());
    assert!(trial.reorder_copy_text().is_err());
    assert!(trial.item_options("", "item", None).is_err());
    assert!(trial.cash_live(&entry("", "2025-04-30", "25", "")).is_err());
}
