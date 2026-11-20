use alm_phase2_local::confirmation::order_planner::{OrderCartLine, OrderPlanner};
use alm_phase2_local::confirmation::{
    ConfirmationFixture, ProductPriceInput, PurchaseOrder, SupplierPriceComparison,
};
use alm_phase2_local::{Money, ProductId, Role};

const OWNER: Role = Role::Owner;
const MANAGER: Role = Role::Manager;
const PRE_ADDED: ProductId = ProductId::new(1);
const MISSING: ProductId = ProductId::new(99);
const GROUP_FIRST: ProductId = ProductId::new(2);
const HIDDEN: ProductId = ProductId::new(3);
const GROUP_LAST: ProductId = ProductId::new(4);
const GROUP_VISIBLE_ABOVE_THRESHOLD: ProductId = ProductId::new(5);
const SUPPLIER: &str = "Group Fixture Supplier";

fn money(value: i64) -> Money {
    Money::from_minor_units(value)
}

fn fixture() -> ConfirmationFixture {
    let mut fixture =
        ConfirmationFixture::with_ordered_lots(PRE_ADDED, &[(4, money(100))], money(500)).unwrap();
    for (product, item, stock, reorder, buy) in [
        (PRE_ADDED, "Synthetic Existing", 4, 5, 110),
        (GROUP_FIRST, "Synthetic First", 1, 3, 210),
        (HIDDEN, "Synthetic Hidden", 20, 2, 310),
        (GROUP_LAST, "Synthetic Last", 2, 4, 410),
        (
            GROUP_VISIBLE_ABOVE_THRESHOLD,
            "Synthetic Visible High",
            8,
            3,
            510,
        ),
    ] {
        if product != PRE_ADDED {
            fixture.add_product(product, stock, money(500)).unwrap();
        }
        let mut entry = fixture.open_product_entry(OWNER, product).unwrap();
        entry.item = item.into();
        entry.reorder_level = Some(reorder);
        entry
            .apply_price(ProductPriceInput::Buy(money(buy)))
            .unwrap();
        fixture.save_product_entry(OWNER, &entry).unwrap();
        fixture
            .register_purchase_product(alm_phase2_local::confirmation::PurchaseProduct {
                id: product,
                item: item.into(),
                buy_rate: money(buy),
            })
            .unwrap();
    }
    let supplier = fixture
        .add_comparison_supplier(OWNER, SUPPLIER)
        .unwrap()
        .unwrap();
    fixture
        .edit_comparison_price(
            OWNER,
            alm_phase2_local::confirmation::SupplierPriceEdit {
                product: GROUP_VISIBLE_ABOVE_THRESHOLD,
                supplier: supplier.id,
                rate: Some(money(475)),
            },
        )
        .unwrap();
    fixture
}

fn expected_line(product: ProductId, item: &str, quantity: i64, rate: i64) -> OrderCartLine {
    OrderCartLine {
        product,
        item: item.into(),
        quantity,
        unit_rate: money(rate),
        best_supplier: None,
    }
}

