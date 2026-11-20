use std::num::NonZeroU32;

use alm_phase2_local::confirmation::{
    ConfirmationFixture, PurchaseDraft, PurchaseError, PurchaseLine, PurchasePasteWarnings,
    PurchaseProduct, PurchaseSupplier,
};
use alm_phase2_local::{BillDate, Money, ProductId, Role};

const OWNER: Role = Role::Owner;
const GREEN: ProductId = ProductId::new(7);
const GOLD: ProductId = ProductId::new(8);
const HEADERED: &str = concat!(
    "Case Qty\tSupplier Name\tItem Description\tPacket Qty\tOrder Date\tBuy Rate\tLine Total\r\n",
    "2\tNorth Supply\tLime Green\t24\t29-Feb-24\t ₹ 1,234.56 \t₹2,469.12\r\n",
    "0\tNorth Supply\tDiscard this\t24\t01-Mar-24\t1\t0\r\n",
    "1\tSouth Supply\tFixture Lime\t12\t02-Mar-24\t₹ 505.01\t₹ 500\r\n",
);
const HEADERLESS: &str = concat!(
    "29/02/24\tNorth Supply\tLime Green\t100\tg\t24\t1350\t₹\u{a0}1,234.56\t2\t₹2, 469.12\n",
    "01/03/24\tNorth Supply\tDiscard this\t100\tg\t24\t1350\t1\t0\t0\n",
    "02/03/24\tSouth Supply\tFixture Lime\t100\tg\t12\t550\t₹ 505.01\t1\t₹ 500\n",
);

fn fixture() -> ConfirmationFixture {
    let mut fixture = ConfirmationFixture::with_ordered_lots(
        GREEN,
        &[(12, Money::from_minor_units(280))],
        Money::from_minor_units(450),
    )
    .unwrap();
    fixture
        .add_product(GOLD, 5, Money::from_minor_units(475))
        .unwrap();
    for (id, item, rate) in [
        (GREEN, "Fixture Lime Green", 310),
        (GOLD, "Fixture Lime Gold", 325),
    ] {
        fixture
            .register_purchase_product(PurchaseProduct {
                id,
                item: item.into(),
                buy_rate: Money::from_minor_units(rate),
            })
            .unwrap();
    }
    fixture
}

fn starting_draft() -> PurchaseDraft {
    PurchaseDraft {
        supplier: Some(PurchaseSupplier {
            id: None,
            name: "Prior supplier".into(),
        }),
        date: Some("2023-12-31".parse().unwrap()),
        note: "Keep this note".into(),
        items: vec![PurchaseLine {
            item: "Prior item".into(),
            product_id: Some(GOLD),
            ordered: NonZeroU32::new(4).unwrap(),
            rate: Money::from_minor_units(999),
        }],
    }
}

fn assert_parse_then_save(text: &str) {
    let mut fixture = fixture();
    let mut draft = starting_draft();
    let date = BillDate::new(2024, 2, 29).unwrap();
    let expected = PurchaseDraft {
        supplier: Some(PurchaseSupplier {
            id: None,
            name: "North Supply".into(),
        }),
        date: Some(date),
        note: "Keep this note".into(),
        items: vec![
            PurchaseLine {
                item: "Fixture Lime Green".into(),
                product_id: Some(GREEN),
                ordered: NonZeroU32::new(2).unwrap(),
                rate: Money::from_minor_units(123_456),
            },
            PurchaseLine {
                item: "Fixture Lime".into(),
                product_id: None,
                ordered: NonZeroU32::MIN,
                rate: Money::from_minor_units(50_501),
            },
        ],
    };

    let warnings = fixture
        .parse_purchase_paste(OWNER, &mut draft, text)
        .unwrap();

    assert_eq!(draft, expected);
    assert_eq!(
        warnings,
        PurchasePasteWarnings {
            unmatched: vec!["Fixture Lime".into()],
            total_mismatches: vec!["Fixture Lime".into()],
            parties: vec!["North Supply".into(), "South Supply".into()],
            skipped_rows: 1,
        }
    );
    assert_eq!(
        warnings.unmatched_warning_text().as_deref(),
        Some(
            "⚠ Not in your product list: Fixture Lime — pick the right name from suggestions before saving"
        )
    );
    assert_eq!(
        warnings.mixed_party_warning_text().as_deref(),
        Some("⚠ Paste had 2 parties — all rows went into this one PO")
    );
    assert!(fixture.purchase_suppliers(OWNER).unwrap().is_empty());
    assert!(fixture.purchase_orders(OWNER).unwrap().is_empty());
    assert_eq!(
        [GREEN, GOLD].map(|id| fixture.product_stock(id)),
        [Some(12), Some(5)]
    );

    let id = fixture.save_purchase_order(OWNER, &draft).unwrap();

    let [supplier] = fixture.purchase_suppliers(OWNER).unwrap() else {
        panic!("expected exactly one saved supplier");
    };
    let [order] = fixture.purchase_orders(OWNER).unwrap() else {
        panic!("expected exactly one saved purchase order");
    };
    assert_eq!(supplier.name, "North Supply");
    assert_eq!((order.id, order.supplier_id), (id, supplier.id));
    assert_eq!(order.supplier_name, "North Supply");
    assert_eq!(order.date, date);
    assert_eq!(order.note, "Keep this note");
    assert_eq!(order.items.len(), expected.items.len());
    for (saved, expected_line) in order.items.iter().zip(expected.items) {
        assert_eq!(saved.line, expected_line);
        assert_eq!(saved.received, 0);
        assert!(saved.receipts().is_empty());
    }
    assert!(fixture.purchase_movements(OWNER, id).unwrap().is_empty());
    assert_eq!(
        [GREEN, GOLD].map(|id| fixture.product_stock(id)),
        [Some(12), Some(5)]
    );
    for (id, rate) in [(GREEN, 310), (GOLD, 325)] {
        let rate = Money::from_minor_units(rate);
        assert_eq!(
            fixture.purchase_cost(OWNER, id, "North Supply").unwrap(),
            Some((rate, rate, None))
        );
    }
}

