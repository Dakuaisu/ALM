//! Legacy case L05-01.
use super::*;
use crate::DraftCustomerId;

pub(super) fn preview_fixture(stock: u32) -> ConfirmationFixture {
    let mut f = fixture(stock);
    f.set_unit_price(Money::from_minor_units(12_500)).unwrap();
    f
}

fn named(f: &mut ConfirmationFixture, draft_id: u64, customer: &str, date: BillDate) {
    f.register_draft(draft(draft_id, 2).with_bill_date(Some(date)).with_customer(
        DraftCustomer::Selected {
            id: DraftCustomerId::new(customer.into()),
            name: "Shop".into(),
        },
    ))
    .unwrap();
    let review = f
        .review_named(
            Role::Owner,
            DraftId::new(draft_id),
            Money::from_minor_units(10_000),
        )
        .unwrap();
    f.confirm(Role::Owner, RequestId::new(draft_id), &review)
        .unwrap();
}

#[test]
fn l05_01_named_receipt_cancellation_and_restore_are_read_only() {
    let mut f = preview_fixture(5);
    named(&mut f, 1, "a", BillDate::new(2024, 2, 29).unwrap());
    let preview =
        |f: &ConfirmationFixture, mode| f.bill_preview(Role::Owner, DraftId::new(1), mode).unwrap();
    let snapshot = preview(&f, BalanceMode::AsOnBillDate);
    let latest = preview(&f, BalanceMode::Latest);
    assert_eq!(
        (snapshot.balance.minor_units(), latest.balance.minor_units()),
        (15_000, 15_000)
    );
    assert_eq!(snapshot.date, BillDate::new(2024, 2, 29).unwrap());
    assert_eq!(
        (snapshot.total.minor_units(), snapshot.paid.minor_units()),
        (25_000, 10_000)
    );
    assert_eq!(
        snapshot.lines,
        vec![PreviewLine {
            product: ProductId::new(1),
            quantity: NonZeroU32::new(2).unwrap(),
            price: Money::from_minor_units(12_500),
            amount: Money::from_minor_units(25_000),
        }]
    );
    assert!(!snapshot.cancelled);
    assert!(snapshot.can_toggle && latest.can_toggle);
    f.preview_payments.push(PreviewPayment {
        customer_id: "a".into(),
        amount: Money::from_minor_units(6_000),
        date: BillDate::new(2024, 3, 1).unwrap(),
        after_invoice: 1,
        mode: None,
        metadata: None,
    });
    let before = f.clone();
    let receipt_before = f.preview_payments.clone();
    let snapshot_after = preview(&f, BalanceMode::AsOnBillDate);
    let latest_after = preview(&f, BalanceMode::Latest);
    assert_eq!(preview(&f, BalanceMode::Latest), latest_after);
    assert_eq!(f.preview_payments, receipt_before);
    assert_eq!(
        (
            snapshot_after.balance.minor_units(),
            latest_after.balance.minor_units()
        ),
        (15_000, 9_000)
    );
    assert_eq!(snapshot_after.lines, snapshot.lines);
    assert_eq!(
        (
            latest_after.total,
            latest_after.paid,
            latest_after.cancelled
        ),
        (snapshot.total, snapshot.paid, false)
    );
    assert_eq!(f, before);
    assert_eq!(
        f.bill_preview(Role::Manager, DraftId::new(1), BalanceMode::Latest),
        Err(ConfirmationError::AccessDenied)
    );
    assert_eq!(f, before);
    f.cancel_sale(Role::Owner, DraftId::new(1)).unwrap();
    let cancelled = preview(&f, BalanceMode::Latest);
    assert!(cancelled.cancelled);
    assert!(cancelled.can_toggle);
    assert!(preview(&f, BalanceMode::AsOnBillDate).can_toggle);
    assert_eq!(cancelled.lines, snapshot.lines);
    assert_eq!(
        (cancelled.total, cancelled.paid),
        (snapshot.total, snapshot.paid)
    );
    assert_eq!(
        (
            preview(&f, BalanceMode::AsOnBillDate).balance.minor_units(),
            cancelled.balance.minor_units()
        ),
        (15_000, -6_000)
    );
    f.undo_cancel_sale(Role::Owner, DraftId::new(1)).unwrap();
    let restored = preview(&f, BalanceMode::Latest);
    assert!(!restored.cancelled);
    assert!(restored.can_toggle);
    assert_eq!(
        (
            preview(&f, BalanceMode::AsOnBillDate).balance.minor_units(),
            restored.balance.minor_units()
        ),
        (15_000, 9_000)
    );
}

