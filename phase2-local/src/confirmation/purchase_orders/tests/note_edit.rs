use super::*;

fn pending_order() -> (ConfirmationFixture, PurchaseOrderSelection) {
    let (mut fixture, mut draft) = fixture_with_prior_receipt();
    draft.note = "Original note".into();
    let id = fixture.save_purchase_order(OWNER, &draft).unwrap();
    (
        fixture,
        PurchaseOrderSelection {
            role: OWNER,
            id,
            number: 2,
        },
    )
}

#[test]
fn l08_04_owner_can_save_only_the_note_on_the_same_pending_order() {
    let (mut fixture, selection) = pending_order();
    let mut expected = fixture.clone();
    let view = fixture.view_purchase_order(selection, &[]).unwrap();
    assert_eq!(view.status, PurchaseOrderStatus::Pending);
    assert!(view.status.can_edit());

    let mut edit = fixture
        .open_purchase_order_note(OWNER, selection.id)
        .unwrap();
    assert_eq!(edit.note, "Original note");
    edit.note = "Updated note".into();
    assert_eq!(fixture, expected);

    assert_eq!(
        fixture.save_purchase_order_note(OWNER, &edit),
        Ok(selection.id)
    );
    expected.purchases.orders[selection.id.0].note = "Updated note".into();
    assert_eq!(fixture, expected);
}

#[test]
fn l08_04_receiving_two_disables_edit_and_rejects_an_open_note_without_mutation() {
    let (mut fixture, selection) = pending_order();
    let mut edit = fixture
        .open_purchase_order_note(OWNER, selection.id)
        .unwrap();
    edit.note = "Stale draft note".into();
    let date = BillDate::new(2025, 1, 12).unwrap();
    let rate = Money::from_minor_units(1_400);
    assert_eq!(
        fixture.receive_purchase_order(ReceiveRequest {
            role: OWNER,
            id: selection.id,
            date: Some(date),
            today: BillDate::new(2025, 1, 13).unwrap(),
            lines: &[ReceiveLine { quantity: 2, rate }],
        }),
        Ok(ReceiveResult::Received)
    );
    let after_receiving = fixture.clone();

    assert_eq!(
        fixture.save_purchase_order_note(OWNER, &edit),
        Err(PurchaseError::OrderNotPending)
    );

    let view = fixture.view_purchase_order(selection, &[]).unwrap();
    assert_eq!(view.status, PurchaseOrderStatus::PartiallyReceived);
    assert!(!view.status.can_edit());
    assert!(matches!(
        fixture.open_purchase_order_note(OWNER, selection.id),
        Err(PurchaseError::OrderNotPending)
    ));
    let order = &fixture.purchase_orders(OWNER).unwrap()[selection.id.0];
    assert_eq!(order.note, "Original note");
    assert_eq!(order.items[0].received, 2);
    assert_eq!(
        order.items[0].receipts(),
        &[PurchaseReceipt {
            date,
            quantity: NonZeroU32::new(2).unwrap(),
            rate
        }]
    );
    assert_eq!(fixture.stock(), 27);
    assert_eq!(
        fixture.catalog.receipt_lots(PRODUCT),
        vec![(date, 2, 1_400)]
    );
    assert_eq!(
        fixture.purchase_movements(OWNER, selection.id),
        Ok(vec![(date, 2)])
    );
    assert_eq!(fixture, after_receiving);
}

#[test]
fn l08_04_pending_order_rejects_a_stale_snapshot_after_another_note_save() {
    let (mut fixture, selection) = pending_order();
    let mut stale = fixture
        .open_purchase_order_note(OWNER, selection.id)
        .unwrap();
    let mut first = fixture
        .open_purchase_order_note(OWNER, selection.id)
        .unwrap();
    stale.note = "Stale draft note".into();
    first.note = "First save".into();
    fixture.save_purchase_order_note(OWNER, &first).unwrap();
    let before = fixture.clone();

    assert_eq!(
        fixture.save_purchase_order_note(OWNER, &stale),
        Err(PurchaseError::StaleOrderNote)
    );
    let view = fixture.view_purchase_order(selection, &[]).unwrap();
    assert!(view.status.can_edit());
    assert_eq!(fixture, before);
}

#[test]
fn l08_04_manager_cannot_open_or_save_a_pending_order_note() {
    let (mut fixture, selection) = pending_order();
    let mut edit = fixture
        .open_purchase_order_note(OWNER, selection.id)
        .unwrap();
    edit.note = "Manager change".into();
    let before = fixture.clone();

    assert!(matches!(
        fixture.open_purchase_order_note(Role::Manager, selection.id),
        Err(PurchaseError::AccessDenied)
    ));
    assert_eq!(
        fixture.save_purchase_order_note(Role::Manager, &edit),
        Err(PurchaseError::AccessDenied)
    );
    assert_eq!(fixture, before);
}
