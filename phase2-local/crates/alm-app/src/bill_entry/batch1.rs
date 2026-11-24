//! Batch 1 integrated check: one named customer from a provisional draft through Owner
//! confirmation, a separately dated receipt, Customers, Cash Book close, cancellation and the
//! Dashboard. Exact paise are asserted from core; rendered rupees are checked separately.
use alm_core::confirmation::DashboardPeriod;
use alm_core::confirmation::owner_billing::{CustomerFilter, SaleTerms};
use alm_core::{BillDate, DraftCustomerId, DraftId, Money, ProductId, Role};

use super::{Header, Intent, quantity};
use crate::Trial;
use crate::screen_tests::{Root, entry};
use crate::views::money_r;

const CUSTOMER: &str = "batch1-customer";
const BILL_DAY: &str = "2025-04-30";
const RECEIPT_DAY: &str = "2025-05-02";

fn date(text: &str) -> BillDate {
    text.parse().unwrap()
}

fn balance(trial: &Trial) -> Money {
    trial
        .authority
        .customer_ledger(Role::Owner, &DraftCustomerId::new(CUSTOMER.into()))
        .unwrap()
        .closing
}

fn day_close(trial: &Trial, day: &str) -> (i64, i64, usize) {
    let close = trial
        .authority
        .cashbook_day_close(Role::Owner, date(day))
        .unwrap();
    (
        close.cash.minor_units(),
        close.online.minor_units(),
        close.count,
    )
}

