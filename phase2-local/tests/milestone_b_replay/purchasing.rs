use alm_phase2_local::confirmation::order_planner::OrderPlannerError;
use alm_phase2_local::confirmation::{
    ConfirmationFixture, PurchaseError, PurchaseLine, PurchaseOrderSelection, PurchaseOrderStatus,
    PurchaseOrderView, PurchaseProduct, PurchaseReceipt, PurchaseSupplier, ReceiveLine,
    ReceiveRequest, ReceiveResult, StockLots, SupplierPriceEdit, SupplierPriceError,
};

use super::{A, B, ITEM_A, ITEM_B, MANAGER, OWNER, feb, fifo, fifo_value, money, qty, stock};

const WINNER: &str = "Synthetic Winning Supplier";

pub(super) fn save_planned_order(fixture: &mut ConfirmationFixture) -> PurchaseOrderSelection {
    let [other, winner] = ["Synthetic Other Supplier", WINNER].map(|name| {
        fixture
            .add_comparison_supplier(OWNER, name)
            .unwrap()
            .unwrap()
            .id
    });
    let products = [(A, ITEM_A, 4, 12, 100, 160), (B, ITEM_B, 3, 9, 180, 260)];
    for (product, item, level, _, low, high) in products {
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
        for (supplier, rate) in [(other, high), (winner, low)] {
            let edit = SupplierPriceEdit {
                product,
                supplier,
                rate: Some(money(rate)),
            };
            fixture.edit_comparison_price(OWNER, edit).unwrap();
        }
        let prices = fixture
            .compare_supplier_prices(OWNER, Some(product))
            .unwrap();
        assert_eq!(prices.rows[0].prices, [Some(money(high)), Some(money(low))]);
    }
    assert_eq!(
        fixture.compare_supplier_prices(MANAGER, Some(A)),
        Err(SupplierPriceError::AccessDenied)
    );
    assert_eq!(
        fixture.open_order_planner(MANAGER),
        Err(OrderPlannerError::AccessDenied)
    );

    let supplier = Some(PurchaseSupplier {
        id: Some(winner),
        name: WINNER.into(),
    });
    let mut planner = fixture.open_order_planner(OWNER).unwrap();
    let rows: Vec<_> = planner.rows(true).collect();
    assert_eq!(rows.len(), products.len());
    for (row, (id, item, level, suggested, low, _)) in rows.into_iter().zip(products) {
        assert_eq!((row.product, row.item.as_str()), (id, item));
        assert_eq!(
            (row.on_hand, row.reorder_level, row.suggested_quantity),
            (0, level, i64::from(suggested))
        );
        assert_eq!((&row.best_supplier, row.unit_rate), (&supplier, money(low)));
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
        products.map(|(id, name, _, count, rate, _)| PurchaseLine {
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
    assert_progress(&pending, (PurchaseOrderStatus::Pending, [0, 0], [12, 9]));
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
    println!(
        "Feb 10: {} planned {planned:?}, total={total}; Save: stock={:?}, FIFO={}, no receipts or register rows",
        order.supplier_name,
        stock(fixture),
        fifo_value(&fifo(fixture))
    );
    selection
}

pub(super) fn receive_partials(
    fixture: &mut ConfirmationFixture,
    selection: PurchaseOrderSelection,
) -> PurchaseOrderView {
    // When Feb 12 is entered first, FIFO must still order the layers by receipt date.
    let [_, final_view] = [
        (12, [(5, 130), (4, 210)], [7, 5], [5, 4], 1_490),
        (11, [(3, 110), (1, 190)], [4, 4], [8, 5], 2_010),
    ]
    .map(|(day, inputs, outstanding, on_hand, value)| {
        let lines = inputs.map(|(quantity, rate)| ReceiveLine {
            quantity,
            rate: money(rate),
        });
        let request = ReceiveRequest {
            role: OWNER,
            id: selection.id,
            date: Some(feb(day)),
            today: feb(12),
            lines: &lines,
        };
        assert_eq!(
            fixture.receive_purchase_order(ReceiveRequest {
                role: MANAGER,
                ..request
            }),
            Err(PurchaseError::AccessDenied)
        );
        assert_eq!(
            fixture.receive_purchase_order(request),
            Ok(ReceiveResult::Received)
        );
        let view = fixture.view_purchase_order(selection, &[]).unwrap();
        assert_progress(
            &view,
            (PurchaseOrderStatus::PartiallyReceived, on_hand, outstanding),
        );
        let orders = fixture.purchase_orders(OWNER).unwrap();
        for (saved, (quantity, rate)) in orders[0].items.iter().zip(inputs) {
            assert_eq!(
                saved.receipts().last(),
                Some(&PurchaseReceipt {
                    date: feb(day),
                    quantity: qty(quantity),
                    rate: money(rate),
                })
            );
        }
        assert_eq!(stock(fixture), on_hand);
        let fifo_total = fifo_value(&fifo(fixture));
        assert_eq!(fifo_total, value);
        let remaining: Vec<_> = view.lines.iter().map(|line| line.remaining).collect();
        println!(
            "Feb {day} receipt {inputs:?}: {:?}, outstanding={remaining:?}, FIFO={fifo_total}",
            view.status
        );
        view
    });
    final_view
}

fn assert_empty_inventory(fixture: &ConfirmationFixture) {
    assert_eq!(stock(fixture), [0, 0]);
    assert_eq!(fixture.posted_count(), 0);
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

fn assert_progress(
    view: &PurchaseOrderView,
    (status, received, outstanding): (PurchaseOrderStatus, [u32; 2], [u32; 2]),
) {
    assert_eq!(view.status, status);
    let progress = view.lines.iter().map(|row| {
        (
            row.item.as_str(),
            row.ordered,
            row.received,
            row.cancelled,
            row.remaining,
        )
    });
    assert!(
        progress.eq([
            (ITEM_A, 12, received[0], 0, outstanding[0]),
            (ITEM_B, 9, received[1], 0, outstanding[1]),
        ]),
        "Unexpected PO progress: {view:?}"
    );
}