#[test]
fn l13_02_group_add_all_preserves_existing_and_adds_supplied_visible_members_in_order() {
    // Given an existing line and a group with a missing ID and an above-threshold hidden row.
    let fixture = fixture();
    let mut plan: OrderPlanner = fixture.open_order_planner(OWNER).unwrap();
    let rows_before: Vec<_> = plan.rows(false).cloned().collect();
    assert!(plan.add_item(PRE_ADDED, Some(17)));
    let existing = plan.cart()[0].clone();
    let group = [
        (PRE_ADDED, Some(99)),
        (MISSING, Some(8)),
        (GROUP_FIRST, None),
        (GROUP_VISIBLE_ABOVE_THRESHOLD, Some(9)),
        (GROUP_LAST, Some(6)),
        (GROUP_FIRST, Some(12)),
    ];

    // When the group action receives its visible members in display order.
    plan.add_all(OWNER, &group).unwrap();
    let after_first_call = plan.cart().to_vec();
    plan.add_all(OWNER, &group).unwrap();

    // Then existing quantity stays intact, hidden/missing rows do not appear, and order/quantities hold.
    println!("group add all cart: {:?}", plan.cart());
    assert_eq!(
        plan.cart(),
        [
            existing,
            expected_line(GROUP_FIRST, "Synthetic First", 8, 210),
            OrderCartLine {
                best_supplier: plan
                    .rows(false)
                    .find(|row| row.product == GROUP_VISIBLE_ABOVE_THRESHOLD)
                    .and_then(|row| row.best_supplier.clone()),
                ..expected_line(
                    GROUP_VISIBLE_ABOVE_THRESHOLD,
                    "Synthetic Visible High",
                    9,
                    475
                )
            },
            expected_line(GROUP_LAST, "Synthetic Last", 6, 410),
        ]
    );
    assert_eq!(plan.cart(), after_first_call);
    assert_eq!(plan.rows(false).cloned().collect::<Vec<_>>(), rows_before);
    assert_eq!(fixture.product_stock(PRE_ADDED), Some(4));
    assert_eq!(fixture.product_stock(GROUP_FIRST), Some(1));
    assert_eq!(fixture.product_stock(HIDDEN), Some(20));
    assert_eq!(fixture.product_stock(GROUP_LAST), Some(2));
    assert_eq!(
        fixture.product_stock(GROUP_VISIBLE_ABOVE_THRESHOLD),
        Some(8)
    );
    let quotes_before = fixture
        .compare_supplier_prices(OWNER, Some(GROUP_VISIBLE_ABOVE_THRESHOLD))
        .unwrap();
    assert_eq!(quotes_before.rows[0].prices[0], Some(money(475)));
    assert!(fixture.purchase_orders(OWNER).unwrap().is_empty());
}

#[test]
fn l13_02_manager_cannot_add_all_or_change_fixture() {
    // Given a fixture with a quote and a visible group row.
    let fixture = fixture();
    let mut plan: OrderPlanner = fixture.open_order_planner(OWNER).unwrap();
    assert!(plan.add_item(PRE_ADDED, Some(17)));
    let plan_before = plan.clone();
    let rows_before: Vec<_> = plan.rows(false).cloned().collect();
    let stock_before = [PRE_ADDED, GROUP_VISIBLE_ABOVE_THRESHOLD, HIDDEN]
        .map(|product| fixture.product_stock(product));
    let quotes_before: Vec<SupplierPriceComparison> = [GROUP_VISIBLE_ABOVE_THRESHOLD]
        .map(|product| {
            fixture
                .compare_supplier_prices(OWNER, Some(product))
                .unwrap()
        })
        .into();
    let orders_before: Vec<PurchaseOrder> = fixture.purchase_orders(OWNER).unwrap().to_vec();

    // When a Manager invokes Group Add All.
    let result = plan.add_all(
        MANAGER,
        &[
            (GROUP_VISIBLE_ABOVE_THRESHOLD, Some(13)),
            (GROUP_LAST, None),
        ],
    );

    // Then access is denied and planner/fixture observations remain identical.
    assert_eq!(
        result,
        Err(alm_phase2_local::confirmation::order_planner::OrderPlannerError::AccessDenied)
    );
    assert_eq!(plan, plan_before);
    assert_eq!(plan.rows(false).cloned().collect::<Vec<_>>(), rows_before);
    assert_eq!(
        [PRE_ADDED, GROUP_VISIBLE_ABOVE_THRESHOLD, HIDDEN]
            .map(|product| fixture.product_stock(product)),
        stock_before
    );
    assert_eq!(
        [GROUP_VISIBLE_ABOVE_THRESHOLD]
            .map(|product| fixture
                .compare_supplier_prices(OWNER, Some(product))
                .unwrap())
            .as_slice(),
        quotes_before
    );
    assert_eq!(fixture.purchase_orders(OWNER).unwrap(), orders_before);
}
