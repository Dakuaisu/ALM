use std::num::NonZeroU32;

use alm_phase2_local::confirmation::{
    ConfirmationFixture, ProductEntryError, ProductPriceInput, PurchaseDraft, PurchaseLine,
    PurchaseOrderSelection, PurchaseOrderStatus, PurchaseProduct, PurchaseReceipt,
    PurchaseSupplier, ReceiveLine, ReceiveRequest, ReceiveResult, SupplierPriceEdit,
};
use alm_phase2_local::{BillDate, Money, ProductId, Role};

const OWNER: Role = Role::Owner;
const A: ProductId = ProductId::new(1);
const B: ProductId = ProductId::new(2);
const A_NAME: &str = "Synthetic A";
const B_NAME: &str = "Synthetic B";
const SUPPLIER: &str = "Synthetic Supplier";
const TWO: NonZeroU32 = NonZeroU32::new(2).expect("two cases");

const fn money(minor: i64) -> Money {
    Money::from_minor_units(minor)
}

fn feb(day: u8) -> BillDate {
    BillDate::new(2025, 2, day).unwrap()
}

fn enter_product(fixture: &mut ConfirmationFixture, id: ProductId, (name, buy): (&str, i64)) {
    let mut entry = fixture.open_product_entry(OWNER, id).unwrap();
    entry.item = name.into();
    entry
        .apply_price(ProductPriceInput::Buy(money(buy)))
        .unwrap();
    entry
        .apply_price(ProductPriceInput::Sell(money(500)))
        .unwrap();
    fixture.save_product_entry(OWNER, &entry).unwrap();
}

#[test]
fn l09_01_catalog_create_and_edit_reject_duplicate_names() {
    // Given an existing named catalog product and an unused PID 2.
    let mut fixture = ConfirmationFixture::new(A, 2, money(500)).unwrap();
    enter_product(&mut fixture, A, (A_NAME, 100));
    let original_a = fixture.open_product_entry(OWNER, A).unwrap();
    let duplicate = ProductEntryError::DuplicateName {
        existing: A_NAME.into(),
    };
    let mut create = fixture.begin_product_entry(OWNER, B).unwrap();

    // When the Owner attempts duplicate creation, PID 2 is still absent; a unique save succeeds.
    create.item = A_NAME.into();
    let denied_create = fixture.save_product_entry(OWNER, &create);
    assert_eq!(denied_create, Err(duplicate.clone()));
    assert_eq!(
        fixture.open_product_entry(OWNER, B),
        Err(ProductEntryError::ProductNotFound)
    );
    create.item = B_NAME.into();
    fixture.save_product_entry(OWNER, &create).unwrap();

    // Then the existing PID 2 cannot be renamed to the exact or normalized duplicate either.
    let original_b = fixture.open_product_entry(OWNER, B).unwrap();
    let mut edit = original_b.clone();
    for name in [A_NAME, "  synthetic a  "] {
        edit.item = name.into();
        let denied_edit = fixture.save_product_entry(OWNER, &edit);
        assert_eq!(denied_edit, Err(duplicate.clone()));
        println!("L09-01 public edit PID 2 to {name:?}: {denied_edit:?}");
    }
    assert_eq!(fixture.open_product_entry(OWNER, A), Ok(original_a));
    assert_eq!(fixture.open_product_entry(OWNER, B), Ok(original_b));
    println!(
        "L09-01 public creation PID 2 as {A_NAME:?}: {denied_create:?}; unique {B_NAME:?} saved"
    );
}

fn product_readback(fixture: &ConfirmationFixture, id: ProductId) -> (String, u32, [i64; 5]) {
    let entry = fixture.open_product_entry(OWNER, id).unwrap();
    let prices = entry.pricing();
    let (average, registered_buy, quote) =
        fixture.purchase_cost(OWNER, id, SUPPLIER).unwrap().unwrap();
    assert_eq!(registered_buy, prices.buy());
    let comparison = fixture.compare_supplier_prices(OWNER, Some(id)).unwrap();
    assert_eq!(comparison.rows[0].prices, [quote]);
    (
        entry.item,
        fixture.product_stock(id).unwrap(),
        [
            average,
            prices.buy(),
            prices.margin(),
            prices.sell(),
            quote.unwrap(),
        ]
        .map(Money::minor_units),
    )
}

