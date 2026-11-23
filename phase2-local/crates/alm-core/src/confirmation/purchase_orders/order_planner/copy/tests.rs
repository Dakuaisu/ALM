use super::super::{OrderPlanner, OrderPlannerError};
use super::{CopyPlanFeedback, CopyPlanRejected};
use crate::confirmation::{ConfirmationFixture, ProductPriceInput, SupplierPriceEdit};
use crate::{BillDate, Money, ProductId, Role};

mod format;

const OWNER: Role = Role::Owner;
const EQUAL: ProductId = ProductId::new(11);
const ABOVE: ProductId = ProductId::new(12);
const BELOW: ProductId = ProductId::new(13);
const NEW_LOW: ProductId = ProductId::new(14);
const SUPPLIER: &str = "Synthetic Orchard";
const HEADER: &str = "*ALM Order Plan* (09-Feb-2024)\n\n";

fn date() -> BillDate {
    BillDate::new(2024, 2, 9).unwrap()
}

fn product(fixture: &mut ConfirmationFixture, seed: (ProductId, &str, u32, u32, i64)) {
    let (id, name, stock, threshold, buy) = seed;
    fixture
        .add_product(id, stock, Money::from_minor_units(100))
        .unwrap();
    let mut form = fixture.open_product_entry(OWNER, id).unwrap();
    form.item = name.into();
    form.reorder_level = Some(threshold);
    form.apply_price(ProductPriceInput::Buy(Money::from_minor_units(buy)))
        .unwrap();
    fixture.save_product_entry(OWNER, &form).unwrap();
}

fn fixture() -> ConfirmationFixture {
    let mut fixture =
        ConfirmationFixture::new(ProductId::new(99), 0, Money::from_minor_units(100)).unwrap();
    product(&mut fixture, (EQUAL, "Synthetic Equal", 5, 5, 149));
    product(&mut fixture, (ABOVE, "Synthetic Above", 8, 5, 999));
    product(&mut fixture, (BELOW, "Synthetic Lime & Soda", 2, 5, 950));
    fixture.add_comparison_supplier(OWNER, SUPPLIER).unwrap();
    set_quote(&mut fixture, BELOW, 149);
    set_quote(&mut fixture, ABOVE, 12_345_678);
    fixture
}

fn set_quote(fixture: &mut ConfirmationFixture, product: ProductId, cents: i64) {
    let supplier = fixture.purchase_suppliers(OWNER).unwrap()[0].id;
    fixture
        .edit_comparison_price(
            OWNER,
            SupplierPriceEdit {
                product,
                supplier,
                rate: Some(Money::from_minor_units(cents)),
            },
        )
        .unwrap();
}

fn cart(fixture: &ConfirmationFixture) -> OrderPlanner {
    let mut plan = fixture.open_order_planner(OWNER).unwrap();
    assert!(plan.add_item(BELOW, Some(1)));
    assert!(plan.add_item(ABOVE, Some(1)));
    assert!(plan.set_quantity(ABOVE, 0));
    assert!(plan.add_item(EQUAL, Some(1)));
    plan
}

fn copy_payload(fixture: &ConfirmationFixture, plan: &OrderPlanner) -> String {
    let mut payloads = Vec::new();
    let feedback = fixture.copy_order_plan(
        OWNER,
        plan,
        date(),
        Some(&mut |text| {
            payloads.push(text.to_owned());
            Ok(())
        }),
    );
    let message = feedback.unwrap().message();
    assert_eq!(message, "Order plan copied ✓");
    println!("L13-03 success feedback: {message:?}");
    let [payload]: [String; 1] = payloads.try_into().unwrap();
    payload
}

