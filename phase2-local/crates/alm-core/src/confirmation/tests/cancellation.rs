use super::*;
use crate::DraftCustomerId;

mod edge_cases;

fn setup() -> (ConfirmationFixture, Review, Receipt) {
    let mut f = ConfirmationFixture::with_ordered_lots(
        ProductId::new(1),
        &[
            (1, Money::from_minor_units(40)),
            (3, Money::from_minor_units(60)),
        ],
        Money::from_minor_units(125),
    )
    .unwrap();
    f.register_draft(draft(1, 2).with_customer(DraftCustomer::Selected {
        id: DraftCustomerId::new("customer".into()),
        name: "Synthetic, \"Shop\"".into(),
    }))
    .unwrap();
    let review = f
        .review_named(Role::Owner, DraftId::new(1), Money::from_minor_units(100))
        .unwrap();
    let receipt = f.confirm(Role::Owner, RequestId::new(1), &review).unwrap();
    (f, review, receipt)
}

#[test]
fn cancellation_and_undo_preserve_frozen_sale_and_serialize_status() {
    let (mut f, review, receipt) = setup();
    let id = DraftId::new(1);
    let allocations = f.catalog.allocation_snapshot();
    let original = f.requests.get(&RequestId::new(1)).unwrap().clone();
    assert_eq!(original, (review.clone(), receipt.clone()));
    assert_eq!(f.current_sale(Role::Owner, id), Ok((&review, &receipt)));
    assert_eq!(
        (f.next_invoice, f.posted.len(), f.current_sales.len()),
        (2, 1, 1)
    );
    let expected = [
        "Date,Bill No,Customer,Product,Price,Quantity,Total,Paid,Status\n2024-02-29,1,\"Synthetic, \"\"Shop\"\"\",1,125,2,250,100,",
        "Date,Bill No,Customer,Product,Price,Quantity,Total,Paid,Status\n2024-02-29,1,\"Synthetic, \"\"Shop\"\"\",1,125,2,250,100,CANCELLED",
    ];
    assert_eq!(f.sales_csv(Role::Owner, id).unwrap(), expected[0]);
    assert_eq!(
        f.gross_profit(Role::Owner, id)
            .unwrap()
            .unwrap()
            .minor_units(),
        150
    );
    f.cancel_sale(Role::Owner, id).unwrap();
    assert_eq!(
        (f.next_invoice, f.posted.len(), f.current_sales.len()),
        (2, 1, 1)
    );
    assert_eq!(f.requests.get(&RequestId::new(1)), Some(&original));
    assert_eq!(f.current_sale(Role::Owner, id), Ok((&review, &receipt)));
    assert_eq!(
        (
            f.stock(),
            f.ledger().minor_units(),
            f.received_ledger().minor_units(),
            f.customer_due("customer").minor_units()
        ),
        (4, 0, 0, 0)
    );
    assert_eq!(f.catalog.lot_snapshot(ProductId::new(1)), vec![1, 3]);
    assert_eq!(f.sales_csv(Role::Owner, id).unwrap(), expected[1]);
    assert_eq!(
        f.gross_profit(Role::Owner, id)
            .unwrap()
            .unwrap()
            .minor_units(),
        0
    );
    assert_eq!(
        f.confirm(Role::Owner, RequestId::new(1), &review),
        Ok(receipt.clone())
    );
    let cancelled = f.clone();
    f.cancel_sale(Role::Owner, id).unwrap();
    assert_eq!(f, cancelled);
    f.undo_cancel_sale(Role::Owner, id).unwrap();
    assert_eq!(
        (f.next_invoice, f.posted.len(), f.current_sales.len()),
        (2, 1, 1)
    );
    assert_eq!(f.requests.get(&RequestId::new(1)), Some(&original));
    assert_eq!(f.current_sale(Role::Owner, id), Ok((&review, &receipt)));
    assert_eq!(
        (
            f.stock(),
            f.ledger().minor_units(),
            f.received_ledger().minor_units(),
            f.customer_due("customer").minor_units()
        ),
        (2, 250, 100, 150)
    );
    assert_eq!(f.catalog.lot_snapshot(ProductId::new(1)), vec![0, 2]);
    assert_eq!(f.catalog.allocation_snapshot(), allocations);
    assert_eq!(f.sales_csv(Role::Owner, id).unwrap(), expected[0]);
    assert_eq!(
        f.gross_profit(Role::Owner, id)
            .unwrap()
            .unwrap()
            .minor_units(),
        150
    );
    let restored = f.clone();
    f.undo_cancel_sale(Role::Owner, id).unwrap();
    assert_eq!(f, restored);
    assert_eq!(f.current_sale(Role::Owner, id).unwrap().1, &receipt);
}

