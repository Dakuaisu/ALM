//! Legacy case L13-01.
use crate::confirmation::{ConfirmationFixture, ManualGodownInput, SupplierPriceEdit};
use crate::{BillDate, Money, ProductId, Role};

use super::super::ProductPriceInput;
use super::ReorderError;
use super::ReorderStatus::{Low, Out};

mod buy_rate;

const OWNER: Role = Role::Owner;
const ABOVE: ProductId = ProductId::new(80);
const EQUAL: ProductId = ProductId::new(70);
const BELOW: ProductId = ProductId::new(50);
const ZERO: ProductId = ProductId::new(10);
const TIED_STOCK: ProductId = ProductId::new(20);
const DORMANT: &str = "Synthetic Dormant";
const TIE_WINNER: &str = "Synthetic Orchard";
const OTHER_QUOTE: &str = "Synthetic Depot";
const HEADER: &str = "*ALM Reorder List* (09-Feb-2024)\n\n";
const ABOVE_SEED: ProductSeed = ProductSeed {
    id: ABOVE,
    item: "Synthetic Above",
    stock: 6,
    level: 5,
    buy: 800,
};

struct ProductSeed {
    id: ProductId,
    item: &'static str,
    stock: u32,
    level: u32,
    buy: i64,
}

impl ProductSeed {
    fn insert(&self, fixture: &mut ConfirmationFixture) {
        fixture
            .add_product(self.id, self.stock, money(100))
            .unwrap();
        let mut form = fixture.open_product_entry(OWNER, self.id).unwrap();
        form.item = self.item.into();
        form.reorder_level = Some(self.level);
        form.apply_price(ProductPriceInput::Buy(money(self.buy)))
            .unwrap();
        fixture.save_product_entry(OWNER, &form).unwrap();
    }
}

const fn money(minor_units: i64) -> Money {
    Money::from_minor_units(minor_units)
}

fn date() -> BillDate {
    BillDate::new(2024, 2, 9).unwrap()
}

fn fixture() -> ConfirmationFixture {
    let mut fixture = ConfirmationFixture::new(ProductId::new(99), 0, money(100)).unwrap();
    ABOVE_SEED.insert(&mut fixture);
    for (id, item, stock, level, buy) in [
        (EQUAL, "Synthetic Equal", 5, 5, 2_350),
        (BELOW, "Synthetic Lime & Soda", 2, 5, 950),
        (ZERO, "Synthetic Zero", 0, 0, 151),
        (TIED_STOCK, "Synthetic Tied Stock", 2, 123_458, 101),
    ] {
        ProductSeed {
            id,
            item,
            stock,
            level,
            buy,
        }
        .insert(&mut fixture);
    }
    let mut stored = fixture.purchases.product_quotes(BELOW);
    stored.set(DORMANT, Some(money(2)));
    fixture.purchases.save_product_quotes(BELOW, stored);
    let columns = [
        OTHER_QUOTE,
        TIE_WINNER,
        "Synthetic Expensive",
        "Synthetic Negative",
        "Synthetic Zero Quote",
    ]
    .map(|name| {
        fixture
            .add_comparison_supplier(OWNER, name)
            .unwrap()
            .unwrap()
            .id
    });
    for (product, column, rate) in [
        (BELOW, 1, 835),
        (BELOW, 0, 835),
        (BELOW, 2, 990),
        (BELOW, 3, -1),
        (BELOW, 4, 0),
        (EQUAL, 3, -200),
        (EQUAL, 4, 0),
    ] {
        fixture
            .edit_comparison_price(
                OWNER,
                SupplierPriceEdit {
                    product,
                    supplier: columns[column],
                    rate: Some(money(rate)),
                },
            )
            .unwrap();
    }
    fixture
        .save_manual_godown(
            OWNER,
            ManualGodownInput {
                item: "Synthetic Lime & Soda",
                cases: "999",
                note: "",
            },
        )
        .unwrap();
    fixture
}