#[test]
fn l13_03_copy_cart_uses_captured_lines_and_rounds_only_the_aggregate() {
    let mut fixture = fixture();
    let plan = cart(&fixture);
    set_quote(&mut fixture, BELOW, 845);
    let before = (fixture.clone(), plan.clone());
    let payload = copy_payload(&fixture, &plan);

    assert_eq!(
        payload,
        concat!(
            "*ALM Order Plan* (09-Feb-2024)\n\n",
            "Synthetic Lime & Soda — 1 pkt (Synthetic Orchard ₹1.49)\n",
            "Synthetic Above — 0 pkt (Synthetic Orchard ₹1,23,456.78)\n",
            "Synthetic Equal — 1 pkt\n",
            "\nTotal ₹3",
        )
    );
    println!("L13-03 cart payload: {payload:?}");
    assert_eq!((fixture, plan), before);
}

#[test]
fn l13_03_copy_cart_puts_canonical_array_index_names_first() {
    let mut fixture =
        ConfirmationFixture::new(ProductId::new(99), 0, Money::from_minor_units(100)).unwrap();
    let names = "Zed 10 4294967295 01 2 +1 0 4294967294 4294967296 Alpha";
    for (index, name) in (1..).zip(names.split_ascii_whitespace()) {
        product(&mut fixture, (ProductId::new(index), name, 0, 0, 100));
    }
    let mut plan = fixture.open_order_planner(OWNER).unwrap();
    for (index, _) in (1..).zip(names.split_ascii_whitespace()) {
        assert!(plan.add_item(ProductId::new(index), Some(1)));
    }
    let before = (fixture.clone(), plan.clone());
    let payload = copy_payload(&fixture, &plan);

    let copied_names: Vec<_> = payload
        .lines()
        .filter_map(|line| line.strip_suffix(" — 1 pkt"))
        .collect();
    let expected: Vec<_> = "0 2 10 4294967294 Zed 4294967295 01 +1 4294967296 Alpha"
        .split_ascii_whitespace()
        .collect();
    assert_eq!(copied_names, expected);
    println!("L13-03 numeric-name payload: {payload:?}");
    assert_eq!((fixture, plan), before);
}

#[test]
fn l13_03_copy_total_matches_frozen_number_arithmetic() {
    for (lines, expected) in [
        ([(29, 50), (0, 1), (0, 1)], "Total ₹14"),
        ([(50, -5), (0, 1), (0, 1)], "Total ₹-2"),
        ([(50, -1), (0, 1), (0, 1)], "Total ₹0"),
        ([(4, 1), (43, 1), (3, 1)], "Total ₹0"),
        (
            [(42_221_246_506_598_500, 1), (0, 1), (0, 1)],
            "Total ₹42,22,12,46,50,65,984.94",
        ),
        (
            [(168_884_986_026_394_800, 1), (0, 1), (0, 1)],
            "Total ₹1,68,88,49,86,02,63,948.2",
        ),
        (
            [(9_007_199_254_740_993, 8), (0, 1), (0, 1)],
            "Total ₹72,05,75,94,03,79,280",
        ),
    ] {
        let mut fixture = fixture();
        let keys = [(9, "3"), (7, "1"), (8, "2")];
        for ((id, name), (rate, _)) in keys.into_iter().zip(lines) {
            product(&mut fixture, (ProductId::new(id), name, 0, 0, rate));
        }
        let mut plan = fixture.open_order_planner(OWNER).unwrap();
        for ((id, _), (_, quantity)) in keys.into_iter().zip(lines) {
            assert!(plan.add_item(ProductId::new(id), Some(quantity)));
        }

        let payload = copy_payload(&fixture, &plan);

        let total = payload.rsplit("\n\n").next().unwrap();
        println!("L13-03 Number total for {lines:?}: {total:?}");
        assert_eq!(total, expected, "{lines:?}");
    }
}

