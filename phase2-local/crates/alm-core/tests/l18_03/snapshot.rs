use alm_core::confirmation::ConfirmationFixture;
use alm_core::{DraftCustomerId, Role};

use super::SALE;

pub fn unchanged(
    fixture: &mut ConfirmationFixture,
    buyer: &DraftCustomerId,
    action: impl FnOnce(&mut ConfirmationFixture),
) {
    let (bill, receipt) = fixture.current_sale(Role::Owner, SALE).unwrap();
    let before = (
        bill.clone(),
        receipt.clone(),
        fixture.customer_ledger(Role::Owner, buyer).unwrap(),
        fixture.ledger(),
        fixture.received_ledger(),
        fixture.posted_count(),
    );
    action(fixture);
    let (bill, receipt) = fixture.current_sale(Role::Owner, SALE).unwrap();
    assert_eq!(
        before,
        (
            bill.clone(),
            receipt.clone(),
            fixture.customer_ledger(Role::Owner, buyer).unwrap(),
            fixture.ledger(),
            fixture.received_ledger(),
            fixture.posted_count()
        )
    );
}

#[test]
#[should_panic(expected = "assertion `left == right` failed")]
fn mutable_snapshot_detects_a_receipt_recorded_during_the_action() {
    use alm_core::confirmation::{CustomerReceiptInput, ReceiptMode};
    use alm_core::{BillDate, Money};

    let (mut fixture, buyer, _) = super::fixture();
    unchanged(&mut fixture, &buyer, |actual| {
        actual
            .record_customer_receipt(
                Role::Owner,
                CustomerReceiptInput {
                    customer_id: buyer.clone(),
                    date: BillDate::new(2025, 2, 23).unwrap(),
                    amount: Money::from_minor_units(50),
                    mode: ReceiptMode::Cash,
                },
            )
            .unwrap();
    });
}
