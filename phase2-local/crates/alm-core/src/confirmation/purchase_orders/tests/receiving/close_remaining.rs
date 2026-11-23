use super::super::{OWNER, PRODUCT};
use super::{date, fixture, money, receive};
use crate::Role;
use crate::confirmation::{
    PurchaseError, PurchaseOrderSelection, PurchaseOrderStatus, PurchaseReceipt, ReceiveResult,
};
use std::num::NonZeroU32;

#[test]
fn l08_close_remaining_requires_owner_confirmation_and_preserves_receipt_effects() {
    let (mut fixture, id) = fixture();
    assert_eq!(
        receive(&mut fixture, id, Some(20), 2, 1300),
        Ok(ReceiveResult::Received)
    );
    let received = fixture.clone();

    let prompt = fixture
        .view_purchase_order(
            PurchaseOrderSelection {
                role: OWNER,
                id,
                number: 1,
            },
            &[],
        )
        .unwrap();
    assert_eq!(prompt.lines[0].remaining, 3);
    assert_eq!(prompt.lines[0].cancelled, 0);
    assert_eq!(prompt.status, PurchaseOrderStatus::PartiallyReceived);
    assert_eq!(fixture, received);
    assert_eq!(fixture.close_remaining(OWNER, id, false), Ok(()));
    assert_eq!(fixture, received);
    assert_eq!(
        fixture.close_remaining(Role::Manager, id, true),
        Err(PurchaseError::AccessDenied)
    );
    assert_eq!(fixture, received);

    let mut expected = received.clone();
    expected.purchases.orders[0].items[0].cancelled = 3;
    assert_eq!(fixture.close_remaining(OWNER, id, true), Ok(()));
    assert_eq!(fixture, expected);

    let view = fixture
        .view_purchase_order(
            PurchaseOrderSelection {
                role: OWNER,
                id,
                number: 1,
            },
            &[],
        )
        .unwrap();
    assert_eq!(
        (
            view.lines[0].ordered,
            view.lines[0].received,
            view.lines[0].cancelled,
            view.lines[0].remaining
        ),
        (5, 2, 3, 0)
    );
    assert_eq!(view.status, PurchaseOrderStatus::Completed);
    assert_eq!(fixture.product_stock(PRODUCT), Some(12));
    assert_eq!(
        fixture.purchase_orders(OWNER).unwrap()[0].items[0].receipts(),
        &[PurchaseReceipt {
            date: date(20),
            quantity: NonZeroU32::new(2).unwrap(),
            rate: money(1300),
        }]
    );
    assert_eq!(
        fixture.purchase_cost(OWNER, PRODUCT, "Supplier").unwrap(),
        Some((money(1050), money(1300), Some(money(1300))))
    );
    assert_eq!(
        fixture.catalog.receipt_lots(PRODUCT),
        vec![(date(20), 2, 1300)]
    );
    assert_eq!(
        fixture.purchase_movements(OWNER, id).unwrap(),
        vec![(date(20), 2)]
    );
    assert_eq!(fixture.purchase_received_on(OWNER, id, date(20)), Ok(2));

    let completed = fixture.clone();
    assert_eq!(fixture.close_remaining(OWNER, id, true), Ok(()));
    assert_eq!(fixture, completed);
}