#[test]
fn manager_and_stale_edits_cannot_mutate_or_report() {
    let (mut f, _, _) = setup();
    let id = DraftId::new(1);
    let changes = EditChanges {
        retained_price: Money::from_minor_units(150),
        appended_line: DraftLine::new(ProductId::new(2), NonZeroU32::new(1).unwrap()),
        appended_price: Money::from_minor_units(90),
    };
    let edit = f.begin_edit(Role::Owner, id, changes).unwrap();
    let before = f.clone();
    // DEC-034: a Manager cancels a bill; restoring one stays the Owner's.
    let mut cancelled = f.clone();
    assert_eq!(cancelled.cancel_sale(Role::Manager, id), Ok(()));
    assert_eq!(
        cancelled.undo_cancel_sale(Role::Manager, id),
        Err(ConfirmationError::AccessDenied)
    );
    assert_eq!(
        f.undo_cancel_sale(Role::Manager, id),
        Err(ConfirmationError::AccessDenied)
    );
    assert_eq!(
        f.gross_profit(Role::Manager, id),
        Err(ConfirmationError::AccessDenied)
    );
    assert_eq!(
        f.sales_csv(Role::Manager, id),
        Err(ConfirmationError::AccessDenied)
    );
    assert_eq!(f, before);
    f.cancel_sale(Role::Owner, id).unwrap();
    assert_eq!(
        f.begin_edit(Role::Owner, id, changes),
        Err(ConfirmationError::InvalidDraft)
    );
    assert_eq!(
        f.save_edit(Role::Owner, &edit),
        Err(ConfirmationError::StaleReview)
    );
    f.undo_cancel_sale(Role::Owner, id).unwrap();
    assert_eq!(
        f.save_edit(Role::Owner, &edit),
        Err(ConfirmationError::StaleReview)
    );
}

#[test]
fn intervening_stock_movement_rejects_cancel_and_undo_atomically() {
    let (mut f, _, _) = setup();
    let id = DraftId::new(1);
    f.register_draft(draft(2, 1)).unwrap();
    let second = f.review(Role::Owner, DraftId::new(2)).unwrap();
    f.confirm(Role::Owner, RequestId::new(2), &second).unwrap();
    let before = f.clone();
    assert_eq!(
        f.cancel_sale(Role::Owner, id),
        Err(ConfirmationError::StaleReview)
    );
    assert_eq!(f, before);
    let (mut f, _, _) = setup();
    f.cancel_sale(Role::Owner, id).unwrap();
    f.register_draft(draft(2, 1)).unwrap();
    let second = f.review(Role::Owner, DraftId::new(2)).unwrap();
    f.confirm(Role::Owner, RequestId::new(2), &second).unwrap();
    let before = f.clone();
    assert_eq!(
        f.undo_cancel_sale(Role::Owner, id),
        Err(ConfirmationError::StaleReview)
    );
    assert_eq!(f, before);
}

#[test]
fn overflow_leaves_everything_unchanged() {
    let (mut f, _, _) = setup();
    f.ledger = i64::MIN;
    let before = f.clone();
    assert_eq!(
        f.cancel_sale(Role::Owner, DraftId::new(1)),
        Err(ConfirmationError::Overflow)
    );
    assert_eq!(f, before);
}

#[test]
fn reports_reject_sales_outside_the_part_paid_scope() {
    let mut f = fixture(4);
    let review = f.review(Role::Owner, DraftId::new(1)).unwrap();
    f.confirm(Role::Owner, RequestId::new(1), &review).unwrap();
    assert_eq!(
        f.gross_profit(Role::Owner, DraftId::new(1)),
        Err(ConfirmationError::InvalidDraft)
    );
    assert_eq!(
        f.sales_csv(Role::Owner, DraftId::new(1)),
        Err(ConfirmationError::InvalidDraft)
    );
}

#[test]
fn edited_sale_restores_each_original_fifo_layer() {
    let mut f = ConfirmationFixture::with_ordered_lots(
        ProductId::new(1),
        &[(3, Money::from_minor_units(40))],
        Money::from_minor_units(125),
    )
    .unwrap();
    f.add_product(ProductId::new(2), 3, Money::from_minor_units(90))
        .unwrap();
    f.catalog
        .seed_lots(
            ProductId::new(2),
            &[
                (1, Money::from_minor_units(30)),
                (2, Money::from_minor_units(50)),
            ],
        )
        .unwrap();
    f.register_draft(draft(1, 2).with_customer(DraftCustomer::Selected {
        id: DraftCustomerId::new("customer".into()),
        name: "Synthetic".into(),
    }))
    .unwrap();
    let review = f
        .review_named(Role::Owner, DraftId::new(1), Money::from_minor_units(100))
        .unwrap();
    f.confirm(Role::Owner, RequestId::new(1), &review).unwrap();
    let edit = f
        .begin_edit(
            Role::Owner,
            DraftId::new(1),
            EditChanges {
                retained_price: Money::from_minor_units(150),
                appended_line: DraftLine::new(ProductId::new(2), NonZeroU32::new(2).unwrap()),
                appended_price: Money::from_minor_units(90),
            },
        )
        .unwrap();
    f.save_edit(Role::Owner, &edit).unwrap();
    let allocations = f.catalog.allocation_snapshot();
    assert_eq!(
        f.gross_profit(Role::Owner, DraftId::new(1))
            .unwrap()
            .unwrap()
            .minor_units(),
        320
    );
    f.cancel_sale(Role::Owner, DraftId::new(1)).unwrap();
    assert_eq!(
        (
            f.catalog.lot_snapshot(ProductId::new(1)),
            f.catalog.lot_snapshot(ProductId::new(2))
        ),
        (vec![3], vec![1, 2])
    );
    f.undo_cancel_sale(Role::Owner, DraftId::new(1)).unwrap();
    assert_eq!(
        (
            f.catalog.lot_snapshot(ProductId::new(1)),
            f.catalog.lot_snapshot(ProductId::new(2))
        ),
        (vec![1], vec![0, 1])
    );
    assert_eq!(f.catalog.allocation_snapshot(), allocations);
    assert_eq!(
        (
            f.ledger().minor_units(),
            f.received_ledger().minor_units(),
            f.customer_due("customer").minor_units()
        ),
        (480, 100, 380)
    );
}
