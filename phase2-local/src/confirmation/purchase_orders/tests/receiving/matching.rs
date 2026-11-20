use super::*;

#[test]
fn l08_later_invalid_line_rolls_back_earlier_staged_receipt() {
    let (mut f, id) = fixture();
    let line = f.purchases.orders[0].items[0].clone();
    f.purchases.orders[0].items.push(line);
    let before = f.clone();
    let lines = [
        ReceiveLine {
            quantity: 1,
            rate: money(1200),
        },
        ReceiveLine {
            quantity: 1,
            rate: money(-1),
        },
    ];
    let request = ReceiveRequest {
        role: OWNER,
        id,
        date: None,
        today: date(25),
        lines: &lines,
    };
    assert_eq!(
        f.receive_purchase_order(request),
        Err(PurchaseError::InvalidReceipt)
    );
    assert_eq!(f, before);
}

#[test]
fn l08_backdated_lot_traversal_keeps_existing_allocation_indices() {
    use crate::confirmation::RequestId;
    use crate::{DraftCustomer, DraftId, DraftLine, ProvisionalDraft};
    let (mut f, id) = fixture();
    let sale = |id, quantity| {
        ProvisionalDraft::new(
            DraftId::new(id),
            vec![DraftLine::new(PRODUCT, NonZeroU32::new(quantity).unwrap())],
        )
        .with_customer(DraftCustomer::Typed("Synthetic buyer".into()))
        .with_bill_date(Some(date(25)))
    };
    let review = f.reconnect_review(OWNER, sale(1, 1)).unwrap();
    f.confirm(OWNER, RequestId::new(1), &review).unwrap();
    receive(&mut f, id, Some(20), 2, 1300).unwrap();
    receive(&mut f, id, Some(10), 3, 900).unwrap();
    let review = f.reconnect_review(OWNER, sale(2, 10)).unwrap();
    f.confirm(OWNER, RequestId::new(2), &review).unwrap();
    assert_eq!(
        f.catalog.allocation_snapshot(),
        vec![
            (1, 0, 0, 1, Some(1000)),
            (2, 0, 0, 9, Some(9000)),
            (2, 0, 2, 1, Some(900))
        ]
    );
    assert_eq!(f.catalog.lot_snapshot(PRODUCT), vec![0, 2, 2]);
}

#[test]
fn l08_receipt_matches_name_when_id_is_absent_or_unresolved() {
    let (mut base, id) = fixture();
    receive(&mut base, id, Some(20), 2, 1300).unwrap();
    for product_id in [None, Some(ProductId::new(999))] {
        let mut f = base.clone();
        f.purchases.orders[0].items[0].line.product_id = product_id;
        receive(&mut f, id, Some(10), 1, 900).unwrap();
        assert_eq!(f.product_stock(PRODUCT), Some(13));
        assert_eq!(
            f.purchase_cost(OWNER, PRODUCT, "Supplier").unwrap(),
            Some((money(1038), money(900), Some(money(900))))
        );
        assert_eq!(
            f.catalog.receipt_lots(PRODUCT),
            vec![(date(10), 1, 900), (date(20), 2, 1300)]
        );
    }
}

#[test]
fn l08_resolved_id_takes_priority_over_a_different_matching_name() {
    let (mut f, id) = fixture();
    let other = ProductId::new(9);
    f.add_product(other, 0, money(1500)).unwrap();
    f.register_purchase_product(PurchaseProduct {
        id: other,
        item: "Other".into(),
        buy_rate: money(1000),
    })
    .unwrap();
    f.purchases.orders[0].items[0].line.item = "Other".into();
    receive(&mut f, id, Some(20), 1, 1300).unwrap();
    assert_eq!(f.product_stock(PRODUCT), Some(11));
    assert_eq!(f.product_stock(other), Some(0));
}
