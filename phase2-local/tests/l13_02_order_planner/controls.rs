use super::*;
use alm_phase2_local::confirmation::PurchaseProduct;

#[test]
fn l13_02_plan_uses_existing_default_for_registered_purchase_products() {
    let mut fixture = ConfirmationFixture::new(RATED, 4, money(2_000)).unwrap();
    fixture.add_product(SECOND, 9, money(2_000)).unwrap();
    for (id, item) in [(RATED, LIME), (SECOND, SODA)] {
        fixture
            .register_purchase_product(PurchaseProduct {
                id,
                item: item.into(),
                buy_rate: money(700),
            })
            .unwrap();
    }

    let plan = fixture.open_order_planner(OWNER).unwrap();

    let low: Vec<_> = plan.rows(true).collect();
    println!("registered product low/out rows: {low:?}");
    assert_eq!(plan.rows(false).count(), 2);
    let [row] = low.as_slice() else {
        panic!("expected only the product under the existing default")
    };
    assert_eq!((row.product, row.reorder_level), (RATED, 5));
    assert_eq!((row.suggested_quantity, row.unit_rate), (11, money(700)));
}

#[test]
fn l13_02_close_leaves_cart_and_orders_unchanged() {
    let (fixture, _) = fixture(0);
    let plan = filled(&fixture);
    let cart_before = plan.clone();

    {
        let mut draft = plan.checkout(OWNER, date()).unwrap();
        draft.items.clear();
    }

    assert_eq!(plan, cart_before);
    assert!(fixture.purchase_orders(OWNER).unwrap().is_empty());
    println!(
        "closed draft: cart unchanged with total {:?}; no PO saved",
        plan.cart_total()
    );
}

#[test]
fn l13_02_save_keeps_a_positive_cart_line_deleted_from_the_draft() {
    let (mut fixture, _) = fixture(0);
    let mut plan = fixture.open_order_planner(OWNER).unwrap();
    assert!(plan.add_item(RATED, Some(0)));
    assert!(plan.add_item(SECOND, None));
    let mut draft = plan.checkout(OWNER, date()).unwrap();
    draft.items.retain(|line| line.item == LIME);

    fixture.save_order_plan(OWNER, &mut plan, &draft).unwrap();

    let order = &fixture.purchase_orders(OWNER).unwrap()[0];
    let lines = saved_readback(order);
    println!("edited draft saved lines={lines:?}; cart={:?}", plan.cart());
    assert_eq!(lines, [(Some(RATED), LIME, 4, 825, 3_300)]);
    let [retained] = plan.cart() else {
        panic!("expected the unsaved cart line")
    };
    assert_eq!((retained.item.as_str(), retained.quantity), (SODA, 11));
}

#[test]
fn l13_02_b_only_quote_is_not_an_a_fallback_or_repriced_on_save() {
    let (mut fixture, [a, b]) = fixture(975);
    assert_eq!(prices(&fixture)[1].rows[0].prices, [None, Some(money(975))]);
    let mut plan = fixture.open_order_planner(OWNER).unwrap();
    let row = plan.rows(true).find(|row| row.product == SECOND).unwrap();
    println!("B-only plan row: {row:?}");
    assert_eq!(row.best_supplier, supplier(b, B));
    assert_eq!(row.unit_rate, money(975));
    assert!(plan.add_item(SECOND, Some(4)));
    assert_cart(
        &plan,
        &[(SECOND, SODA, 4, 975, 3_900, supplier(b, B))],
        3_900,
    );
    let plan_before = plan.clone();
    let expected = [(Some(SECOND), SODA, 4, 975, 3_900)];

    let mut draft = plan.checkout(OWNER, date()).unwrap();
    assert_draft(&draft, supplier(b, B), (&expected, 3_900));
    assert_eq!(plan, plan_before);
    draft.supplier = supplier(a, A);
    assert_draft(&draft, supplier(a, A), (&expected, 3_900));
    assert_eq!(plan, plan_before);
    assert!(fixture.purchase_orders(OWNER).unwrap().is_empty());

    fixture.save_order_plan(OWNER, &mut plan, &draft).unwrap();

    let [order] = fixture.purchase_orders(OWNER).unwrap() else {
        panic!("expected one B-only order")
    };
    let lines = saved_readback(order);
    let total: i64 = lines.iter().map(|line| line.4).sum();
    println!(
        "B-only saved supplier={:?}, lines={lines:?}, total={total}",
        order.supplier_name
    );
    assert_eq!((order.supplier_id, order.supplier_name.as_str()), (a, A));
    assert_eq!(lines, expected);
    assert_eq!(total, 3_900);
    assert_cart(&plan, &[], 0);
}