#[test]
fn l13_01_owner_projects_billing_stock_and_checked_minor_unit_costs() {
    let fixture = fixture();
    let before = fixture.clone();

    let rows = fixture.open_reorder_planner(OWNER).unwrap();

    let expected = [
        (ZERO, 0, 0, 1, None, 151, 151, "₹2", Out),
        (BELOW, 2, 5, 13, Some(TIE_WINNER), 835, 10_855, "₹109", Low),
        (
            TIED_STOCK,
            2,
            123_458,
            370_372,
            None,
            101,
            37_407_572,
            "₹3,74,076",
            Low,
        ),
        (EQUAL, 5, 5, 10, None, 2_350, 23_500, "₹235", Low),
    ];
    assert_eq!(rows.len(), expected.len());
    for (row, (product, stock, level, quantity, supplier, rate, cost, display, status)) in
        rows.iter().zip(expected)
    {
        println!(
            "l13_01 Owner row: {row:?}; display={}",
            row.estimated_cost_display()
        );
        assert_eq!(row.product, product);
        assert_eq!((row.on_hand, row.reorder_level), (stock, level));
        assert_eq!(row.suggested_quantity.get(), quantity);
        assert_eq!(
            (row.best_supplier.as_deref(), row.unit_rate),
            (supplier, money(rate))
        );
        assert_eq!(row.estimated_cost, money(cost));
        assert_eq!(row.estimated_cost_display(), display);
        assert_eq!(row.status, status);
    }
    let above = fixture.open_product_entry(OWNER, ABOVE).unwrap();
    println!(
        "l13_01 excluded: {:?}; billing stock={:?}, reorder={:?}",
        above.item,
        fixture.product_stock(ABOVE),
        above.reorder_level
    );
    assert_eq!(fixture, before);
}

#[test]
fn l13_01_copy_captures_exact_dated_text_and_leaves_fixture_unchanged() {
    let fixture = fixture();
    let before = fixture.clone();
    let mut captured = Vec::new();

    fixture
        .copy_reorder_list(OWNER, date(), &mut |text| captured.push(text))
        .unwrap();

    println!("l13_01 nonempty payload: {:?}", captured[0]);
    assert_eq!(
        captured,
        [concat!(
            "*ALM Reorder List* (09-Feb-2024)\n\n",
            "Synthetic Zero — 1 pkt\n",
            "Synthetic Lime & Soda — 13 pkt (Synthetic Orchard ₹8.35)\n",
            "Synthetic Tied Stock — 3,70,372 pkt\n",
            "Synthetic Equal — 10 pkt\n",
        )]
    );
    assert_eq!(fixture, before);
}

#[test]
fn l13_01_copy_still_emits_one_header_when_all_products_are_above_threshold() {
    let mut fixture = ConfirmationFixture::new(ProductId::new(99), 0, money(100)).unwrap();
    ABOVE_SEED.insert(&mut fixture);
    let before = fixture.clone();
    let mut captured = Vec::new();

    fixture
        .copy_reorder_list(OWNER, date(), &mut |text| captured.push(text))
        .unwrap();

    println!("l13_01 empty payload: {:?}", captured[0]);
    assert_eq!(captured, [HEADER]);
    assert_eq!(fixture.open_reorder_planner(OWNER).unwrap(), []);
    assert_eq!(fixture, before);
}

#[test]
fn l13_01_reopening_uses_new_comparison_columns() {
    let mut fixture = fixture();
    fixture
        .add_comparison_supplier(OWNER, DORMANT)
        .unwrap()
        .unwrap();
    let before = fixture.clone();

    let rows = fixture.open_reorder_planner(OWNER).unwrap();

    let row = rows.iter().find(|row| row.product == BELOW).unwrap();
    assert_eq!(
        (row.best_supplier.as_deref(), row.unit_rate),
        (Some(DORMANT), money(2))
    );
    assert_eq!(fixture, before);
}

#[test]
fn l13_01_equal_stored_quote_can_label_a_minimum_established_by_live_columns() {
    let mut fixture = fixture();
    let mut stored = fixture.purchases.product_quotes(BELOW);
    stored.set(DORMANT, Some(money(835)));
    fixture.purchases.save_product_quotes(BELOW, stored);

    let rows = fixture.open_reorder_planner(OWNER).unwrap();

    let row = rows.iter().find(|row| row.product == BELOW).unwrap();
    assert_eq!(
        (row.best_supplier.as_deref(), row.unit_rate),
        (Some(DORMANT), money(835))
    );
}

#[test]
fn l13_01_manager_cannot_open_or_copy_and_never_calls_the_sink() {
    let fixture = fixture();
    let before = fixture.clone();
    let mut captured = Vec::new();

    let rows = fixture.open_reorder_planner(Role::Manager);
    let copy = fixture.copy_reorder_list(Role::Manager, date(), &mut |text| captured.push(text));

    assert_eq!(rows, Err(ReorderError::AccessDenied));
    assert_eq!(copy, Err(ReorderError::AccessDenied));
    assert_eq!(captured.len(), 0);
    assert_eq!(fixture, before);
    println!(
        "l13_01 Manager: open={rows:?}, copy={copy:?}, sink calls={}, whole fixture unchanged",
        captured.len()
    );
}
