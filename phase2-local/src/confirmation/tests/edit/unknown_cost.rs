use super::*;

fn fixture(known_retained: bool, known_appended: bool) -> (ConfirmationFixture, EditChanges) {
    let mut f =
        ConfirmationFixture::new(ProductId::new(1), 4, Money::from_minor_units(125)).unwrap();
    if known_retained {
        f.catalog
            .seed_lots(ProductId::new(1), &[(4, Money::from_minor_units(40))])
            .unwrap();
    }
    f.add_product(ProductId::new(2), 3, Money::from_minor_units(80))
        .unwrap();
    if known_appended {
        f.catalog
            .seed_lots(ProductId::new(2), &[(3, Money::from_minor_units(30))])
            .unwrap();
    }
    f.register_draft(draft(1, 2).with_customer(DraftCustomer::Selected {
        id: DraftCustomerId::new("customer".into()),
        name: "Synthetic".into(),
    }))
    .unwrap();
    let review = f
        .review_named(Role::Owner, DraftId::new(1), Money::from_minor_units(100))
        .unwrap();
    f.confirm(Role::Owner, RequestId::new(1), &review).unwrap();
    (
        f,
        EditChanges {
            retained_price: Money::from_minor_units(150),
            appended_line: DraftLine::new(ProductId::new(2), NonZeroU32::new(2).unwrap()),
            appended_price: Money::from_minor_units(90),
        },
    )
}

#[test]
fn retained_unknown_cost_remains_unknown_after_edit() {
    let (mut f, changes) = fixture(false, true);
    let edit = f.begin_edit(Role::Owner, DraftId::new(1), changes).unwrap();
    let result = f.save_edit(Role::Owner, &edit).unwrap();
    assert_eq!(result.total.minor_units(), 480);
    assert_eq!(f.catalog.allocated_cost(DraftId::new(1)), Some(None));
    assert_eq!(
        f.catalog.allocation_snapshot(),
        vec![(1, 0, 0, 2, None), (1, 1, 0, 2, Some(60))]
    );
    assert_eq!(
        (
            f.catalog.stock(ProductId::new(1)),
            f.catalog.stock(ProductId::new(2))
        ),
        (Some(2), Some(1))
    );
}

#[test]
fn appended_unknown_cost_remains_unknown_after_edit() {
    let (mut f, changes) = fixture(true, false);
    let edit = f.begin_edit(Role::Owner, DraftId::new(1), changes).unwrap();
    let result = f.save_edit(Role::Owner, &edit).unwrap();
    assert_eq!(result.total.minor_units(), 480);
    assert_eq!(f.catalog.allocated_cost(DraftId::new(1)), Some(None));
    assert_eq!(
        f.catalog.allocation_snapshot(),
        vec![(1, 0, 0, 2, Some(80)), (1, 1, 0, 2, None)]
    );
    assert_eq!(
        (
            f.catalog.stock(ProductId::new(1)),
            f.catalog.stock(ProductId::new(2))
        ),
        (Some(2), Some(1))
    );
}