#[test]
fn one_customer_draft_bill_receipt_books_cancellation_and_dashboard_reconcile() {
    let root = Root::new("batch1");
    // Manager saves a provisional draft; nothing is posted.
    let mut trial = Trial::open(&root.0, Some(date(BILL_DAY))).unwrap();
    trial.owner = false;
    trial.render("bill").unwrap();
    trial
        .bill_add(ProductId::new(1), quantity("2").unwrap())
        .unwrap();
    trial
        .bill_header(Header {
            customer: CUSTOMER.into(),
            date: BILL_DAY.into(),
            intent: Intent::Save,
        })
        .unwrap();
    assert_eq!(trial.authority.posted_count(), 0);
    drop(trial);

    // Reopened after a restart, still provisional; the Owner reviews changed (Owner) prices.
    let mut trial = Trial::open(&root.0, Some(date(BILL_DAY))).unwrap();
    trial.bill_open(DraftId::new(1)).unwrap();
    assert_eq!(trial.authority.posted_count(), 0);
    let review = trial.render("review").unwrap();
    assert!(review.contains("<b>Owner total ₹241.00</b>"), "{review}");
    let (draft, _) = trial.current.clone().unwrap();
    trial
        .posting
        .prepare(
            &mut trial.authority,
            &trial.directory,
            Role::Owner,
            Some(&draft),
            SaleTerms {
                date: BILL_DAY,
                paid: "50.00",
            },
        )
        .unwrap();
    let serial = trial.posting.pending(Role::Owner).unwrap().serial();
    assert!(
        trial
            .posting
            .confirm(&mut trial.authority, Role::Manager, serial)
            .is_err()
    );
    let receipt = trial
        .posting
        .confirm(&mut trial.authority, Role::Owner, serial)
        .unwrap();
    // A lost response retried with the same request returns the same receipt, posting once.
    let retry = trial
        .posting
        .confirm(&mut trial.authority, Role::Owner, serial)
        .unwrap();
    assert_eq!(retry, receipt);
    assert_eq!(trial.authority.posted_count(), 1);
    assert_eq!(receipt.total.minor_units(), 24_100);
    assert_eq!(receipt.received, Some(Money::from_minor_units(5_000)));

    let sales = trial.render("sales").unwrap();
    assert!(sales.contains("<b>#1</b>"));
    assert!(
        sales.contains(r#"<td class="num">₹241</td><td class="num neg">₹191</td>"#),
        "{sales}"
    );

    // Separately dated online receipt.
    crate::receipts::save(
        &mut trial.authority,
        &trial.directory,
        Role::Owner,
        crate::receipts::Terms {
            customer: CUSTOMER.into(),
            date: RECEIPT_DAY.into(),
            amount: "30.50".into(),
            mode: crate::receipts::Mode::Online,
        },
    )
    .unwrap();
    assert_eq!(balance(&trial).minor_units(), 24_100 - 5_000 - 3_050);
    trial.customers.open = Some(CUSTOMER.into());
    let customers = trial.render("customers").unwrap();
    assert_eq!(money_r(balance(&trial)), "₹161");
    assert!(customers.contains(r#"<td class="num neg">₹161</td>"#));
    assert!(customers.contains("<td>02-May-2025</td><td>Payment (Online)</td><td class=\"num\"></td><td class=\"num pos\">₹31</td><td class=\"num neg\">₹161</td>"));

    // Cash Book close for both dates, saved from system figures.
    assert_eq!(day_close(&trial, BILL_DAY), (5_000, 0, 1));
    assert_eq!(day_close(&trial, RECEIPT_DAY), (0, 3_050, 1));
    for day in [BILL_DAY, RECEIPT_DAY] {
        trial.cash_entry(entry("", day, "", "system")).unwrap();
        let (draft, _) = trial.cash_form.unwrap();
        let mut save = entry("", day, &draft.amounts.cash.to_rupee_text(), "save");
        save.paytm = draft.amounts.paytm.to_rupee_text();
        trial.cash_entry(save).unwrap();
    }
    let rows = trial.cashbook.rows(Role::Owner).unwrap();
    let saved: Vec<_> = rows
        .iter()
        .map(|row| {
            let a = row.amounts;
            (
                row.date.to_string(),
                a.cash.minor_units(),
                a.paytm.minor_units(),
            )
        })
        .collect();
    assert_eq!(
        saved,
        [(RECEIPT_DAY.into(), 0, 3_100), (BILL_DAY.into(), 5_000, 0)]
    );

    trial.today = Some(date(RECEIPT_DAY));
    let before = trial
        .authority
        .dashboard(Role::Owner, date(RECEIPT_DAY), DashboardPeriod::Month)
        .unwrap();
    assert_eq!(before.prior.sales.minor_units(), 24_100);
    assert_eq!(before.current.receipts.minor_units(), 3_050);

    // Cancel the bill: the receipt survives, balances, Cash Book and Dashboard follow.
    trial
        .authority
        .cancel_sale(Role::Owner, DraftId::new(1))
        .unwrap();
    assert_eq!(balance(&trial).minor_units(), -3_050);
    assert_eq!(day_close(&trial, BILL_DAY), (0, 0, 0));
    assert_eq!(day_close(&trial, RECEIPT_DAY), (0, 3_050, 1));
    let book = trial
        .directory
        .customer_book(&trial.authority, Role::Owner, &CustomerFilter::All)
        .unwrap();
    assert_eq!(
        (book.receivable.minor_units(), book.advances.minor_units()),
        (0, 3_050)
    );
    let customers = trial.render("customers").unwrap();
    assert!(customers.contains(r#"<td class="num pos">₹31 adv</td>"#));
    assert!(customers.contains("Payment (Online)"));
    assert!(!customers.contains("Bill #1"));
    trial.cash_edit_for_test(0);
    let cash = trial.render("expenses").unwrap();
    assert!(
        cash.contains("Cash difference: <b class=\"neg\">+₹50</b>"),
        "{cash}"
    );
    let after = trial
        .authority
        .dashboard(Role::Owner, date(RECEIPT_DAY), DashboardPeriod::Month)
        .unwrap();
    assert_eq!(after.prior.sales.minor_units(), 0);
    assert_eq!(after.current.collected.minor_units(), 3_050);
    let dashboard = trial.render("dashboard").unwrap();
    assert!(dashboard.contains(
        "0 bills · 0 pkt · avg <span class=\"priv\">₹0</span> per bill · last month <span class=\"priv\">₹0</span>"
    ));
    assert!(dashboard.contains(r#"<div class="help">01-May-2025 to 02-May-2025</div>"#));
    assert!(dashboard.contains("bills ₹0 · receipts ₹31"));
    assert!(dashboard.contains(
        r#"<div class="sl">Receivables</div><div class="sv"><span class="priv">₹0</span>"#
    ));
    let counter = trial.render("counter").unwrap();
    assert!(counter.contains("Nothing needs attention right now"));

    // The Manager is denied every book screen after the same timeline.
    trial.owner = false;
    for tab in ["sales", "customers", "expenses", "dashboard", "counter"] {
        let html = trial.render(tab).unwrap();
        assert!(html.contains("role=\"alert\""), "{tab}");
        assert!(!html.contains('₹'), "{tab}");
    }
}