#[test]
fn l07_04_headered_paste_and_save_keep_the_exact_lines() {
    assert_parse_then_save(HEADERED);
}

#[test]
fn l07_04_headerless_paste_and_save_keep_the_exact_lines() {
    assert_parse_then_save(HEADERLESS);
}

#[test]
fn l07_04_header_recognition_accepts_date_without_rate_but_not_qty_alone() {
    let fixture = fixture();
    let mut draft = starting_draft();
    let date_only = "Item\tOrder Date\tQty\nfixture lime green\t1.2.25\t1\n";
    assert_eq!(
        fixture.parse_purchase_paste(OWNER, &mut draft, date_only),
        Ok(PurchasePasteWarnings::default())
    );
    assert_eq!(draft.date, Some(BillDate::new(2025, 2, 1).unwrap()));
    assert_eq!(draft.items[0].product_id, Some(GREEN));
    assert_eq!(draft.items[0].rate, Money::from_minor_units(310));

    let before = draft.clone();
    assert_eq!(
        fixture.parse_purchase_paste(OWNER, &mut draft, "Item\tQty\nLime Green\t1\n"),
        Err(PurchaseError::NoUsableRows)
    );
    assert_eq!(draft, before);
}

#[test]
fn l07_04_discrepancies_use_a_strict_floor_and_percentage_of_the_pasted_total() {
    let fixture = fixture();
    for (rate, total, mismatches) in [
        ("102.00", "100", vec![]),
        ("102.01", "100", vec!["Fixture Lime Green"]),
        ("505.00", "500", vec![]),
        ("505.01", "500", vec!["Fixture Lime Green"]),
    ] {
        let mut draft = starting_draft();
        let text = format!("Item\tQty\tRate\tTotal\nFixture-Lime-Green\t1\t{rate}\t{total}\n");
        let warnings = fixture
            .parse_purchase_paste(OWNER, &mut draft, &text)
            .unwrap();
        assert_eq!(warnings.total_mismatches, mismatches, "{rate} vs {total}");
        assert_eq!(warnings.skipped_rows, 0);
        assert!(warnings.unmatched.is_empty());
        assert_eq!(draft.items[0].product_id, Some(GREEN));
    }
}

#[test]
fn l07_04_invalid_calendar_dates_do_not_override_the_draft() {
    let fixture = fixture();
    for invalid in ["2025-02-29", "29/02/25", "31 April 25"] {
        let mut draft = starting_draft();
        let before = draft.date;
        let text = format!("Date\tItem\tQty\n{invalid}\tLime Green\t1\n");
        assert_eq!(
            fixture.parse_purchase_paste(OWNER, &mut draft, &text),
            Ok(PurchasePasteWarnings::default())
        );
        assert_eq!(draft.date, before, "{invalid}");
        assert_eq!(draft.items[0].product_id, Some(GREEN));
    }
}

#[test]
fn l07_04_formatted_money_stays_exact_and_rejects_invalid_amounts() {
    let fixture = fixture();
    let mut draft = starting_draft();
    let text = concat!(
        "Item\tQty\tRate\tTotal\n",
        "Lime Green\t1\t₹ 90,071,992,547,409.93\t₹90,071,992,547,409.93\n",
        "Lime Green\t1\t₹1.001\t1\n",
        "Lime Green\t1\t1\t₹1.00oops\n",
        "Lime Green\t1\t₹92,233,720,368,547,758.08\t1\n",
    );

    let warnings = fixture
        .parse_purchase_paste(OWNER, &mut draft, text)
        .unwrap();

    assert_eq!(warnings.skipped_rows, 3);
    assert!(warnings.total_mismatches.is_empty());
    let [line] = draft.items.as_slice() else {
        panic!("expected only the exact, representable amount");
    };
    assert_eq!(line.rate.minor_units(), 9_007_199_254_740_993);
}

#[test]
fn l07_04_manager_cannot_parse_or_save_for_either_layout() {
    for text in [HEADERED, HEADERLESS] {
        let mut fixture = fixture();
        let mut draft = starting_draft();
        let before = draft.clone();

        assert_eq!(
            fixture.parse_purchase_paste(Role::Manager, &mut draft, text),
            Err(PurchaseError::AccessDenied)
        );
        assert_eq!(draft, before);
        assert_eq!(
            fixture.save_purchase_order(Role::Manager, &draft),
            Err(PurchaseError::AccessDenied)
        );
        assert!(fixture.purchase_orders(OWNER).unwrap().is_empty());
        assert!(fixture.purchase_suppliers(OWNER).unwrap().is_empty());
    }
}
