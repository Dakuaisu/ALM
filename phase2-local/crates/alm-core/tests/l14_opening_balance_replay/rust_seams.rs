use std::num::NonZeroU32;

use alm_core::confirmation::{
    BalanceMode, ConfirmationFixture, CustomerReceiptInput, ReceiptMode, RequestId,
};
use alm_core::{
    BillDate, DraftCustomer, DraftCustomerId, DraftId, DraftLine, Money, ProductId,
    ProvisionalDraft, Role,
};

const OWNER: Role = Role::Owner;
const PRODUCT: ProductId = ProductId::new(1);

const fn money(paise: i64) -> Money {
    Money::from_minor_units(paise)
}

fn date(day: u8) -> BillDate {
    BillDate::new(2025, 2, day).unwrap()
}

fn named_draft(id: u64, customer: &DraftCustomerId, day: u8) -> ProvisionalDraft {
    ProvisionalDraft::new(
        DraftId::new(id),
        vec![DraftLine::new(PRODUCT, NonZeroU32::MIN)],
    )
    .with_customer(DraftCustomer::Selected {
        id: customer.clone(),
        name: "Shared Name".into(),
    })
    .with_bill_date(Some(date(day)))
}

#[test]
fn l14_opening_balance_api_seams_print_exact_observations() {
    let acct_a = DraftCustomerId::new("acct-a".into());
    let acct_b = DraftCustomerId::new("acct-b".into());
    let mut fixture = ConfirmationFixture::new(PRODUCT, 10, money(10_000)).unwrap();

    fixture.register_draft(named_draft(1, &acct_a, 10)).unwrap();
    let bill_review = fixture
        .review_named(OWNER, DraftId::new(1), money(2_000))
        .unwrap();
    let bill = fixture
        .confirm(OWNER, RequestId::new(1), &bill_review)
        .unwrap();
    assert_eq!(
        (
            bill.total.minor_units(),
            bill.received.unwrap().minor_units()
        ),
        (10_000, 2_000)
    );
    assert_eq!(bill.due.unwrap().minor_units(), 8_000);

    fixture
        .record_customer_receipt(
            OWNER,
            CustomerReceiptInput {
                customer_id: acct_a.clone(),
                date: date(12),
                amount: money(3_000),
                mode: ReceiptMode::Cash,
            },
        )
        .unwrap();
    let first_receipt_due = fixture.customer_due(acct_a.value()).minor_units();
    assert_eq!(first_receipt_due, 5_000);
    let first_receipt_as_on = fixture
        .bill_preview(OWNER, DraftId::new(1), BalanceMode::AsOnBillDate)
        .unwrap();
    let first_receipt_latest = fixture
        .bill_preview(OWNER, DraftId::new(1), BalanceMode::Latest)
        .unwrap();
    assert_eq!(first_receipt_as_on.balance.minor_units(), 8_000);
    assert_eq!(first_receipt_latest.balance.minor_units(), 5_000);

    fixture
        .record_customer_receipt(
            OWNER,
            CustomerReceiptInput {
                customer_id: acct_a.clone(),
                date: date(9),
                amount: money(1_000),
                mode: ReceiptMode::Online,
            },
        )
        .unwrap();
    let ledger_a = fixture.customer_ledger(OWNER, &acct_a).unwrap();
    let ledger_observations: Vec<_> = ledger_a
        .entries
        .iter()
        .map(|row| (row.date, row.balance.minor_units()))
        .collect();
    let as_on = fixture
        .bill_preview(OWNER, DraftId::new(1), BalanceMode::AsOnBillDate)
        .unwrap();
    let latest = fixture
        .bill_preview(OWNER, DraftId::new(1), BalanceMode::Latest)
        .unwrap();

    assert_eq!(acct_a.value(), "acct-a");
    assert_eq!(acct_b.value(), "acct-b");
    assert_ne!(acct_a, acct_b);
    assert_eq!(as_on.balance.minor_units(), 8_000);
    assert_eq!(latest.balance.minor_units(), 4_000);
    assert_eq!(
        ledger_observations,
        vec![(date(9), -1_000), (date(10), 7_000), (date(12), 4_000)]
    );
    assert_eq!(ledger_a.closing.minor_units(), 4_000);

    fixture.register_draft(named_draft(2, &acct_b, 10)).unwrap();
    let other_review = fixture
        .review_named(OWNER, DraftId::new(2), money(10_000))
        .unwrap();
    fixture
        .confirm(OWNER, RequestId::new(2), &other_review)
        .unwrap();
    assert_eq!(fixture.customer_due(acct_a.value()).minor_units(), 4_000);
    assert_eq!(fixture.customer_due(acct_b.value()).minor_units(), 0);
    let ledger_b = fixture.customer_ledger(OWNER, &acct_b).unwrap();
    assert_eq!(ledger_b.entries.len(), 1);
    assert_eq!(ledger_b.closing.minor_units(), 0);
    let preview_b = fixture
        .bill_preview(OWNER, DraftId::new(2), BalanceMode::AsOnBillDate)
        .unwrap();
    assert_eq!(as_on.customer, preview_b.customer);

    let walk_in_id = DraftId::new(3);
    fixture
        .register_draft(
            ProvisionalDraft::new(walk_in_id, vec![DraftLine::new(PRODUCT, NonZeroU32::MIN)])
                .with_customer(DraftCustomer::Typed("Local - Neighbor".into()))
                .with_bill_date(Some(date(13))),
        )
        .unwrap();
    let walk_in_review = fixture.review_walk_in_cash(OWNER, walk_in_id).unwrap();
    let walk_in = fixture
        .confirm(OWNER, RequestId::new(3), &walk_in_review)
        .unwrap();
    let walk_in_as_on = fixture
        .bill_preview(OWNER, walk_in_id, BalanceMode::AsOnBillDate)
        .unwrap();
    let walk_in_latest = fixture
        .bill_preview(OWNER, walk_in_id, BalanceMode::Latest)
        .unwrap();
    assert_eq!(walk_in.total.minor_units(), 10_000);
    assert_eq!(walk_in.received.unwrap().minor_units(), 10_000);
    assert_eq!(walk_in.due.unwrap().minor_units(), 0);
    assert_eq!(walk_in_as_on.balance.minor_units(), 0);
    assert_eq!(walk_in_latest.balance.minor_units(), 0);
    assert_eq!(walk_in_as_on.balance, walk_in_latest.balance);
    assert!(!walk_in_as_on.can_toggle);
    assert!(!walk_in_latest.can_toggle);
    assert_eq!(
        (
            fixture.customer_ledger(OWNER, &acct_a).unwrap(),
            fixture.customer_ledger(OWNER, &acct_b).unwrap(),
        ),
        (ledger_a.clone(), ledger_b.clone()),
        "walk-in posting must leave both named ledgers unchanged"
    );
    assert!(matches!(
        fixture.customer_ledger(OWNER, &DraftCustomerId::new("Local - Neighbor".into())),
        Err(alm_core::confirmation::ConfirmationError::InvalidDraft)
    ));

    println!(
        "L14 Rust API observations: IDs=({:?}:{}, {:?}:{}); invoice={} total={}p paid={}p bill_due={}p; after 12-Feb receipt: acct-a bill-preview as-on={}p latest={}p due={}p; backdated 09-Feb receipt posted later; acct-a customer-ledger document-order balances={:?} closing={}p; final acct-a bill-preview as-on={}p latest={}p due={}p; acct-b same_name={} due={}p ledger_closing={}p; walk-in invoice={} total={}p paid={}p due={}p bill-preview as-on={}p latest={}p toggles=({}, {}) named_ledgers_unchanged=true linked_ledger=InvalidDraft",
        acct_a,
        acct_a.value(),
        acct_b,
        acct_b.value(),
        bill.invoice,
        bill.total.minor_units(),
        bill.received.unwrap().minor_units(),
        bill.due.unwrap().minor_units(),
        first_receipt_as_on.balance.minor_units(),
        first_receipt_latest.balance.minor_units(),
        first_receipt_due,
        ledger_observations,
        ledger_a.closing.minor_units(),
        as_on.balance.minor_units(),
        latest.balance.minor_units(),
        fixture.customer_due(acct_a.value()).minor_units(),
        as_on.customer == preview_b.customer,
        fixture.customer_due(acct_b.value()).minor_units(),
        ledger_b.closing.minor_units(),
        walk_in.invoice,
        walk_in.total.minor_units(),
        walk_in.received.unwrap().minor_units(),
        walk_in.due.unwrap().minor_units(),
        walk_in_as_on.balance.minor_units(),
        walk_in_latest.balance.minor_units(),
        walk_in_as_on.can_toggle,
        walk_in_latest.can_toggle,
    );
}
