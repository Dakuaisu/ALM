use super::*;
use crate::DraftCustomerId;

mod isolation;
mod unknown_cost;

// L04-01: ALM_Business_Manager.html:4042-4074, 4118-4124, 4137-4201.
#[test]
fn edit_preserves_original_replay_and_retained_fifo() {
    let mut f = ConfirmationFixture::with_ordered_lots(
        ProductId::new(1),
        &[(3, Money::from_minor_units(40))],
        Money::from_minor_units(125),
    )
    .unwrap();
    f.add_product(ProductId::new(2), 5, Money::from_minor_units(80))
        .unwrap();
    f.catalog
        .seed_lots(
            ProductId::new(2),
            &[
                (2, Money::from_minor_units(30)),
                (3, Money::from_minor_units(50)),
            ],
        )
        .unwrap();
    let sale = draft(1, 2).with_customer(DraftCustomer::Selected {
        id: DraftCustomerId::new("customer".into()),
        name: "Synthetic".into(),
    });
    f.register_draft(sale).unwrap();
    let review = f
        .review_named(Role::Owner, DraftId::new(1), Money::from_minor_units(100))
        .unwrap();
    let original = f.confirm(Role::Owner, RequestId::new(1), &review).unwrap();
    assert_eq!(
        (
            f.catalog.stock(ProductId::new(1)),
            f.catalog.stock(ProductId::new(2))
        ),
        (Some(1), Some(5))
    );
    assert_eq!(
        (
            f.posted_count(),
            f.posted.len(),
            f.current_sales.len(),
            f.next_invoice
        ),
        (1, 1, 1, 2)
    );
    let edit = f
        .begin_edit(
            Role::Owner,
            DraftId::new(1),
            EditChanges {
                retained_price: Money::from_minor_units(150),
                appended_line: DraftLine::new(ProductId::new(2), NonZeroU32::new(3).unwrap()),
                appended_price: Money::from_minor_units(90),
            },
        )
        .unwrap();
    let updated = f.save_edit(Role::Owner, &edit).unwrap();
    assert_eq!(
        (
            f.catalog.stock(ProductId::new(1)),
            f.catalog.stock(ProductId::new(2))
        ),
        (Some(1), Some(2))
    );
    assert_eq!(
        (
            f.posted_count(),
            f.posted.len(),
            f.current_sales.len(),
            f.next_invoice
        ),
        (1, 1, 1, 2)
    );
    assert_eq!(
        (
            original.invoice,
            original.quantity,
            original.total.minor_units(),
            original.due.unwrap().minor_units()
        ),
        (1, 2, 250, 150)
    );
    assert_eq!(
        (
            updated.invoice,
            updated.quantity,
            updated.total.minor_units(),
            updated.received.unwrap().minor_units(),
            updated.due.unwrap().minor_units()
        ),
        (1, 5, 570, 100, 470)
    );
    assert_eq!(
        (
            f.ledger().minor_units(),
            f.received_ledger().minor_units(),
            f.customer_due("customer").minor_units()
        ),
        (570, 100, 470)
    );
    assert_eq!(
        f.catalog.allocation_snapshot(),
        vec![
            (1, 0, 0, 2, Some(80)),
            (1, 1, 0, 2, Some(60)),
            (1, 1, 1, 1, Some(50))
        ]
    );
    assert_eq!(f.catalog.allocated_cost(DraftId::new(1)), Some(Some(190)));
    let (current, saved) = f.current_sale(Role::Owner, DraftId::new(1)).unwrap();
    assert_eq!(current.draft().customer(), review.draft().customer());
    assert_eq!(current.draft().bill_date(), review.draft().bill_date());
    assert_eq!(
        current.unit_prices(),
        &[Money::from_minor_units(150), Money::from_minor_units(90)]
    );
    assert_eq!(saved, &updated);
    assert_eq!(
        f.confirm(Role::Owner, RequestId::new(1), &review),
        Ok(original)
    );
    assert_eq!(
        f.save_edit(Role::Owner, &edit),
        Err(ConfirmationError::StaleReview)
    );
}

fn setup() -> (ConfirmationFixture, Review, Receipt, EditChanges) {
    let mut f = ConfirmationFixture::with_ordered_lots(
        ProductId::new(1),
        &[(4, Money::from_minor_units(40))],
        Money::from_minor_units(125),
    )
    .unwrap();
    f.add_product(ProductId::new(2), 2, Money::from_minor_units(80))
        .unwrap();
    f.catalog
        .seed_lots(ProductId::new(2), &[(2, Money::from_minor_units(30))])
        .unwrap();
    f.register_draft(draft(1, 2).with_customer(DraftCustomer::Selected {
        id: DraftCustomerId::new("customer".into()),
        name: "Synthetic".into(),
    }))
    .unwrap();
    let review = f
        .review_named(Role::Owner, DraftId::new(1), Money::from_minor_units(100))
        .unwrap();
    let receipt = f.confirm(Role::Owner, RequestId::new(1), &review).unwrap();
    let changes = EditChanges {
        retained_price: Money::from_minor_units(150),
        appended_line: DraftLine::new(ProductId::new(2), NonZeroU32::new(3).unwrap()),
        appended_price: Money::from_minor_units(90),
    };
    (f, review, receipt, changes)
}

#[test]
fn shortage_and_manager_denial_leave_sale_unchanged() {
    let (mut f, review, receipt, changes) = setup();
    assert_eq!(
        f.begin_edit(Role::Manager, DraftId::new(1), changes),
        Err(ConfirmationError::AccessDenied)
    );
    assert_eq!(
        f.current_sale(Role::Manager, DraftId::new(1)),
        Err(ConfirmationError::AccessDenied)
    );
    let edit = f.begin_edit(Role::Owner, DraftId::new(1), changes).unwrap();
    let before = f.catalog.allocation_snapshot();
    assert_eq!(
        f.save_edit(Role::Manager, &edit),
        Err(ConfirmationError::AccessDenied)
    );
    assert_eq!(
        f.save_edit(Role::Owner, &edit),
        Err(ConfirmationError::InsufficientStock)
    );
    assert_eq!(
        f.current_sale(Role::Owner, DraftId::new(1)),
        Ok((&review, &receipt))
    );
    assert_eq!(f.catalog.allocation_snapshot(), before);
    assert_eq!(f.catalog.lot_snapshot(ProductId::new(1)), vec![2]);
    assert_eq!(f.catalog.lot_snapshot(ProductId::new(2)), vec![2]);
    assert_eq!(
        (
            f.ledger().minor_units(),
            f.received_ledger().minor_units(),
            f.customer_due("customer").minor_units()
        ),
        (250, 100, 150)
    );
}

#[test]
fn independent_price_and_quantity_changes_compute_expected_totals() {
    for (price, qty, expected) in [(130, 1, 350), (130, 2, 440), (160, 1, 410)] {
        let (mut f, _, _, mut changes) = setup();
        changes.retained_price = Money::from_minor_units(price);
        changes.appended_line = DraftLine::new(ProductId::new(2), NonZeroU32::new(qty).unwrap());
        let edit = f.begin_edit(Role::Owner, DraftId::new(1), changes).unwrap();
        assert_eq!(
            f.save_edit(Role::Owner, &edit).unwrap().total.minor_units(),
            expected
        );
    }
}
