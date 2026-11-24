//! Batch 1 integrated check, second half: a posted bill edited by the Owner, conflicting reuse of
//! its request, and the estimate's `Latest` balance against the historical bill values. The
//! half-rupee case uses the signed_recovery "aggregate-first" shape: paise are summed, then
//! rounded once by core.
use alm_core::confirmation::owner_billing::SaleTerms;
use alm_core::confirmation::{
    BalanceMode, ConfirmationError, DashboardPeriod, EditChanges, ReceiptMode,
};
use alm_core::{BillDate, DraftCustomerId, DraftId, DraftLine, Money, ProductId, Role};

use super::{Header, Intent, quantity};
use crate::Trial;
use crate::screen_tests::Root;
use crate::views::money_r;

const CUSTOMER: &str = "batch1-customer";
const BILL_DAY: &str = "2025-04-30";
const RECEIPT_DAY: &str = "2025-05-02";

fn date(text: &str) -> BillDate {
    text.parse().unwrap()
}

fn effects(trial: &Trial) -> (usize, Option<u32>, Option<u32>, Money, Money) {
    let a = &trial.authority;
    (
        a.posted_count(),
        a.product_stock(ProductId::new(1)),
        a.product_stock(ProductId::new(2)),
        a.ledger(),
        a.customer_due(CUSTOMER),
    )
}

fn prepare(trial: &mut Trial, paid: &str) -> u64 {
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
                paid,
            },
        )
        .unwrap();
    trial.posting.pending(Role::Owner).unwrap().serial()
}