#[test]
fn l08_01_pid_two_receives_with_unique_or_same_name_purchase_metadata() {
    // Given one catalog and two independent purchase metadata scenarios; only the latter is seeded.
    for (scenario, second_purchase_name) in
        [("unique", B_NAME), ("trusted duplicate metadata", A_NAME)]
    {
        let mut fixture = ConfirmationFixture::new(A, 2, money(500)).unwrap();
        fixture.add_product(B, 2, money(500)).unwrap();
        let supplier = fixture
            .add_comparison_supplier(OWNER, SUPPLIER)
            .unwrap()
            .unwrap();
        for (id, catalog_name, purchase_name, buy) in [
            (A, A_NAME, A_NAME, 100),
            (B, B_NAME, second_purchase_name, 200),
        ] {
            enter_product(&mut fixture, id, (catalog_name, buy));
            fixture
                .register_purchase_product(PurchaseProduct {
                    id,
                    item: purchase_name.into(),
                    buy_rate: money(buy),
                })
                .unwrap();
            fixture
                .edit_comparison_price(
                    OWNER,
                    SupplierPriceEdit {
                        product: id,
                        supplier: supplier.id,
                        rate: Some(money(buy)),
                    },
                )
                .unwrap();
        }
        let before = [A, B].map(|id| product_readback(&fixture, id));
        assert_eq!(
            before,
            [
                (A_NAME.into(), 2, [100, 100, 400, 500, 100]),
                (B_NAME.into(), 2, [200, 200, 300, 500, 200]),
            ]
        );
        let ordered_line = PurchaseLine {
            item: second_purchase_name.into(),
            product_id: Some(B),
            ordered: TWO,
            rate: money(200),
        };
        let id = fixture
            .save_purchase_order(
                OWNER,
                &PurchaseDraft {
                    supplier: Some(PurchaseSupplier {
                        id: Some(supplier.id),
                        name: SUPPLIER.into(),
                    }),
                    date: Some(feb(10)),
                    note: "Synthetic note".into(),
                    items: vec![ordered_line.clone()],
                },
            )
            .unwrap();
        let selection = PurchaseOrderSelection {
            role: OWNER,
            id,
            number: 1,
        };
        let pending = &fixture.purchase_orders(OWNER).unwrap()[0];
        assert_eq!(pending.items.len(), 1);
        assert_eq!(pending.items[0].line, ordered_line);
        assert!(pending.items[0].receipts().is_empty());
        assert_eq!(
            fixture.view_purchase_order(selection, &[]).unwrap().status,
            PurchaseOrderStatus::Pending
        );

        // When the existing Receive Goods path posts the dated receipt against the saved PO.
        let form = fixture
            .open_purchase_receive_form(OWNER, id, feb(13))
            .unwrap();
        let [entry] = form.rows.as_slice() else {
            panic!("one receive row expected")
        };
        assert_eq!(
            (
                entry.item.as_str(),
                entry.remaining,
                entry.receive_now,
                entry.rate
            ),
            (second_purchase_name, 2, 2, money(200))
        );
        let request = ReceiveRequest {
            role: OWNER,
            id,
            date: Some(feb(12)),
            today: form.received_on,
            lines: &[ReceiveLine {
                quantity: 2,
                rate: money(300),
            }],
        };
        assert_eq!([A, B].map(|id| product_readback(&fixture, id)), before);
        assert_eq!(
            fixture.receive_purchase_order(request),
            Ok(ReceiveResult::Received)
        );

        // Then only PID 2 changes; the receipt is nested on the original line, not matched by name.
        let after = [A, B].map(|id| product_readback(&fixture, id));
        assert_eq!(
            after,
            [
                (A_NAME.into(), 2, [100, 100, 400, 500, 100]),
                (B_NAME.into(), 4, [250, 300, 200, 500, 300]),
            ]
        );
        let [order] = fixture.purchase_orders(OWNER).unwrap() else {
            panic!("one saved PO expected")
        };
        assert_eq!(
            (
                order.id,
                order.supplier_id,
                order.supplier_name.as_str(),
                order.date,
                order.note.as_str()
            ),
            (id, supplier.id, SUPPLIER, feb(10), "Synthetic note")
        );
        let [saved] = order.items.as_slice() else {
            panic!("one saved line expected")
        };
        assert_eq!(
            saved.line,
            PurchaseLine {
                rate: money(300),
                ..ordered_line
            }
        );
        assert_eq!(saved.received, 2);
        assert_eq!(
            saved.receipts(),
            [PurchaseReceipt {
                date: feb(12),
                quantity: TWO,
                rate: money(300)
            }]
        );
        assert_eq!(
            fixture.purchase_movements(OWNER, id).unwrap(),
            [(feb(12), 2)]
        );
        let view = fixture.view_purchase_order(selection, &[]).unwrap();
        assert_eq!(
            (view.number.as_str(), view.status, view.lines[0].remaining),
            ("PO-001", PurchaseOrderStatus::Completed, 0)
        );
        println!(
            "L08-01 {scenario}: catalog names={A_NAME:?}/{B_NAME:?}, registered names={A_NAME:?}/{second_purchase_name:?}; PID 1/2 (catalog name, stock, [avg, buy, margin, sell, quote] minor) before={before:?}; after={after:?}; {} {:?}, PID 2 nested receipts={:?}",
            view.number,
            view.status,
            saved.receipts()
        );
    }
    println!(
        "Public receipt readback does not expose legacy _v/_m/_d stamps or the sync watermark."
    );
}
