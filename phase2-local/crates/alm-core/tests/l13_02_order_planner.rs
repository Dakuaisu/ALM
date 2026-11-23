use alm_core::confirmation::order_planner::{OrderPlanner, OrderPlannerError};
use alm_core::confirmation::{
    ConfirmationFixture, ProductPriceInput, PurchaseDraft, PurchaseError, PurchaseLine,
    PurchaseOrder, PurchaseOrderSelection, PurchaseOrderStatus, PurchaseSupplier,
    PurchaseSupplierId, StockLots, SupplierPriceComparison, SupplierPriceEdit,
};
use alm_core::{BillDate, Money, ProductId, Role};

#[path = "l13_02_order_planner/controls.rs"]
mod controls;

const OWNER: Role = Role::Owner;
const RATED: ProductId = ProductId::new(1);
const SECOND: ProductId = ProductId::new(2);
const LIME: &str = "Synthetic Lime";
const SODA: &str = "Synthetic Soda";
const A: &str = "Synthetic Supplier A";
const B: &str = "Synthetic Supplier B";

const fn money(value: i64) -> Money {
    Money::from_minor_units(value)
}

fn date() -> BillDate {
    BillDate::new(2024, 2, 9).unwrap()
}

fn fixture(second_b_quote: i64) -> (ConfirmationFixture, [PurchaseSupplierId; 2]) {
    let opening = [(1, money(610)), (1, money(720))];
    let mut fixture =
        ConfirmationFixture::with_ordered_lots(RATED, &opening, money(2_000)).unwrap();
    fixture.add_product(SECOND, 4, money(2_000)).unwrap();
    for (id, item, reorder_level, buy) in [(RATED, LIME, 2, 700), (SECOND, SODA, 5, 1_325)] {
        let mut form = fixture.open_product_entry(OWNER, id).unwrap();
        form.item = item.into();
        form.reorder_level = Some(reorder_level);
        form.apply_price(ProductPriceInput::Buy(money(buy)))
            .unwrap();
        fixture.save_product_entry(OWNER, &form).unwrap();
    }
    let [a, b] = [A, B].map(|name| {
        fixture
            .add_comparison_supplier(OWNER, name)
            .unwrap()
            .unwrap()
            .id
    });
    let second_a_quote = if second_b_quote == 0 { -100 } else { 0 };
    for (product, supplier, rate) in [
        (RATED, b, 1_100),
        (RATED, a, 825),
        (SECOND, a, second_a_quote),
        (SECOND, b, second_b_quote),
    ] {
        fixture
            .edit_comparison_price(
                OWNER,
                SupplierPriceEdit {
                    product,
                    supplier,
                    rate: Some(money(rate)),
                },
            )
            .unwrap();
    }
    (fixture, [a, b])
}

fn supplier(id: PurchaseSupplierId, name: &str) -> Option<PurchaseSupplier> {
    Some(PurchaseSupplier {
        id: Some(id),
        name: name.into(),
    })
}

fn filled(fixture: &ConfirmationFixture) -> OrderPlanner {
    let mut plan = fixture.open_order_planner(OWNER).unwrap();
    assert!(plan.add_item(RATED, Some(3)));
    assert!(plan.add_item(SECOND, Some(4)));
    plan
}

type LineReadback<'a> = (Option<ProductId>, &'a str, u32, i64, i64);
type CartReadback<'a> = (ProductId, &'a str, i64, i64, i64, Option<PurchaseSupplier>);

fn assert_cart(plan: &OrderPlanner, expected: &[CartReadback<'_>], expected_total: i64) {
    let rows: Vec<_> = plan
        .cart()
        .iter()
        .map(|line| {
            let qty = line.quantity;
            let rate = line.unit_rate.minor_units();
            (
                line.product,
                line.item.as_str(),
                qty,
                rate,
                qty * rate,
                line.best_supplier.clone(),
            )
        })
        .collect();
    let total = plan.cart_total().unwrap().minor_units();
    println!("cart rows={rows:?}, total={total}");
    assert_eq!(rows, expected);
    assert_eq!(total, expected_total);
}

fn line_readback(line: &PurchaseLine) -> LineReadback<'_> {
    let product = line.product_id;
    let item = line.item.as_str();
    let qty = line.ordered.get();
    let rate = line.rate.minor_units();
    (product, item, qty, rate, i64::from(qty) * rate)
}

fn saved_readback(order: &PurchaseOrder) -> Vec<LineReadback<'_>> {
    order
        .items
        .iter()
        .map(|line| line_readback(&line.line))
        .collect()
}

fn prices(fixture: &ConfirmationFixture) -> [SupplierPriceComparison; 2] {
    [RATED, SECOND].map(|id| fixture.compare_supplier_prices(OWNER, Some(id)).unwrap())
}

fn fifo(fixture: &ConfirmationFixture) -> [StockLots; 2] {
    [RATED, SECOND].map(|id| fixture.open_stock_lots(OWNER, id).unwrap())
}

