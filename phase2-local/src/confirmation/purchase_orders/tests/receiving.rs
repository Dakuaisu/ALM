use super::*;
use crate::confirmation::{ReceiveLine, ReceiveRequest, ReceiveResult};
mod close_remaining;
mod matching;

fn date(day: u8) -> BillDate {
    BillDate::new(2025, 1, day).unwrap()
}
fn money(n: i64) -> Money {
    Money::from_minor_units(n)
}

fn fixture() -> (ConfirmationFixture, PurchaseOrderId) {
    let mut f =
        ConfirmationFixture::with_ordered_lots(PRODUCT, &[(10, money(1000))], money(1500)).unwrap();
    f.register_purchase_product(PurchaseProduct {
        id: PRODUCT,
        item: "Synthetic".into(),
        buy_rate: money(1000),
    })
    .unwrap();
    let id = f
        .save_purchase_order(
            OWNER,
            &PurchaseDraft {
                supplier: Some(PurchaseSupplier {
                    id: None,
                    name: "Supplier".into(),
                }),
                date: Some(date(1)),
                note: String::new(),
                items: vec![PurchaseLine {
                    item: "Synthetic".into(),
                    product_id: Some(PRODUCT),
                    ordered: NonZeroU32::new(5).unwrap(),
                    rate: money(1000),
                }],
            },
        )
        .unwrap();
    (f, id)
}

fn receive(
    f: &mut ConfirmationFixture,
    id: PurchaseOrderId,
    day: Option<u8>,
    qty: u32,
    rate: i64,
) -> Result<ReceiveResult, PurchaseError> {
    f.receive_purchase_order(ReceiveRequest {
        role: OWNER,
        id,
        date: day.map(date),
        today: date(25),
        lines: &[ReceiveLine {
            quantity: qty,
            rate: money(rate),
        }],
    })
}

#[test]
fn l08_partial_receipts_preserve_submission_order_and_sort_fifo() {
    let (mut f, id) = fixture();
    assert_eq!(
        receive(&mut f, id, Some(20), 2, 1300),
        Ok(ReceiveResult::Received)
    );
    assert_eq!(f.product_stock(PRODUCT), Some(12));
    assert_eq!(
        f.purchase_cost(OWNER, PRODUCT, "Supplier").unwrap(),
        Some((money(1050), money(1300), Some(money(1300))))
    );
    let first_line = &f.purchase_orders(OWNER).unwrap()[0].items[0];
    assert_eq!(first_line.received, 2);
    assert_eq!(
        first_line.receipts(),
        &[PurchaseReceipt {
            date: date(20),
            quantity: NonZeroU32::new(2).unwrap(),
            rate: money(1300)
        }]
    );
    assert_eq!(f.catalog.receipt_lots(PRODUCT), vec![(date(20), 2, 1300)]);
    assert_eq!(
        f.purchase_movements(OWNER, id).unwrap(),
        vec![(date(20), 2)]
    );
    assert_eq!(f.purchase_received_on(OWNER, id, date(20)), Ok(2));
    assert_eq!(f.purchase_received_on(OWNER, id, date(10)), Ok(0));
    let first = f
        .view_purchase_order(
            super::super::PurchaseOrderSelection {
                role: OWNER,
                id,
                number: 1,
            },
            &[],
        )
        .unwrap();
    assert_eq!(first.lines[0].remaining, 3);
    assert_eq!(
        first.status,
        super::super::PurchaseOrderStatus::PartiallyReceived
    );
    assert_eq!(
        receive(&mut f, id, Some(10), 3, 900),
        Ok(ReceiveResult::Received)
    );
    assert_eq!(f.product_stock(PRODUCT), Some(15));
    assert_eq!(
        f.purchase_cost(OWNER, PRODUCT, "Supplier").unwrap(),
        Some((money(1020), money(900), Some(money(900))))
    );
    let line = &f.purchase_orders(OWNER).unwrap()[0].items[0];
    assert_eq!(line.received, 5);
    assert_eq!(
        line.receipts(),
        &[
            PurchaseReceipt {
                date: date(20),
                quantity: NonZeroU32::new(2).unwrap(),
                rate: money(1300)
            },
            PurchaseReceipt {
                date: date(10),
                quantity: NonZeroU32::new(3).unwrap(),
                rate: money(900)
            },
        ]
    );
    assert_eq!(
        f.catalog.receipt_lots(PRODUCT),
        vec![(date(10), 3, 900), (date(20), 2, 1300)]
    );
    assert_eq!(
        f.purchase_movements(OWNER, id).unwrap(),
        vec![(date(20), 2), (date(10), 3)]
    );
    assert_eq!(f.purchase_received_on(OWNER, id, date(20)), Ok(2));
    assert_eq!(f.purchase_received_on(OWNER, id, date(10)), Ok(3));
    let view = f
        .view_purchase_order(
            super::super::PurchaseOrderSelection {
                role: OWNER,
                id,
                number: 1,
            },
            &[],
        )
        .unwrap();
    assert_eq!(view.lines[0].remaining, 0);
    assert_eq!(view.status, super::super::PurchaseOrderStatus::Completed);
}