#[test]
fn l05_01_walk_in_has_no_toggle_and_no_due() {
    let mut f = preview_fixture(5);
    f.register_draft(draft(1, 2).with_customer(DraftCustomer::Typed("Local - Neighbor".into())))
        .unwrap();
    let review = f.review_walk_in_cash(Role::Owner, DraftId::new(1)).unwrap();
    f.confirm(Role::Owner, RequestId::new(1), &review).unwrap();
    let before = f.clone();
    for mode in [BalanceMode::AsOnBillDate, BalanceMode::Latest] {
        let preview = f.bill_preview(Role::Owner, DraftId::new(1), mode).unwrap();
        assert!(!preview.can_toggle);
        assert_eq!(
            (
                preview.total.minor_units(),
                preview.paid.minor_units(),
                preview.balance.minor_units()
            ),
            (25_000, 25_000, 0)
        );
    }
    assert_eq!(f, before);
}

#[test]
fn l05_01_order_and_identity_are_not_document_date_or_name() {
    let mut f = preview_fixture(10);
    named(&mut f, 1, "a", BillDate::new(2024, 4, 1).unwrap());
    named(&mut f, 2, "b", BillDate::new(2024, 1, 1).unwrap());
    f.preview_payments.push(PreviewPayment {
        customer_id: "a".into(),
        amount: Money::from_minor_units(6_000),
        date: BillDate::new(2024, 5, 1).unwrap(),
        after_invoice: 1,
        mode: None,
        metadata: None,
    });
    named(&mut f, 3, "a", BillDate::new(2024, 2, 1).unwrap());
    let snap = f
        .bill_preview(Role::Owner, DraftId::new(3), BalanceMode::AsOnBillDate)
        .unwrap();
    assert_eq!(snap.balance.minor_units(), 24_000);
    assert_eq!(snap.previous_balance.unwrap().amount.minor_units(), 9_000);
    assert_eq!(
        snap.previous_balance.unwrap().date,
        Some(BillDate::new(2024, 5, 1).unwrap())
    );
}

#[test]
fn l05_01_preview_reads_edited_sale_not_original_replay() {
    let mut f = preview_fixture(8);
    f.add_product(ProductId::new(2), 3, Money::from_minor_units(9_000))
        .unwrap();
    named(&mut f, 1, "a", BillDate::new(2024, 2, 29).unwrap());
    let edit = f
        .begin_edit(
            Role::Owner,
            DraftId::new(1),
            EditChanges {
                retained_price: Money::from_minor_units(15_000),
                appended_line: DraftLine::new(ProductId::new(2), NonZeroU32::new(1).unwrap()),
                appended_price: Money::from_minor_units(9_000),
            },
        )
        .unwrap();
    f.save_edit(Role::Owner, &edit).unwrap();
    let before = f.clone();
    let preview = f
        .bill_preview(Role::Owner, DraftId::new(1), BalanceMode::AsOnBillDate)
        .unwrap();
    assert_eq!(preview.total.minor_units(), 39_000);
    assert_eq!(preview.paid.minor_units(), 10_000);
    assert_eq!(preview.balance.minor_units(), 29_000);
    assert_eq!(
        preview.lines,
        vec![
            PreviewLine {
                product: ProductId::new(1),
                quantity: NonZeroU32::new(2).unwrap(),
                price: Money::from_minor_units(15_000),
                amount: Money::from_minor_units(30_000),
            },
            PreviewLine {
                product: ProductId::new(2),
                quantity: NonZeroU32::new(1).unwrap(),
                price: Money::from_minor_units(9_000),
                amount: Money::from_minor_units(9_000),
            },
        ]
    );
    assert_eq!(f, before);
}

#[test]
fn l05_01_overflow_is_an_error_without_mutation() {
    let mut f = preview_fixture(5);
    named(&mut f, 1, "a", BillDate::new(2024, 2, 29).unwrap());
    f.preview_payments.push(PreviewPayment {
        customer_id: "a".into(),
        amount: Money::from_minor_units(i64::MIN),
        date: BillDate::new(2024, 3, 1).unwrap(),
        after_invoice: 1,
        mode: None,
        metadata: None,
    });
    let before = f.clone();
    assert_eq!(
        f.bill_preview(Role::Owner, DraftId::new(1), BalanceMode::Latest),
        Err(ConfirmationError::Overflow)
    );
    assert_eq!(f, before);
}