fn assert_draft(
    draft: &PurchaseDraft,
    expected_supplier: Option<PurchaseSupplier>,
    (expected_lines, expected_total): (&[LineReadback<'_>], i64),
) {
    let lines: Vec<_> = draft.items.iter().map(line_readback).collect();
    let total: i64 = lines.iter().map(|line| line.4).sum();
    println!(
        "draft supplier={:?}, date={:?}, lines={lines:?}, total={total}",
        draft.supplier, draft.date
    );
    assert_eq!(draft.supplier, expected_supplier);
    assert_eq!(draft.date, Some(date()));
    assert_eq!(draft.note, "");
    assert_eq!(lines, expected_lines);
    assert_eq!(total, expected_total);
}

#[test]
fn l13_02_live_minimum_and_buy_fallback_survive_detached_override_and_save() {
    let (mut fixture, [a, b]) = fixture(0);
    let prices_before = prices(&fixture);
    assert_eq!(
        prices_before[0].rows[0].prices,
        [Some(money(825)), Some(money(1_100))]
    );
    assert_eq!(prices_before[1].rows[0].prices, [Some(money(-100)), None]);
    let fifo_before = fifo(&fixture);
    println!("FIFO before checkout: {fifo_before:?}");
    let lime_layers: Vec<_> = fifo_before[0]
        .rows
        .iter()
        .map(|row| (row.remaining.get(), row.unit_cost, row.value))
        .collect();
    assert_eq!(
        lime_layers,
        [
            (1, Some(money(610)), Some(money(610))),
            (1, Some(money(720)), Some(money(720)))
        ]
    );
    assert_eq!(fifo_before[0].total_value, Some(money(1_330)));
    let mut plan = filled(&fixture);
    let rows: Vec<_> = plan.rows(true).collect();
    assert_eq!(rows.len(), 2);
    for (row, (id, item, stock, level, suggested, rate, best)) in rows.into_iter().zip([
        (RATED, LIME, 2, 2, 4, 825, Some(A)),
        (SECOND, SODA, 4, 5, 11, 1_325, None),
    ]) {
        println!("plan row: {row:?}");
        assert_eq!((row.product, row.item.as_str()), (id, item));
        assert_eq!(
            (row.on_hand, row.reorder_level, row.suggested_quantity),
            (stock, level, suggested)
        );
        assert_eq!(row.unit_rate, money(rate));
        assert_eq!(row.best_supplier.as_ref().map(|s| s.name.as_str()), best);
    }
    let expected = [
        (Some(RATED), LIME, 3, 825, 2_475),
        (Some(SECOND), SODA, 4, 1_325, 5_300),
    ];
    assert_cart(
        &plan,
        &[
            (RATED, LIME, 3, 825, 2_475, supplier(a, A)),
            (SECOND, SODA, 4, 1_325, 5_300, None),
        ],
        7_775,
    );
    let plan_before = plan.clone();

    let mut draft = plan.checkout(OWNER, date()).unwrap();
    assert_draft(&draft, supplier(a, A), (&expected, 7_775));
    assert_eq!(plan, plan_before);
    assert!(fixture.purchase_orders(OWNER).unwrap().is_empty());
    draft.supplier = supplier(b, B);
    assert_draft(&draft, supplier(b, B), (&expected, 7_775));
    assert_eq!(plan, plan_before);
    assert!(fixture.purchase_orders(OWNER).unwrap().is_empty());
    let fifo_at_draft = fifo(&fixture);
    println!("FIFO with detached draft: {fifo_at_draft:?}");
    assert_eq!(fifo_at_draft, fifo_before);

    let saved = fixture.save_order_plan(OWNER, &mut plan, &draft).unwrap();
    let [order] = fixture.purchase_orders(OWNER).unwrap() else {
        panic!("expected one saved order")
    };
    let lines = saved_readback(order);
    let total: i64 = lines.iter().map(|line| line.4).sum();
    println!("saved={order:?}, lines={lines:?}, total={total}");
    assert_eq!(
        (order.id, order.supplier_id, order.date),
        (saved, b, date())
    );
    assert_eq!(order.supplier_name, B);
    assert_eq!(order.note, "");
    assert_eq!(lines, expected);
    assert_eq!(total, 7_775);
    for line in &order.items {
        assert_eq!(line.received, 0);
        assert!(line.receipts().is_empty());
    }
    let selection = PurchaseOrderSelection {
        role: OWNER,
        id: saved,
        number: 1,
    };
    let progress = fixture.view_purchase_order(selection, &[]).unwrap();
    println!("saved progress: {progress:?}");
    assert_eq!(progress.status, PurchaseOrderStatus::Pending);
    let counts: Vec<_> = progress
        .lines
        .iter()
        .map(|line| {
            (
                line.item.as_str(),
                line.ordered,
                line.received,
                line.cancelled,
                line.remaining,
            )
        })
        .collect();
    assert_eq!(counts, [(LIME, 3, 0, 0, 3), (SODA, 4, 0, 0, 4)]);
    assert_cart(&plan, &[], 0);
    let stock_after = [RATED, SECOND].map(|id| fixture.product_stock(id));
    let fifo_after = fifo(&fixture);
    println!("FIFO after explicit Save: {fifo_after:?}");
    assert_eq!(fifo_after, fifo_before);
    let prices_after = prices(&fixture);
    println!("after explicit Save: stock={stock_after:?}; quotes and buy={prices_after:?}");
    assert_eq!(stock_after, [Some(2), Some(4)]);
    assert_eq!(prices_after, prices_before);
}