#[test]
fn l08_controls_are_independent_after_first_receipt() {
    let (mut base, id) = fixture();
    receive(&mut base, id, Some(20), 2, 1300).unwrap();

    let mut zero = base.clone();
    let warning = receive(&mut zero, id, Some(10), 0, 900).unwrap();
    assert_eq!(warning, ReceiveResult::NothingToReceive);
    assert_eq!(zero, base);
    assert_eq!(
        warning.warning(),
        Some("Enter at least one quantity to receive")
    );

    let mut cap = base.clone();
    receive(&mut cap, id, Some(10), 50, 900).unwrap();
    let line = &cap.purchase_orders(OWNER).unwrap()[0].items[0];
    assert_eq!(line.received, 5);
    assert_eq!(line.receipts()[1].date, date(10));
    assert_eq!(line.receipts()[1].quantity.get(), 3);

    let mut blank_date = base.clone();
    receive(&mut blank_date, id, None, 1, 900).unwrap();
    assert_eq!(
        blank_date.purchase_orders(OWNER).unwrap()[0].items[0].receipts()[1].date,
        date(25)
    );

    let mut free = base.clone();
    receive(&mut free, id, Some(10), 3, 0).unwrap();
    let line = &free.purchase_orders(OWNER).unwrap()[0].items[0];
    assert_eq!(line.receipts()[1].rate, money(0));
    assert_eq!(line.line.rate, money(1300));
    assert_eq!(
        free.purchase_cost(OWNER, PRODUCT, "Supplier").unwrap(),
        Some((money(1050), money(1300), Some(money(1300))))
    );
    assert_eq!(
        free.catalog.receipt_lots(PRODUCT),
        vec![(date(10), 3, 1300), (date(20), 2, 1300)]
    );

    let mut denied = base.clone();
    let request = ReceiveRequest {
        role: Role::Manager,
        id,
        date: None,
        today: date(25),
        lines: &[ReceiveLine {
            quantity: 3,
            rate: money(900),
        }],
    };
    assert_eq!(
        denied.receive_purchase_order(request),
        Err(PurchaseError::AccessDenied)
    );
    assert_eq!(denied, base);
}

#[test]
fn l08_unmatched_receipt_logs_without_inventory_effects() {
    let (mut base, first_id) = fixture();
    receive(&mut base, first_id, Some(20), 2, 1300).unwrap();
    let mut f = base.clone();
    let id = f
        .save_purchase_order(
            OWNER,
            &PurchaseDraft {
                supplier: Some(PurchaseSupplier {
                    id: None,
                    name: "Supplier".into(),
                }),
                date: Some(date(1)),
                note: String::new(),
                items: vec![PurchaseLine {
                    item: "Unknown".into(),
                    product_id: None,
                    ordered: NonZeroU32::new(5).unwrap(),
                    rate: money(500),
                }],
            },
        )
        .unwrap();
    let catalog = f.catalog.clone();
    let cost = f.purchase_cost(OWNER, PRODUCT, "Supplier").unwrap();
    receive(&mut f, id, Some(10), 2, 900).unwrap();
    assert_eq!(f.catalog, catalog);
    assert_eq!(f.purchase_cost(OWNER, PRODUCT, "Supplier").unwrap(), cost);
    assert_eq!(f.purchase_orders(OWNER).unwrap()[1].items[0].received, 2);
    assert_eq!(
        f.purchase_movements(OWNER, id).unwrap(),
        vec![(date(10), 2)]
    );
}
