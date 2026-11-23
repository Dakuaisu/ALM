use alm_core::confirmation::order_planner::OrderPlannerError;
use alm_core::confirmation::{
    ConfirmationFixture, PurchaseError, PurchaseLine, PurchaseOrderSelection, PurchaseOrderStatus,
    PurchaseProduct, PurchaseSupplier, StockLots, SupplierPriceEdit, SupplierPriceError,
};

use super::super::{
    A, B, ITEM_A, ITEM_B, MANAGER, OWNER, feb, fifo, fifo_value, money, qty, stock,
};

const WINNER: &str = "Synthetic Winning Supplier";
const OTHER: &str = "Synthetic Other Supplier";

pub(crate) fn save_planned_order(fixture: &mut ConfirmationFixture) -> PurchaseOrderSelection {
    assert_eq!(
        fixture.add_comparison_supplier(MANAGER, OTHER),
        Err(SupplierPriceError::AccessDenied)
    );
    let [other, winner] = [OTHER, WINNER].map(|name| {
        fixture
            .add_comparison_supplier(OWNER, name)
            .unwrap()
            .unwrap()
            .id
    });
    let products = [
        (A, ITEM_A, 4, 12, (160, 200, 100)),
        (B, ITEM_B, 3, 9, (260, 280, 180)),
    ];
    for (product, item, level, _, (other_rate, initial, _)) in products {
        let mut form = fixture.open_product_entry(OWNER, product).unwrap();
        form.item = item.into();
        form.reorder_level = Some(level);
        fixture.save_product_entry(OWNER, &form).unwrap();
        fixture
            .register_purchase_product(PurchaseProduct {
                id: product,
                item: item.into(),
                buy_rate: money(300),
            })
            .unwrap();
        for (supplier, rate) in [(other, other_rate), (winner, initial)] {
            let edit = SupplierPriceEdit {
                product,
                supplier,
                rate: Some(money(rate)),
            };
            fixture.edit_comparison_price(OWNER, edit).unwrap();
        }
    }
    assert_eq!(
        fixture.compare_supplier_prices(MANAGER, Some(A)),
        Err(SupplierPriceError::AccessDenied)
    );
    assert_eq!(
        fixture.open_order_planner(MANAGER),
        Err(OrderPlannerError::AccessDenied)
    );
    assert_empty_inventory(fixture);

    let initial_plan = fixture.open_order_planner(OWNER).unwrap();
    for (product, item, _, _, (other_rate, initial, revised)) in products {
        let comparison = fixture
            .compare_supplier_prices(OWNER, Some(product))
            .unwrap();
        let [before] = comparison.rows.as_slice() else {
            panic!("Expected one comparison row for {item}");
        };
        assert_eq!(before.product, product);
        assert_eq!(
            before.prices,
            [Some(money(other_rate)), Some(money(initial))]
        );
        assert_eq!(before.best, Some((other, money(other_rate))));
        let row = initial_plan
            .rows(true)
            .find(|row| row.product == product)
            .unwrap();
        assert_eq!(row.unit_rate, money(other_rate));
        assert_eq!(row.best_supplier.as_ref().unwrap().id, Some(other));

        let edit = SupplierPriceEdit {
            product,
            supplier: winner,
            rate: Some(money(revised)),
        };
        assert_eq!(
            fixture.edit_comparison_price(MANAGER, edit),
            Err(SupplierPriceError::AccessDenied)
        );
        assert_eq!(
            fixture.compare_supplier_prices(OWNER, Some(product)),
            Ok(comparison)
        );
        fixture.edit_comparison_price(OWNER, edit).unwrap();
        let comparison = fixture
            .compare_supplier_prices(OWNER, Some(product))
            .unwrap();
        let [after] = comparison.rows.as_slice() else {
            panic!("Expected one revised comparison row for {item}");
        };
        assert_eq!(after.product, product);
        assert_eq!(
            after.prices,
            [Some(money(other_rate)), Some(money(revised))]
        );
        assert_eq!(after.best, Some((winner, money(revised))));
        println!(
            "Quote {item}: {WINNER} {initial} -> {revised}; best {OTHER} @{other_rate} -> {WINNER} @{revised}"
        );
    }
    assert_empty_inventory(fixture);

    let supplier = Some(PurchaseSupplier {
        id: Some(winner),
        name: WINNER.into(),
    });
    let mut planner = fixture.open_order_planner(OWNER).unwrap();
    let rows: Vec<_> = planner.rows(true).collect();
    assert_eq!(rows.len(), products.len());
    for (row, (id, item, level, suggested, (_, _, revised))) in rows.into_iter().zip(products) {
        assert_eq!((row.product, row.item.as_str()), (id, item));
        assert_eq!(
            (row.on_hand, row.reorder_level, row.suggested_quantity),
            (0, level, i64::from(suggested))
        );
        assert_eq!(
            (&row.best_supplier, row.unit_rate),
            (&supplier, money(revised))
        );
    }
    for id in [A, B] {
        assert!(planner.add_item(id, None));
    }
    let total = planner.cart_total().unwrap().minor_units();
    assert_eq!(total, 2_820);
    assert_eq!(
        planner.checkout(MANAGER, feb(10)),
        Err(OrderPlannerError::AccessDenied)
    );
    let draft = planner.checkout(OWNER, feb(10)).unwrap();
    assert_eq!((draft.date, &draft.supplier), (Some(feb(10)), &supplier));
    assert_eq!(
        draft.items,
        products.map(|(id, name, _, count, (_, _, rate))| PurchaseLine {
            item: name.into(),
            product_id: Some(id),
            ordered: qty(count),
            rate: money(rate),
        })
    );
    assert_eq!(
        fixture.save_order_plan(MANAGER, &mut planner, &draft),
        Err(PurchaseError::AccessDenied)
    );
    assert!(fixture.purchase_orders(OWNER).unwrap().is_empty());
    assert_empty_inventory(fixture);
    let planned: Vec<_> = draft
        .items
        .iter()
        .map(|line| {
            (
                line.item.as_str(),
                line.ordered.get(),
                line.rate.minor_units(),
            )
        })
        .collect();
    println!("Live plan selected {WINNER}: {planned:?}, total={total}; stock=[0, 0], FIFO=0");

    let id = fixture
        .save_order_plan(OWNER, &mut planner, &draft)
        .unwrap();
    assert!(planner.cart().is_empty());
    let selection = PurchaseOrderSelection {
        role: OWNER,
        id,
        number: 1,
    };
    let manager = PurchaseOrderSelection {
        role: MANAGER,
        ..selection
    };
    assert_eq!(
        fixture.view_purchase_order(manager, &[]),
        Err(PurchaseError::AccessDenied)
    );
    let [order] = fixture.purchase_orders(OWNER).unwrap() else {
        panic!("Save should create exactly one PO");
    };
    assert_eq!(
        (order.id, order.date, order.supplier_id),
        (id, feb(10), winner)
    );
    assert_eq!(order.supplier_name, WINNER);
    assert!(order.items.iter().map(|line| &line.line).eq(&draft.items));
    assert!(order.items.iter().all(|line| line.receipts().is_empty()));
    let pending = fixture.view_purchase_order(selection, &[]).unwrap();
    assert_eq!(pending.status, PurchaseOrderStatus::Pending);
    assert!(
        pending
            .lines
            .iter()
            .map(|line| (line.ordered, line.received, line.cancelled, line.remaining))
            .eq([(12, 0, 0, 12), (9, 0, 0, 9)])
    );
    assert_empty_inventory(fixture);
    println!(
        "{} saved {}: {:?}, stock={:?}, FIFO={}, no receipts or register rows",
        pending.date,
        pending.number,
        pending.status,
        stock(fixture),
        fifo_value(&fifo(fixture))
    );
    selection
}

fn assert_empty_inventory(fixture: &ConfirmationFixture) {
    assert_eq!(stock(fixture), [0, 0]);
    assert_eq!(fixture.posted_count(), 0);
    assert_eq!(
        (fixture.ledger(), fixture.received_ledger()),
        (money(0), money(0))
    );
    for lots in fifo(fixture) {
        assert_eq!(
            lots,
            StockLots {
                on_hand: 0,
                rows: vec![],
                total_value: Some(money(0))
            }
        );
    }
    for day in 10..=14 {
        let rows = fixture
            .open_daily_movement_register(OWNER, feb(day))
            .unwrap();
        assert!(
            rows.is_empty(),
            "Feb {day} register before receiving: {rows:?}"
        );
    }
}
