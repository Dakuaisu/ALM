use super::*;

#[test]
fn unspecified_and_explicit_zero_cost_post_equivalently_but_record_distinct_costs() {
    let mut unspecified = fixture(2);
    let mut zero = ConfirmationFixture::with_ordered_lots(
        ProductId::new(1),
        &[(2, Money::from_minor_units(0))],
        Money::from_minor_units(125),
    )
    .unwrap();
    zero.register_draft(draft(1, 2)).unwrap();
    let unknown_review = unspecified.review(Role::Owner, DraftId::new(1)).unwrap();
    let zero_review = zero.review(Role::Owner, DraftId::new(1)).unwrap();
    let unknown_receipt = unspecified
        .confirm(Role::Owner, RequestId::new(9), &unknown_review)
        .unwrap();
    let zero_receipt = zero
        .confirm(Role::Owner, RequestId::new(9), &zero_review)
        .unwrap();
    assert_eq!(unknown_receipt, zero_receipt);
    assert_eq!(
        (
            unspecified.stock(),
            unspecified.ledger(),
            unspecified.next_invoice
        ),
        (zero.stock(), zero.ledger(), zero.next_invoice)
    );
    assert_eq!(
        unspecified.confirm(Role::Owner, RequestId::new(9), &unknown_review),
        Ok(unknown_receipt)
    );
    assert_eq!(
        zero.confirm(Role::Owner, RequestId::new(9), &zero_review),
        Ok(zero_receipt)
    );
    assert_eq!(
        unspecified.catalog.allocation_snapshot(),
        vec![(1, 0, 0, 2, None)]
    );
    assert_eq!(
        zero.catalog.allocation_snapshot(),
        vec![(1, 0, 0, 2, Some(0))]
    );
    assert_eq!(
        unspecified.catalog.allocated_cost(DraftId::new(1)),
        Some(None)
    );
    assert_eq!(zero.catalog.allocated_cost(DraftId::new(1)), Some(Some(0)));
}

#[test]
fn mixed_known_and_unspecified_costs_leave_sale_cost_unknown() {
    let mut f = ConfirmationFixture::with_ordered_lots(
        ProductId::new(1),
        &[(1, Money::from_minor_units(80))],
        Money::from_minor_units(125),
    )
    .unwrap();
    f.add_product(ProductId::new(2), 1, Money::from_minor_units(125))
        .unwrap();
    let order = dated_named_draft(
        DraftId::new(2),
        [1, 2]
            .into_iter()
            .map(|id| DraftLine::new(ProductId::new(id), NonZeroU32::new(1).unwrap()))
            .collect(),
    );
    let review = f.reconnect_review(Role::Owner, order).unwrap();
    f.confirm(Role::Owner, RequestId::new(2), &review).unwrap();
    assert_eq!(
        f.catalog.allocation_snapshot(),
        vec![(2, 0, 0, 1, Some(80)), (2, 1, 0, 1, None)]
    );
    assert_eq!(f.catalog.allocated_cost(DraftId::new(2)), Some(None));
}

#[test]
fn unknown_cost_does_not_disable_checked_known_cost_arithmetic() {
    let mut f = ConfirmationFixture::new(ProductId::new(1), 1, Money::from_minor_units(0)).unwrap();
    for (id, cost) in [(2, i64::MAX), (3, 1)] {
        f.add_product(ProductId::new(id), 1, Money::from_minor_units(0))
            .unwrap();
        f.catalog
            .seed_lots(ProductId::new(id), &[(1, Money::from_minor_units(cost))])
            .unwrap();
    }
    let order = dated_named_draft(
        DraftId::new(3),
        [1, 2, 3]
            .into_iter()
            .map(|id| DraftLine::new(ProductId::new(id), NonZeroU32::new(1).unwrap()))
            .collect(),
    );
    let review = f.reconnect_review(Role::Owner, order).unwrap();
    assert_eq!(
        f.confirm(Role::Owner, RequestId::new(3), &review),
        Err(ConfirmationError::Overflow)
    );
    assert!(f.catalog.allocation_snapshot().is_empty());
    assert_eq!(f.catalog.allocated_cost(DraftId::new(3)), None);
    assert_eq!(
        (
            f.stock(),
            f.product_stock(ProductId::new(2)),
            f.product_stock(ProductId::new(3)),
            f.ledger,
            f.next_invoice,
            f.requests.len(),
            f.posted.len()
        ),
        (1, Some(1), Some(1), 0, 1, 0, 0)
    );
}