#[test]
fn l13_02_mixed_positive_suppliers_leave_both_lines_and_require_supplier_on_save() {
    let (mut fixture, [a, b]) = fixture(975);
    let mut plan = filled(&fixture);
    let plan_before = plan.clone();
    let prices_before = prices(&fixture);
    let fifo_before = fifo(&fixture);
    let suppliers_before = fixture.purchase_suppliers(OWNER).unwrap().to_vec();
    assert_cart(
        &plan,
        &[
            (RATED, LIME, 3, 825, 2_475, supplier(a, A)),
            (SECOND, SODA, 4, 975, 3_900, supplier(b, B)),
        ],
        6_375,
    );

    let draft = plan.checkout(OWNER, date()).unwrap();
    let expected = [
        (Some(RATED), LIME, 3, 825, 2_475),
        (Some(SECOND), SODA, 4, 975, 3_900),
    ];
    assert_draft(&draft, None, (&expected, 6_375));
    assert_eq!(plan, plan_before);
    assert!(fixture.purchase_orders(OWNER).unwrap().is_empty());

    let save = fixture.save_order_plan(OWNER, &mut plan, &draft);

    assert_eq!(save, Err(PurchaseError::MissingSupplier));
    assert_eq!(plan, plan_before);
    assert_eq!(plan.cart_total(), Ok(money(6_375)));
    assert!(fixture.purchase_orders(OWNER).unwrap().is_empty());
    assert_eq!(fixture.purchase_suppliers(OWNER).unwrap(), suppliers_before);
    assert_eq!(prices(&fixture), prices_before);
    assert_eq!(fifo(&fixture), fifo_before);
    println!("mixed-positive Save={save:?}; cart and FIFO unchanged; no PO saved");
}

#[test]
fn l13_02_zero_line_blocks_inference_until_removed_and_survives_unsaved_paths() {
    let (mut fixture, [a, b]) = fixture(975);
    let mut plan = filled(&fixture);
    assert!(plan.set_quantity(SECOND, 0));
    assert!(!plan.add_item(SECOND, None));
    let snapshot = plan.clone();
    assert_cart(
        &plan,
        &[
            (RATED, LIME, 3, 825, 2_475, supplier(a, A)),
            (SECOND, SODA, 0, 975, 0, supplier(b, B)),
        ],
        2_475,
    );
    let expected = [(Some(RATED), LIME, 3, 825, 2_475)];

    let mut draft = plan.checkout(OWNER, date()).unwrap();
    assert_draft(&draft, None, (&expected, 2_475));
    let save = fixture.save_order_plan(OWNER, &mut plan, &draft);
    println!("zero-line Save with no supplier: {save:?}");
    assert_eq!(save, Err(PurchaseError::MissingSupplier));
    assert_eq!(plan, snapshot);
    assert!(fixture.purchase_orders(OWNER).unwrap().is_empty());
    let mut without_zero = plan.clone();
    assert!(without_zero.remove_item(SECOND));
    let after_removal = without_zero.checkout(OWNER, date()).unwrap();
    assert_draft(&after_removal, supplier(a, A), (&expected, 2_475));

    draft.supplier = supplier(a, A);
    fixture.save_order_plan(OWNER, &mut plan, &draft).unwrap();
    assert_cart(&plan, &[(SECOND, SODA, 0, 975, 0, supplier(b, B))], 0);
    let empty = Err(OrderPlannerError::EmptyCart);
    assert_eq!(plan.checkout(OWNER, date()), empty);
    assert!(plan.set_quantity(SECOND, -1));
    assert_eq!(plan.checkout(OWNER, date()), empty);
}