#[test]
fn l13_03_copy_empty_cart_uses_fresh_catalog_and_keeps_catalog_order() {
    let mut fixture = fixture();
    let plan = fixture.open_order_planner(OWNER).unwrap();
    set_quote(&mut fixture, BELOW, 845);
    product(&mut fixture, (NEW_LOW, "Synthetic New Low", 0, 41_152, 150));
    let visible: Vec<_> = plan
        .rows(true)
        .filter(|row| row.item.contains("Lime"))
        .map(|row| row.item.as_str())
        .collect();
    assert_eq!(visible, ["Synthetic Lime & Soda"]);
    println!("L13-03 restricted visible rows: {visible:?}");
    let before = (fixture.clone(), plan.clone());
    let payload = copy_payload(&fixture, &plan);

    assert_eq!(
        payload,
        concat!(
            "*ALM Order Plan* (09-Feb-2024)\n\n",
            "Synthetic Equal — 10 pkt\n",
            "Synthetic Lime & Soda — 13 pkt (Synthetic Orchard ₹8.45)\n",
            "Synthetic New Low — 1,23,456 pkt\n",
        )
    );
    println!("L13-03 empty cart payload: {payload:?}");
    assert_eq!((fixture, plan), before);
}

#[test]
fn l13_03_copy_reports_rejection_after_one_attempt_even_without_low_rows() {
    let fixture =
        ConfirmationFixture::new(ProductId::new(99), 0, Money::from_minor_units(100)).unwrap();
    let plan = fixture.open_order_planner(OWNER).unwrap();
    let before = (fixture.clone(), plan.clone());
    let mut payloads = Vec::new();

    let feedback = fixture.copy_order_plan(
        OWNER,
        &plan,
        date(),
        Some(&mut |text| {
            payloads.push(text.to_owned());
            Err(CopyPlanRejected)
        }),
    );

    assert_eq!(feedback, Ok(CopyPlanFeedback::Rejected));
    let message = feedback.unwrap().message();
    assert_eq!(message, "Copy failed");
    assert_eq!(payloads, [HEADER]);
    println!("L13-03 rejected header-only payload: {:?}", payloads[0]);
    println!("L13-03 rejection feedback: {message:?}");
    assert_eq!((fixture, plan), before);
}

#[test]
fn l13_03_copy_reports_unavailable_when_owner_has_no_writer() {
    let fixture = fixture();
    let plan = cart(&fixture);
    let before = (fixture.clone(), plan.clone());

    let feedback = fixture.copy_order_plan(OWNER, &plan, date(), None);

    assert_eq!(feedback, Ok(CopyPlanFeedback::Unavailable));
    let message = feedback.unwrap().message();
    assert_eq!(message, "Copy not available in this browser");
    println!("L13-03 unavailable feedback: {message:?}");
    assert_eq!((fixture, plan), before);
}

#[test]
fn l13_03_copy_denies_manager_before_using_an_owner_cart_or_writer() {
    let fixture = fixture();
    let plan = cart(&fixture);
    let before = (fixture.clone(), plan.clone());
    let mut calls = 0;

    let feedback = fixture.copy_order_plan(
        Role::Manager,
        &plan,
        date(),
        Some(&mut |_| {
            calls += 1;
            Ok(())
        }),
    );

    assert_eq!(feedback, Err(OrderPlannerError::AccessDenied));
    assert_eq!(calls, 0);
    assert_eq!((fixture, plan), before);
}

#[test]
fn l13_03_copy_does_not_write_a_partial_payload_when_cart_overflows() {
    let fixture = fixture();
    let mut plan = cart(&fixture);
    assert!(plan.set_quantity(BELOW, i64::MAX));
    let before = (fixture.clone(), plan.clone());
    let mut calls = 0;

    let feedback = fixture.copy_order_plan(
        OWNER,
        &plan,
        date(),
        Some(&mut |_| {
            calls += 1;
            Ok(())
        }),
    );

    assert_eq!(feedback, Err(OrderPlannerError::AmountOverflow));
    assert_eq!(calls, 0);
    assert_eq!((fixture, plan), before);
}
