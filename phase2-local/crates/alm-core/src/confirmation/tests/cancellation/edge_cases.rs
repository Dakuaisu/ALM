use super::*;

#[test]
fn edited_sale_with_zero_due_can_cancel_report_and_restore() {
    let mut f = ConfirmationFixture::with_ordered_lots(
        ProductId::new(1),
        &[(4, Money::from_minor_units(10))],
        Money::from_minor_units(125),
    )
    .unwrap();
    f.add_product(ProductId::new(2), 2, Money::from_minor_units(60))
        .unwrap();
    f.catalog
        .seed_lots(ProductId::new(2), &[(2, Money::from_minor_units(20))])
        .unwrap();
    f.register_draft(draft(1, 2).with_customer(DraftCustomer::Selected {
        id: DraftCustomerId::new("customer".into()),
        name: "Synthetic".into(),
    }))
    .unwrap();
    let review = f
        .review_named(Role::Owner, DraftId::new(1), Money::from_minor_units(100))
        .unwrap();
    let original = f.confirm(Role::Owner, RequestId::new(1), &review).unwrap();
    let edit = f
        .begin_edit(
            Role::Owner,
            DraftId::new(1),
            EditChanges {
                retained_price: Money::from_minor_units(20),
                appended_line: DraftLine::new(ProductId::new(2), NonZeroU32::new(1).unwrap()),
                appended_price: Money::from_minor_units(60),
            },
        )
        .unwrap();
    let receipt = f.save_edit(Role::Owner, &edit).unwrap();
    assert_eq!(receipt.due, Some(Money::from_minor_units(0)));
    let allocations = f.catalog.allocation_snapshot();
    let active_csv = "Date,Bill No,Customer,Product,Price,Quantity,Total,Paid,Status\n2024-02-29,1,Synthetic,1,20,2,100,100,\n2024-02-29,1,Synthetic,2,60,1,,,";
    let cancelled_csv = "Date,Bill No,Customer,Product,Price,Quantity,Total,Paid,Status\n2024-02-29,1,Synthetic,1,20,2,100,100,CANCELLED\n2024-02-29,1,Synthetic,2,60,1,,,CANCELLED";
    assert_eq!(
        f.sales_csv(Role::Owner, DraftId::new(1)).unwrap(),
        active_csv
    );
    assert_eq!(
        f.gross_profit(Role::Owner, DraftId::new(1)),
        Ok(Some(Money::from_minor_units(60)))
    );
    f.cancel_sale(Role::Owner, DraftId::new(1)).unwrap();
    assert_eq!(
        (
            f.stock(),
            f.product_stock(ProductId::new(2)),
            f.ledger().minor_units(),
            f.received_ledger().minor_units(),
            f.customer_due("customer").minor_units()
        ),
        (4, Some(2), 0, 0, 0)
    );
    assert_eq!(
        f.sales_csv(Role::Owner, DraftId::new(1)).unwrap(),
        cancelled_csv
    );
    assert_eq!(
        f.gross_profit(Role::Owner, DraftId::new(1)),
        Ok(Some(Money::from_minor_units(0)))
    );
    assert_eq!(
        f.confirm(Role::Owner, RequestId::new(1), &review),
        Ok(original)
    );
    f.undo_cancel_sale(Role::Owner, DraftId::new(1)).unwrap();
    assert_eq!(
        (
            f.stock(),
            f.product_stock(ProductId::new(2)),
            f.ledger().minor_units(),
            f.received_ledger().minor_units(),
            f.customer_due("customer").minor_units()
        ),
        (2, Some(1), 100, 100, 0)
    );
    assert_eq!(f.catalog.allocation_snapshot(), allocations);
    assert_eq!(
        f.current_sale(Role::Owner, DraftId::new(1)).unwrap().1,
        &receipt
    );
    assert_eq!(
        f.sales_csv(Role::Owner, DraftId::new(1)).unwrap(),
        active_csv
    );
    assert_eq!(
        f.gross_profit(Role::Owner, DraftId::new(1)),
        Ok(Some(Money::from_minor_units(60)))
    );
}

#[test]
fn unknown_cost_restores_lots_and_reports_unknown_only_when_active() {
    let mut f =
        ConfirmationFixture::new(ProductId::new(1), 4, Money::from_minor_units(125)).unwrap();
    f.register_draft(draft(1, 2).with_customer(DraftCustomer::Selected {
        id: DraftCustomerId::new("customer".into()),
        name: "Synthetic".into(),
    }))
    .unwrap();
    let review = f
        .review_named(Role::Owner, DraftId::new(1), Money::from_minor_units(100))
        .unwrap();
    let receipt = f.confirm(Role::Owner, RequestId::new(1), &review).unwrap();
    let allocations = f.catalog.allocation_snapshot();
    assert_eq!(f.gross_profit(Role::Owner, DraftId::new(1)), Ok(None));
    assert_eq!(
        f.gross_profit(Role::Manager, DraftId::new(1)),
        Err(ConfirmationError::AccessDenied)
    );
    f.cancel_sale(Role::Owner, DraftId::new(1)).unwrap();
    assert_eq!(f.catalog.lot_snapshot(ProductId::new(1)), vec![4]);
    assert_eq!(
        (
            f.ledger().minor_units(),
            f.received_ledger().minor_units(),
            f.customer_due("customer").minor_units()
        ),
        (0, 0, 0)
    );
    assert_eq!(
        f.gross_profit(Role::Owner, DraftId::new(1)),
        Ok(Some(Money::from_minor_units(0)))
    );
    assert_eq!(
        f.sales_csv(Role::Owner, DraftId::new(1)).unwrap(),
        "Date,Bill No,Customer,Product,Price,Quantity,Total,Paid,Status\n2024-02-29,1,Synthetic,1,125,2,250,100,CANCELLED"
    );
    f.undo_cancel_sale(Role::Owner, DraftId::new(1)).unwrap();
    assert_eq!(f.catalog.lot_snapshot(ProductId::new(1)), vec![2]);
    assert_eq!(f.catalog.allocation_snapshot(), allocations);
    assert_eq!(f.catalog.allocated_cost(DraftId::new(1)), Some(None));
    assert_eq!(f.gross_profit(Role::Owner, DraftId::new(1)), Ok(None));
    assert_eq!(
        f.current_sale(Role::Owner, DraftId::new(1)).unwrap().1,
        &receipt
    );
    assert_eq!(
        f.sales_csv(Role::Owner, DraftId::new(1)).unwrap(),
        "Date,Bill No,Customer,Product,Price,Quantity,Total,Paid,Status\n2024-02-29,1,Synthetic,1,125,2,250,100,"
    );
}