#[test]
fn separate_sales_do_not_overflow_each_others_allocation_cost() {
    let mut f = ConfirmationFixture::with_ordered_lots(
        ProductId::new(1),
        &[
            (1, Money::from_minor_units(i64::MAX)),
            (1, Money::from_minor_units(1)),
        ],
        Money::from_minor_units(0),
    )
    .unwrap();
    let first = f.reconnect_review(Role::Owner, draft(1, 1)).unwrap();
    f.confirm(Role::Owner, RequestId::new(1), &first).unwrap();
    let second = f.reconnect_review(Role::Owner, draft(2, 1)).unwrap();
    assert!(f.confirm(Role::Owner, RequestId::new(2), &second).is_ok());
    assert_eq!(
        f.catalog.allocated_cost(DraftId::new(1)),
        Some(Some(i64::MAX))
    );
    assert_eq!(f.catalog.allocated_cost(DraftId::new(2)), Some(Some(1)));
    assert_eq!(f.catalog.lot_snapshot(ProductId::new(1)), vec![0, 0]);
    assert_eq!(
        f.catalog.allocation_snapshot(),
        vec![(1, 0, 0, 1, Some(i64::MAX)), (2, 0, 1, 1, Some(1))]
    );
}

#[test]
fn repeated_lines_allocate_oldest_lots_first() {
    let mut f = fixture(0);
    f.catalog
        .seed_lots(
            ProductId::new(1),
            &[
                (3, Money::from_minor_units(80)),
                (4, Money::from_minor_units(20)),
            ],
        )
        .unwrap();
    let draft = dated_named_draft(
        DraftId::new(43),
        [2, 3]
            .into_iter()
            .map(|qty| DraftLine::new(ProductId::new(1), NonZeroU32::new(qty).unwrap()))
            .collect(),
    );
    let review = f.reconnect_review(Role::Owner, draft).unwrap();
    f.confirm(Role::Owner, RequestId::new(9), &review).unwrap();
    assert_eq!(f.catalog.lot_snapshot(ProductId::new(1)), vec![0, 2]);
    assert_eq!(
        f.catalog.allocation_snapshot(),
        vec![
            (43, 0, 0, 2, Some(160)),
            (43, 1, 0, 1, Some(80)),
            (43, 1, 1, 2, Some(40))
        ]
    );
    assert_eq!(f.ledger().minor_units(), 625);
    assert_eq!(f.catalog.allocated_cost(DraftId::new(43)), Some(Some(280)));
}

#[test]
fn late_repeated_line_shortage_rolls_back_all_staged_effects() {
    let mut f = fixture(0);
    f.catalog
        .seed_lots(
            ProductId::new(1),
            &[
                (3, Money::from_minor_units(80)),
                (4, Money::from_minor_units(20)),
            ],
        )
        .unwrap();
    let draft = dated_named_draft(
        DraftId::new(43),
        [2, 6]
            .into_iter()
            .map(|qty| DraftLine::new(ProductId::new(1), NonZeroU32::new(qty).unwrap()))
            .collect(),
    );
    let review = f.reconnect_review(Role::Owner, draft).unwrap();
    assert_eq!(
        f.confirm(Role::Owner, RequestId::new(9), &review),
        Err(ConfirmationError::InsufficientStock)
    );
    assert_eq!(f.catalog.lot_snapshot(ProductId::new(1)), vec![3, 4]);
    assert!(f.catalog.allocation_snapshot().is_empty());
    assert_eq!(
        (
            f.catalog.allocated_cost(DraftId::new(43)),
            f.ledger,
            f.next_invoice,
            f.requests.len(),
            f.posted.len()
        ),
        (None, 0, 1, 0, 0)
    );
}

#[test]
fn allocation_cost_overflow_rolls_back_every_effect() {
    let mut f = fixture(0);
    f.catalog
        .seed_lots(
            ProductId::new(1),
            &[
                (1, Money::from_minor_units(i64::MAX)),
                (1, Money::from_minor_units(1)),
            ],
        )
        .unwrap();
    let review = f.review(Role::Owner, DraftId::new(1)).unwrap();
    assert_eq!(
        f.confirm(Role::Owner, RequestId::new(9), &review),
        Err(ConfirmationError::Overflow)
    );
    assert_eq!(f.catalog.lot_snapshot(ProductId::new(1)), vec![1, 1]);
    assert!(f.catalog.allocation_snapshot().is_empty());
    assert_eq!(
        (
            f.catalog.allocated_cost(DraftId::new(1)),
            f.ledger,
            f.next_invoice,
            f.requests.len(),
            f.posted.len()
        ),
        (None, 0, 1, 0, 0)
    );
}