#[test]
fn edited_bill_conflicting_request_and_latest_estimate_balance_reconcile() {
    let root = Root::new("batch1-edit");
    let mut trial = Trial::open(&root.0, Some(date(BILL_DAY))).unwrap();
    trial.render("bill").unwrap();
    trial
        .bill_add(ProductId::new(1), quantity("2").unwrap())
        .unwrap();
    trial
        .bill_header(Header {
            customer: CUSTOMER.into(),
            date: BILL_DAY.into(),
            loading: None,
            intent: Intent::Save,
        })
        .unwrap();
    let serial = prepare(&mut trial, "50.01");
    let first = trial
        .posting
        .confirm(&mut trial.authority, Role::Owner, serial)
        .unwrap();
    assert_eq!(
        (first.total.minor_units(), first.due),
        (24_100, Some(Money::from_minor_units(19_099)))
    );

    // Conflicting reuse: the same request ID (the draft's) with different reviewed terms.
    let posted = effects(&trial);
    let conflicting = prepare(&mut trial, "60.00");
    assert_eq!(
        trial
            .posting
            .confirm(&mut trial.authority, Role::Owner, conflicting),
        Err(ConfirmationError::RequestConflict.into())
    );
    assert_eq!(effects(&trial), posted);
    trial.posting.invalidate();

    // Owner edit: keep the Owner price and append 3 Synthetic Soda at ₹62.50.
    let changes = EditChanges {
        retained_price: Money::from_minor_units(12_050),
        appended_line: DraftLine::new(ProductId::new(2), quantity("3").unwrap()),
        appended_price: Money::from_minor_units(6_250),
    };
    assert_eq!(
        trial
            .authority
            .begin_edit(Role::Manager, DraftId::new(1), changes)
            .err(),
        Some(ConfirmationError::AccessDenied)
    );
    let edit = trial
        .authority
        .begin_edit(Role::Owner, DraftId::new(1), changes)
        .unwrap();
    let edited = trial.authority.save_edit(Role::Owner, &edit).unwrap();
    assert_eq!(
        (edited.invoice, edited.quantity, edited.total.minor_units()),
        (1, 5, 42_850)
    );
    assert_eq!(edited.due, Some(Money::from_minor_units(37_849)));
    assert_eq!(
        effects(&trial),
        (
            1,
            Some(48),
            Some(37),
            Money::from_minor_units(42_850),
            Money::from_minor_units(37_849)
        )
    );
    let sales = trial.render("sales").unwrap();
    assert!(sales.contains(r#"<td class="muted">2 items · 5 pkt</td><td><span class="tag credit">Credit</span></td><td class="num">₹429</td><td class="num neg">₹378</td>"#), "{sales}");
    let close = trial
        .authority
        .cashbook_day_close(Role::Owner, date(BILL_DAY))
        .unwrap();
    assert_eq!(
        (close.cash.minor_units(), close.cash_rupees, close.count),
        (5_001, 50, 1)
    );

    // Separately dated receipt; historical bill values stay, Latest follows the ledger.
    crate::receipts::save(
        &mut trial.authority,
        &trial.directory,
        Role::Owner,
        crate::receipts::Terms {
            customer: CUSTOMER.into(),
            date: RECEIPT_DAY.into(),
            amount: "30.99".into(),
            mode: crate::receipts::Mode::Cash,
        },
    )
    .unwrap();
    let as_on = trial
        .authority
        .bill_preview(Role::Owner, DraftId::new(1), BalanceMode::AsOnBillDate)
        .unwrap();
    let latest = trial
        .authority
        .bill_preview(Role::Owner, DraftId::new(1), BalanceMode::Latest)
        .unwrap();
    assert_eq!(
        (
            as_on.total.minor_units(),
            as_on.paid.minor_units(),
            as_on.balance.minor_units()
        ),
        (42_850, 5_001, 37_849)
    );
    assert_eq!((latest.total, latest.paid), (as_on.total, as_on.paid));
    // Core sums the exact ledger (34,750 paise) and rounds the aggregate once to whole rupees.
    assert_eq!(latest.balance.minor_units(), 34_800);
    // Aggregate first: ₹378.49 - ₹30.99 = ₹347.50 displays ₹348, not ₹378 - ₹31 = ₹347.
    assert_eq!(money_r(latest.balance), "₹348");
    assert_eq!(money_r(as_on.balance), "₹378");
    assert!(
        trial
            .authority
            .bill_preview(Role::Manager, DraftId::new(1), BalanceMode::Latest)
            .is_err()
    );

    trial.customers.open = Some(CUSTOMER.into());
    let customers = trial.render("customers").unwrap();
    assert!(customers.contains("<td>30-Apr-2025</td><td>Bill #1 (2 items)</td><td class=\"num\">₹429</td><td class=\"num pos\">₹50</td><td class=\"num neg\">₹378</td>"), "{customers}");
    assert!(customers.contains("<td>02-May-2025</td><td>Payment (Cash)</td><td class=\"num\"></td><td class=\"num pos\">₹31</td><td class=\"num neg\">₹348</td>"));
    let receipt_day = trial
        .authority
        .cashbook_day_close(Role::Owner, date(RECEIPT_DAY))
        .unwrap();
    assert_eq!(
        (receipt_day.cash.minor_units(), receipt_day.entries[0].mode),
        (3_099, ReceiptMode::Cash)
    );

    let dashboard = trial
        .authority
        .dashboard(Role::Owner, date(BILL_DAY), DashboardPeriod::Month)
        .unwrap();
    assert_eq!(
        (
            dashboard.current.sales.minor_units(),
            dashboard.current.collected.minor_units()
        ),
        (42_850, 5_001)
    );
    trial.today = Some(date(BILL_DAY));
    let html = trial.render("dashboard").unwrap();
    assert!(html.contains(r#"<span class="priv">₹429</span>"#));
    let book = trial
        .authority
        .customer_ledger(Role::Owner, &DraftCustomerId::new(CUSTOMER.into()))
        .unwrap();
    assert_eq!(book.closing.minor_units(), 34_750);

    // A second edit of the same bill is refused (core allows one revision), without effects.
    let before = effects(&trial);
    assert!(
        trial
            .authority
            .begin_edit(Role::Owner, DraftId::new(1), changes)
            .is_err()
    );
    assert_eq!(effects(&trial), before);
}

#[test]
fn in_place_line_edit_flows_through_sales_customers_cash_book_and_dashboard() {
    use std::collections::HashMap;
    let root = Root::new("batch1-inplace");
    let mut trial = Trial::open(&root.0, Some(date(BILL_DAY))).unwrap();
    trial.render("bill").unwrap();
    trial
        .bill_add(ProductId::new(1), quantity("2").unwrap())
        .unwrap();
    trial
        .bill_header(Header {
            customer: CUSTOMER.into(),
            date: BILL_DAY.into(),
            loading: None,
            intent: Intent::Save,
        })
        .unwrap();
    let serial = prepare(&mut trial, "50.01");
    trial
        .posting
        .confirm(&mut trial.authority, Role::Owner, serial)
        .unwrap();
    let posted = effects(&trial);
    assert_eq!(
        posted,
        (
            1,
            Some(48),
            Some(40),
            Money::from_minor_units(24_100),
            Money::from_minor_units(19_099)
        )
    );

    trial.sale_edit = Some(DraftId::new(1));
    let form = trial.render("sales").unwrap();
    assert!(form.contains(r#"name="qty_0" inputmode="numeric" aria-label="Edit quantity Synthetic Lime" value="2""#), "{form}");
    let terms = |qty: &str, rate: &str| -> HashMap<String, String> {
        [("id", "1"), ("qty_0", qty), ("rate_0", rate)]
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect()
    };
    assert!(trial.sale_edit_save(terms("0", "120.50")).is_err());
    assert!(trial.sale_edit_save(terms("1", "40")).is_err());
    assert_eq!(effects(&trial), posted);
    trial.sale_edit_save(terms("3", "118")).unwrap();
    let edited = effects(&trial);
    assert_eq!(
        edited,
        (
            1,
            Some(47),
            Some(40),
            Money::from_minor_units(35_400),
            Money::from_minor_units(30_399)
        )
    );
    let sales = trial.render("sales").unwrap();
    assert!(sales.contains(r#"<td class="muted">1 items · 3 pkt</td><td><span class="tag credit">Credit</span></td><td class="num">₹354</td><td class="num neg">₹304</td>"#), "{sales}");
    trial.customers.open = Some(CUSTOMER.into());
    let customers = trial.render("customers").unwrap();
    assert!(customers.contains("<td>Bill #1 (1 items)</td><td class=\"num\">₹354</td><td class=\"num pos\">₹50</td><td class=\"num neg\">₹304</td>"), "{customers}");
    let close = trial
        .authority
        .cashbook_day_close(Role::Owner, date(BILL_DAY))
        .unwrap();
    assert_eq!(close.cash.minor_units(), 5_001);
    let dashboard = trial
        .authority
        .dashboard(Role::Owner, date(BILL_DAY), DashboardPeriod::Month)
        .unwrap();
    assert_eq!(
        (
            dashboard.current.sales.minor_units(),
            dashboard.current.collected.minor_units()
        ),
        (35_400, 5_001)
    );
    trial.customers.open = None;
    crate::device::assert_book_view(&mut trial);
    trial.set_owner(false).unwrap();
    assert!(trial.sale_edit_save(terms("2", "118")).is_err());
    assert_eq!(effects(&trial), edited);
}
