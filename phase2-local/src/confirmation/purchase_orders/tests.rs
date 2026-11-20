use super::*;

mod output;

const PRODUCT: ProductId = ProductId::new(8);
const OWNER: Role = Role::Owner;

fn synthetic_receipt() -> PurchaseReceipt {
    PurchaseReceipt {
        date: BillDate::new(2025, 1, 6).unwrap(),
        quantity: NonZeroU32::MIN,
        rate: Money::from_minor_units(330),
    }
}

fn fixture_with_prior_receipt() -> (ConfirmationFixture, PurchaseDraft) {
    let mut fixture = ConfirmationFixture::with_ordered_lots(
        PRODUCT,
        &[(25, Money::from_minor_units(330))],
        Money::from_minor_units(500),
    )
    .unwrap();
    fixture
        .register_purchase_product(PurchaseProduct {
            id: PRODUCT,
            item: "Synthetic Orange".into(),
            buy_rate: Money::from_minor_units(350),
        })
        .unwrap();
    let draft = PurchaseDraft {
        date: Some(BillDate::new(2025, 1, 2).unwrap()),
        supplier: Some(PurchaseSupplier {
            id: None,
            name: "Synthetic Grove".into(),
        }),
        note: "Prior PO".into(),
        items: vec![PurchaseLine {
            item: "Synthetic Orange".into(),
            product_id: Some(PRODUCT),
            ordered: NonZeroU32::new(4).unwrap(),
            rate: Money::from_minor_units(300),
        }],
    };
    fixture.save_purchase_order(OWNER, &draft).unwrap();
    let line = &mut fixture.purchases.orders[0].items[0];
    line.received = 1;
    line.receipts.push(synthetic_receipt());
    (fixture, draft)
}

#[test]
fn l07_01_parse_preserves_prior_receipt_records_received_count_and_stock() {
    let (fixture, mut draft) = fixture_with_prior_receipt();
    let inventory_before = fixture.catalog.clone();
    let purchases_before = fixture.purchases.clone();

    let paste = concat!(
        "Date\tSupplier\tItem\tQty\tRate\n",
        "2025-02-01\tSynthetic Grove\tSynthetic Orange\t2\t3.80\n",
    );
    fixture
        .parse_purchase_paste(OWNER, &mut draft, paste)
        .unwrap();

    assert_eq!(draft.items[0].ordered.get(), 2);
    let line = &fixture.purchase_orders(OWNER).unwrap()[0].items[0];
    assert_eq!(line.received, 1);
    assert_eq!(line.receipts(), &[synthetic_receipt()]);
    assert_eq!(fixture.purchases, purchases_before);
    assert_eq!(fixture.catalog, inventory_before);
}

#[test]
fn l07_02_save_preserves_a_seeded_partially_received_order_and_inventory() {
    let (mut fixture, mut draft) = fixture_with_prior_receipt();
    let prior_order = fixture.purchases.orders[0].clone();
    let inventory_before = fixture.catalog.clone();
    let suppliers_before = fixture.purchases.suppliers.clone();
    draft.note = "New PO".into();
    draft.items[0].product_id = None;
    draft.items[0].rate = Money::from_minor_units(370);

    let new_id = fixture.save_purchase_order(OWNER, &draft).unwrap();

    let [prior, new] = fixture.purchase_orders(OWNER).unwrap() else {
        panic!("expected the original and a new PO")
    };
    assert_eq!(prior, &prior_order);
    assert_eq!(prior.items[0].received, 1);
    assert_eq!(prior.items[0].receipts(), &[synthetic_receipt()]);
    assert_ne!(new_id, prior.id);
    assert_eq!(new.supplier_id, prior.supplier_id);
    assert_eq!(new.items[0].line.product_id, Some(PRODUCT));
    assert_eq!(new.items[0].line.rate, Money::from_minor_units(370));
    assert_eq!(new.items[0].received, 0);
    assert!(new.items[0].receipts().is_empty());
    assert_eq!(fixture.catalog, inventory_before);
    assert_eq!(fixture.purchases.suppliers, suppliers_before);
}
