use super::*;

fn mixed_order() -> (ConfirmationFixture, PurchaseOrderId) {
    let mut fixture = ConfirmationFixture::with_ordered_lots(
        PRODUCT,
        &[(10, Money::from_minor_units(700))],
        Money::from_minor_units(1500),
    )
    .unwrap();
    let mut items = Vec::new();
    for (id, item, ordered, rate) in [
        (PRODUCT, "Tangerine", 8, 1000),
        (ProductId::new(9), "Apple", 2, 850),
        (ProductId::new(10), "Lime", 4, 0),
    ] {
        if id != PRODUCT {
            fixture
                .add_product(id, 0, Money::from_minor_units(1500))
                .unwrap();
        }
        fixture
            .register_purchase_product(PurchaseProduct {
                id,
                item: item.into(),
                buy_rate: Money::from_minor_units(700),
            })
            .unwrap();
        items.push(PurchaseLine {
            item: item.into(),
            product_id: Some(id),
            ordered: NonZeroU32::new(ordered).unwrap(),
            rate: Money::from_minor_units(rate),
        });
    }
    let id = fixture
        .save_purchase_order(
            OWNER,
            &PurchaseDraft {
                supplier: Some(PurchaseSupplier {
                    id: None,
                    name: "Synthetic Grove".into(),
                }),
                date: Some(BillDate::new(2025, 1, 2).unwrap()),
                note: String::new(),
                items,
            },
        )
        .unwrap();
    assert_eq!(
        fixture
            .receive_purchase_order(ReceiveRequest {
                role: OWNER,
                id,
                date: Some(BillDate::new(2025, 1, 6).unwrap()),
                today: BillDate::new(2025, 1, 7).unwrap(),
                lines: &[
                    ReceiveLine {
                        quantity: 3,
                        rate: Money::from_minor_units(1125),
                    },
                    ReceiveLine {
                        quantity: 2,
                        rate: Money::from_minor_units(890),
                    },
                    ReceiveLine {
                        quantity: 0,
                        rate: Money::from_minor_units(999),
                    },
                ],
            })
            .unwrap(),
        ReceiveResult::Received
    );
    (fixture, id)
}

#[test]
fn l08_10_receive_form_projects_saved_rows_in_order() {
    let (fixture, id) = mixed_order();
    let today = BillDate::new(2025, 1, 20).unwrap();

    let form = fixture
        .open_purchase_receive_form(OWNER, id, today)
        .unwrap();

    assert_eq!(form.id, id);
    assert_eq!(form.supplier_name, "Synthetic Grove");
    assert_eq!(form.received_on, today);
    let rows: Vec<_> = form
        .rows
        .iter()
        .map(|row| {
            (
                row.item.as_str(),
                row.ordered.get(),
                row.received,
                row.remaining,
                row.rate.minor_units(),
                row.receive_now,
                row.rate_editable,
                row.quantity_editable,
            )
        })
        .collect();
    assert_eq!(
        rows,
        [
            ("Tangerine", 8, 3, 5, 1125, 5, true, true),
            ("Apple", 2, 2, 0, 890, 0, false, false),
            ("Lime", 4, 0, 4, 0, 4, true, true),
        ]
    );
}

#[test]
fn l08_10_receive_form_open_and_discard_leave_fixture_unchanged() {
    let (fixture, id) = mixed_order();
    let before = fixture.clone();
    let today = BillDate::new(2025, 1, 20).unwrap();

    let mut form = fixture
        .open_purchase_receive_form(OWNER, id, today)
        .unwrap();
    assert_eq!(fixture, before);

    form.received_on = BillDate::new(2025, 1, 19).unwrap();
    form.rows[0].rate = Money::from_minor_units(1400);
    form.rows[0].receive_now = 1;
    drop(form);

    assert_eq!(fixture, before);
}

#[test]
fn l08_10_receive_form_warns_owner_when_order_is_missing() {
    let (fixture, _) = mixed_order();
    let before = fixture.clone();
    let today = BillDate::new(2025, 1, 20).unwrap();

    let error = fixture
        .open_purchase_receive_form(OWNER, PurchaseOrderId(99), today)
        .unwrap_err();

    assert_eq!(error, ReceiveFormError::OrderNotFound);
    assert_eq!(error.to_string(), "PO not found");
    assert_eq!(fixture, before);
}

#[test]
fn l08_10_receive_form_denies_manager_for_existing_and_missing_ids() {
    let (fixture, id) = mixed_order();
    let before = fixture.clone();
    let today = BillDate::new(2025, 1, 20).unwrap();

    for id in [id, PurchaseOrderId(99)] {
        assert_eq!(
            fixture.open_purchase_receive_form(Role::Manager, id, today),
            Err(ReceiveFormError::AccessDenied)
        );
    }
    assert_eq!(fixture, before);
}
