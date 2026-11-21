use super::super::{
    BillDate, PurchaseDraft, PurchaseLine, PurchaseProduct, PurchaseReceipt, PurchaseSupplier,
    ReceiveLine, ReceiveRequest, ReceiveResult,
};
use super::*;
use std::num::NonZeroU32;

fn money(amount: i64) -> Money {
    Money::from_minor_units(amount)
}

#[test]
fn l09_05_rename_preserves_partial_receipt_and_target_quotes() {
    let product = ProductId::new(1);
    let moved_product = ProductId::new(2);
    let zero_product = ProductId::new(3);
    let order_date = BillDate::new(2025, 1, 2).unwrap();
    let receipt_date = BillDate::new(2025, 1, 3).unwrap();
    let mut fixture = ConfirmationFixture::with_ordered_lots(
        product,
        &[(10, Money::from_minor_units(100))],
        Money::from_minor_units(200),
    )
    .unwrap();
    for (id, item) in [
        (product, "Synthetic item"),
        (moved_product, "Moved"),
        (zero_product, "Zero"),
    ] {
        if id != product {
            fixture
                .add_product(id, 0, Money::from_minor_units(200))
                .unwrap();
        }
        fixture
            .register_purchase_product(PurchaseProduct {
                id,
                item: item.into(),
                buy_rate: Money::from_minor_units(100),
            })
            .unwrap();
    }
    let target = fixture
        .add_comparison_supplier(Role::Owner, "New")
        .unwrap()
        .unwrap();
    let other = fixture
        .add_comparison_supplier(Role::Owner, "Other")
        .unwrap()
        .unwrap();
    let order = fixture
        .save_purchase_order(
            Role::Owner,
            &PurchaseDraft {
                supplier: Some(PurchaseSupplier {
                    id: None,
                    name: "Old".into(),
                }),
                date: Some(order_date),
                note: String::new(),
                items: vec![PurchaseLine {
                    item: "Synthetic item".into(),
                    product_id: Some(product),
                    ordered: NonZeroU32::new(4).unwrap(),
                    rate: Money::from_minor_units(100),
                }],
            },
        )
        .unwrap();
    assert_eq!(
        fixture.receive_purchase_order(ReceiveRequest {
            role: Role::Owner,
            id: order,
            date: Some(receipt_date),
            today: receipt_date,
            lines: &[ReceiveLine {
                quantity: 2,
                rate: Money::from_minor_units(120)
            }],
        }),
        Ok(ReceiveResult::Received)
    );
    let supplier = fixture.purchase_orders(Role::Owner).unwrap()[order.0].supplier_id;
    let draft = PurchaseDraft {
        supplier: Some(PurchaseSupplier {
            id: None,
            name: "New".into(),
        }),
        date: Some(order_date),
        note: String::new(),
        items: vec![fixture.purchases.orders[order.0].items[0].line.clone()],
    };
    let exact = fixture.save_purchase_order(Role::Owner, &draft).unwrap();
    let different_case = fixture.save_purchase_order(Role::Owner, &draft).unwrap();
    fixture.purchases.orders[exact.0].supplier_name = "Old".into();
    fixture.purchases.orders[different_case.0].supplier_id = supplier;
    fixture.purchases.orders[different_case.0].supplier_name = "old".into();
    for (id, supplier_id, rate) in [
        (product, target.id, 90),
        (product, other.id, 110),
        (moved_product, supplier, 70),
        (moved_product, other.id, 75),
        (zero_product, supplier, 80),
    ] {
        fixture
            .edit_comparison_price(
                Role::Owner,
                SupplierPriceEdit {
                    product: id,
                    supplier: supplier_id,
                    rate: Some(Money::from_minor_units(rate)),
                },
            )
            .unwrap();
    }
    fixture
        .purchases
        .quotes
        .get_mut(&zero_product.value())
        .unwrap()
        .0
        .push(("New".into(), Money::from_minor_units(0)));
    let before = fixture.clone();
    let trace = |stage: &str, state: &ConfirmationFixture| {
        let suppliers = state.purchase_suppliers(Role::Owner).unwrap();
        let orders = state.purchase_orders(Role::Owner).unwrap();
        let form = state
            .open_purchase_receive_form(Role::Owner, order, receipt_date)
            .unwrap();
        let lots = state.open_stock_lots(Role::Owner, product).unwrap();
        println!(
            "{stage} suppliers={suppliers:?} orders={orders:?} receive={form:?} fifo={lots:?}"
        );
        for id in [product, moved_product, zero_product] {
            println!(
                "{stage} product={id:?} quotes={:?}",
                state.purchases.product_quotes(id)
            );
        }
    };
    trace("before", &fixture);
    assert_eq!(
        fixture.purchases.product_quotes(product).0,
        vec![
            ("Old".into(), Money::from_minor_units(120)),
            ("New".into(), Money::from_minor_units(90)),
            ("Other".into(), Money::from_minor_units(110)),
        ]
    );
    let suppliers = fixture.purchase_suppliers(Role::Owner).unwrap();
    assert_eq!(suppliers[supplier.0].name, "Old");
    assert_eq!(suppliers[target.id.0].name, "New");
    assert_eq!(fixture.purchases.orders[order.0].supplier_name, "Old");
    assert_eq!(
        fixture
            .open_purchase_receive_form(Role::Owner, order, receipt_date)
            .unwrap()
            .supplier_name,
        "Old"
    );
    assert_eq!(
        fixture.rename_purchase_supplier(Role::Manager, supplier, "New"),
        Err(SupplierPriceError::AccessDenied)
    );
    assert_eq!(fixture, before);
    assert_eq!(
        fixture.rename_purchase_supplier(Role::Owner, supplier, "New"),
        Ok(true)
    );
    trace("after", &fixture);

    let renamed = &fixture.purchase_orders(Role::Owner).unwrap()[order.0];
    let mut expected_order = before.purchase_orders(Role::Owner).unwrap()[order.0].clone();
    expected_order.supplier_name = "New".into();
    assert_eq!(*renamed, expected_order);
    assert_eq!(renamed.id, order);
    assert_eq!(renamed.supplier_id, supplier);
    assert_eq!(renamed.supplier_name, "New");
    assert_eq!(renamed.date, order_date);
    assert_eq!(renamed.items[0].line.ordered.get(), 4);
    assert_eq!(renamed.items[0].received, 2);
    assert_eq!(renamed.items[0].remaining(), 2);
    assert_eq!(
        renamed.items[0].receipts(),
        &[PurchaseReceipt {
            date: receipt_date,
            quantity: NonZeroU32::new(2).unwrap(),
            rate: Money::from_minor_units(120),
        }]
    );
    assert_eq!(fixture.purchases.orders[exact.0].supplier_name, "New");
    assert_eq!(fixture.purchases.orders[exact.0].supplier_id, target.id);
    assert_eq!(
        fixture.purchases.orders[different_case.0],
        before.purchases.orders[different_case.0]
    );
    let suppliers = fixture.purchase_suppliers(Role::Owner).unwrap();
    assert_eq!(suppliers[supplier.0].name, "New");
    assert_eq!(suppliers[target.id.0].name, "New");
    let form = fixture
        .open_purchase_receive_form(Role::Owner, order, receipt_date)
        .unwrap();
    assert_eq!((form.id, form.supplier_name.as_str()), (order, "New"));
    assert_eq!(form.rows[0].ordered.get(), 4);
    assert_eq!(form.rows[0].received, 2);
    assert_eq!(form.rows[0].remaining, 2);
    let lots = fixture.open_stock_lots(Role::Owner, product).unwrap();
    assert_eq!(lots, before.open_stock_lots(Role::Owner, product).unwrap());
    assert_eq!(fixture.product_stock(product), Some(12));
    assert_eq!(lots.on_hand, 12);
    assert_eq!(lots.total_value, Some(money(1240)));
    assert_eq!(
        lots.rows
            .iter()
            .map(|row| (row.date, row.remaining.get(), row.unit_cost, row.value))
            .collect::<Vec<_>>(),
        vec![
            (None, 10, Some(money(100)), Some(money(1000))),
            (Some(receipt_date), 2, Some(money(120)), Some(money(240))),
        ]
    );
    assert_eq!(fixture.purchases.costs, before.purchases.costs);
    for (id, expected) in [
        (product, vec![("New", 90), ("Other", 110)]),
        (moved_product, vec![("Other", 75), ("New", 70)]),
        (zero_product, vec![("New", 0)]),
    ] {
        let quotes = fixture.purchases.product_quotes(id);
        assert_eq!(
            quotes.0,
            expected
                .into_iter()
                .map(|(name, rate)| (name.to_owned(), Money::from_minor_units(rate)))
                .collect::<Vec<_>>()
        );
    }
    let comparison = fixture
        .compare_supplier_prices(Role::Owner, Some(product))
        .unwrap();
    assert_eq!(
        comparison
            .suppliers
            .iter()
            .map(|row| (row.id, row.name.as_str()))
            .collect::<Vec<_>>(),
        vec![(target.id, "New"), (other.id, "Other")]
    );
    assert_eq!(
        comparison.rows[0].prices,
        [Some(money(90)), Some(money(110))]
    );
}
