use super::*;

fn save_order(fixture: &mut ConfirmationFixture, product_id: Option<ProductId>) -> PurchaseOrderId {
    fixture
        .save_purchase_order(
            OWNER,
            &PurchaseDraft {
                date: Some(date(8)),
                supplier: Some(PurchaseSupplier {
                    id: None,
                    name: "Synthetic Grove".into(),
                }),
                note: String::new(),
                items: vec![PurchaseLine {
                    item: ITEM.into(),
                    product_id,
                    ordered: qty(20),
                    rate: BUY,
                }],
            },
        )
        .unwrap()
}

fn receive(fixture: &mut ConfirmationFixture, id: PurchaseOrderId, (day, quantity): (u8, u32)) {
    assert_eq!(
        fixture.receive_purchase_order(ReceiveRequest {
            role: OWNER,
            id,
            date: Some(date(day)),
            today: date(25),
            lines: &[ReceiveLine {
                quantity,
                rate: BUY,
            }],
        }),
        Ok(ReceiveResult::Received)
    );
}

fn name_and_register_product(fixture: &mut ConfirmationFixture) {
    let mut entry = fixture.open_product_entry(OWNER, PRODUCT).unwrap();
    entry.item = ITEM.into();
    fixture.save_product_entry(OWNER, &entry).unwrap();
    fixture
        .register_purchase_product(PurchaseProduct {
            id: PRODUCT,
            item: ITEM.into(),
            buy_rate: BUY,
        })
        .unwrap();
}

fn receipt_row((opening, received, closing): (i64, i64, i64)) -> DailyMovementRow<'static> {
    DailyMovementRow {
        product: PRODUCT,
        item: Some(ITEM),
        opening,
        received,
        dispatched: 0,
        adjustment: 0,
        display_received: NonZeroI64::new(received),
        display_dispatched: None,
        closing,
    }
}

#[test]
fn l12_02_name_only_receipt_logs_follow_the_current_exact_catalog_name() {
    let mut fixture = ConfirmationFixture::new(PRODUCT, 10, SELL).unwrap();
    let id = save_order(&mut fixture, None);
    receive(&mut fixture, id, (10, 3));
    assert_eq!(fixture.product_stock(PRODUCT), Some(10));
    name_and_register_product(&mut fixture);
    receive(&mut fixture, id, (11, 5));
    assert_eq!(fixture.product_stock(PRODUCT), Some(15));

    let line = &fixture.purchase_orders(OWNER).unwrap()[0].items[0];
    assert_eq!(line.line.product_id, None);
    assert_eq!(line.received, 8);
    assert_eq!(
        line.receipts(),
        &[
            PurchaseReceipt {
                date: date(10),
                quantity: qty(3),
                rate: BUY,
            },
            PurchaseReceipt {
                date: date(11),
                quantity: qty(5),
                rate: BUY,
            },
        ]
    );

    // Then the frozen source counts both logs, despite the earlier receipt adding no stock.
    for (day, expected) in [(10, (10, 3, 13)), (11, (13, 5, 18)), (12, (18, 0, 18))] {
        check_rows(&fixture, day, &[receipt_row(expected)]);
    }
}

#[test]
fn l12_02_stale_stored_receipt_key_never_rekeys_to_the_name_match() {
    let mut fixture = ConfirmationFixture::new(PRODUCT, 10, SELL).unwrap();
    name_and_register_product(&mut fixture);
    let stale = ProductId::new(99);
    let id = save_order(&mut fixture, Some(stale));
    receive(&mut fixture, id, (11, 5));

    assert_eq!(fixture.product_stock(stale), None);
    assert_eq!(fixture.product_stock(PRODUCT), Some(15));
    let line = &fixture.purchase_orders(OWNER).unwrap()[0].items[0];
    assert_eq!(line.line.product_id, Some(stale));
    assert_eq!(
        line.receipts(),
        &[PurchaseReceipt {
            date: date(11),
            quantity: qty(5),
            rate: BUY,
        }]
    );

    // Then the register omits the unknown key, even though receiving found stock by name.
    check_rows(&fixture, 11, &[receipt_row((10, 0, 10))]);
    check_rows(&fixture, 12, &[receipt_row((10, 0, 10))]);
}
