use alm_core::confirmation::{
    ConfirmationFixture, PurchaseDraft, PurchaseError, PurchaseLine, PurchaseProduct,
    PurchaseSupplier,
};
use alm_core::{BillDate, Money, ProductId, Role};
use std::num::NonZeroU32;

const OWNER: Role = Role::Owner;
const PRODUCT: ProductId = ProductId::new(81);
const HEADER: &str = "Order Date\tParty Name\tItem Name\tPacket Qty\tRate\tPackets\tTotal\n";
const PASTE: &str = concat!(
    "2025-02-03\tSynthetic North\t\t12\t1.23\t2\t2.46\n",
    "2025-02-04\tSynthetic South\tfixture lime\t12\t2.30\t3\t6.90\n",
    "2025-02-05\tSynthetic South\tFixture Unknown\t24\t1.05\t4\t9.20\n",
);

fn fixture() -> ConfirmationFixture {
    let mut fixture = ConfirmationFixture::with_ordered_lots(
        PRODUCT,
        &[
            (12, Money::from_minor_units(130)),
            (5, Money::from_minor_units(140)),
        ],
        Money::from_minor_units(500),
    )
    .unwrap();
    fixture
        .register_purchase_product(PurchaseProduct {
            id: PRODUCT,
            item: "Fixture Lime".into(),
            buy_rate: Money::from_minor_units(175),
        })
        .unwrap();
    fixture
}

fn draft() -> PurchaseDraft {
    PurchaseDraft {
        supplier: Some(PurchaseSupplier {
            id: None,
            name: "Original Supplier".into(),
        }),
        date: Some("2024-02-29".parse().unwrap()),
        note: "Keep this note".into(),
        items: vec![PurchaseLine {
            item: "Prior item".into(),
            product_id: None,
            ordered: NonZeroU32::new(12).unwrap(),
            rate: Money::from_minor_units(530),
        }],
    }
}

#[test]
fn l07_01_mixed_paste_replaces_items_and_reports_warnings_before_save() {
    let fixture = fixture();
    let mut draft = draft();
    let stock_before = fixture.product_stock(PRODUCT);

    let warnings = fixture
        .parse_purchase_paste(OWNER, &mut draft, &format!("{HEADER}{PASTE}"))
        .unwrap();

    let supplier = draft.supplier.unwrap();
    assert_eq!(supplier.name, "Synthetic North");
    assert_eq!(supplier.id, None);
    assert_eq!(draft.date, Some("2025-02-03".parse::<BillDate>().unwrap()));
    assert_eq!(draft.note, "Keep this note");
    let [lime, unknown] = draft.items.as_slice() else {
        panic!("expected both usable rows")
    };
    assert_eq!(lime.item, "Fixture Lime");
    assert_eq!(lime.product_id, Some(PRODUCT));
    assert_eq!((lime.ordered.get(), lime.rate.minor_units()), (3, 230));
    assert_eq!(unknown.item, "Fixture Unknown");
    assert_eq!(unknown.product_id, None);
    assert_eq!(
        (unknown.ordered.get(), unknown.rate.minor_units()),
        (4, 105)
    );
    assert_eq!(warnings.unmatched, ["Fixture Unknown"]);
    assert_eq!(warnings.total_mismatches, ["Fixture Unknown"]);
    assert_eq!(warnings.parties, ["Synthetic North", "Synthetic South"]);
    assert_eq!(warnings.skipped_rows, 1);
    assert_eq!(
        warnings.unmatched_warning_text().as_deref(),
        Some(concat!(
            "⚠ Not in your product list: Fixture Unknown",
            " — pick the right name from suggestions before saving",
        ))
    );
    assert_eq!(
        warnings.mixed_party_warning_text().as_deref(),
        Some("⚠ Paste had 2 parties — all rows went into this one PO")
    );
    assert!(fixture.purchase_orders(OWNER).unwrap().is_empty());
    assert!(fixture.purchase_suppliers(OWNER).unwrap().is_empty());
    assert_eq!(fixture.product_stock(PRODUCT), stock_before);
}

#[test]
fn l07_01_no_usable_rows_preserve_the_entire_draft() {
    let mut fixture = fixture();
    let mut draft = draft();
    fixture.save_purchase_order(OWNER, &draft).unwrap();
    let saved = &fixture.purchase_suppliers(OWNER).unwrap()[0];
    draft.supplier = Some(PurchaseSupplier {
        id: Some(saved.id),
        name: saved.name.clone(),
    });
    let before = draft.clone();

    let text = format!("{HEADER}2025-02-03\tOther\tFixture Lime\t12\t1.23\t1.5\t3.00\n");
    assert_eq!(
        fixture.parse_purchase_paste(OWNER, &mut draft, &text),
        Err(PurchaseError::NoUsableRows)
    );

    assert_eq!(draft, before);
}

#[test]
fn l07_01_absent_party_and_invalid_date_preserve_existing_link_and_date() {
    let mut fixture = fixture();
    let mut draft = draft();
    fixture.save_purchase_order(OWNER, &draft).unwrap();
    let saved = &fixture.purchase_suppliers(OWNER).unwrap()[0];
    let supplier = PurchaseSupplier {
        id: Some(saved.id),
        name: saved.name.clone(),
    };
    draft.supplier = Some(supplier.clone());
    let before_date = draft.date;

    let text = format!("{HEADER}2025-02-29\t\tFixture-Lime\t12\t\t1\t\n");
    let warnings = fixture
        .parse_purchase_paste(OWNER, &mut draft, &text)
        .unwrap();

    assert!(warnings.parties.is_empty());
    assert_eq!(warnings.unmatched_warning_text(), None);
    assert_eq!(warnings.mixed_party_warning_text(), None);
    assert_eq!(draft.supplier, Some(supplier));
    assert_eq!(draft.date, before_date);
    assert_eq!(draft.items[0].product_id, Some(PRODUCT));
    assert_eq!(draft.items[0].rate.minor_units(), 175);
}

#[test]
fn l07_01_rows_require_positive_whole_quantity_and_exact_money() {
    let fixture = fixture();
    let mut draft = draft();
    let text = concat!(
        "Item\tQty\tRate\tTotal\n",
        "Fixture Lime\t0\t1.00\t0\n",
        "Fixture Lime\t-3\t1.00\t3\n",
        "Fixture Lime\t4294967296\t1.00\t3\n",
        "Fixture Lime\t1\t1.001\t1\n",
        "Fixture Lime\t1\t1.00\t1.001\n",
        "Fixture Lime\t1\t92233720368547758.08\t0\n",
        "Fixture Lime\t1\t0.09\t0.09\n",
    );

    let warnings = fixture
        .parse_purchase_paste(OWNER, &mut draft, text)
        .unwrap();

    assert_eq!(warnings.skipped_rows, 6);
    assert_eq!(draft.items.len(), 1);
    assert_eq!(draft.items[0].rate.minor_units(), 9);
}

#[test]
fn l07_02_save_adds_one_supplier_and_one_unreceived_order_without_stock_effects() {
    let mut fixture = fixture();
    let mut draft = draft();
    fixture
        .parse_purchase_paste(OWNER, &mut draft, &format!("{HEADER}{PASTE}"))
        .unwrap();
    let stock_before = fixture.product_stock(PRODUCT);
    assert!(fixture.purchase_suppliers(OWNER).unwrap().is_empty());
    assert!(fixture.purchase_orders(OWNER).unwrap().is_empty());

    let id = fixture.save_purchase_order(OWNER, &draft).unwrap();

    let [supplier] = fixture.purchase_suppliers(OWNER).unwrap() else {
        panic!("expected one supplier")
    };
    let [order] = fixture.purchase_orders(OWNER).unwrap() else {
        panic!("expected one order")
    };
    assert_eq!(supplier.name, "Synthetic North");
    assert_eq!(order.supplier_id, supplier.id);
    assert_eq!(order.id, id);
    assert_eq!(order.date, "2025-02-03".parse::<BillDate>().unwrap());
    assert_eq!(order.note, "Keep this note");
    assert_eq!(order.items.len(), 2);
    for (saved, filled) in order.items.iter().zip(draft.items) {
        assert_eq!(saved.line, filled);
        assert_eq!(saved.received, 0);
        assert!(saved.receipts().is_empty());
    }
    assert_eq!(fixture.product_stock(PRODUCT), stock_before);
}

#[test]
fn l07_02_save_with_one_missing_field_reports_the_source_message_without_writing() {
    let valid = draft();
    for (incomplete, error, message) in [
        (
            PurchaseDraft {
                supplier: None,
                ..valid.clone()
            },
            PurchaseError::MissingSupplier,
            "Enter supplier",
        ),
        (
            PurchaseDraft {
                items: Vec::new(),
                ..valid.clone()
            },
            PurchaseError::MissingItems,
            "Add at least one item with an order quantity",
        ),
        (
            PurchaseDraft {
                date: None,
                ..valid
            },
            PurchaseError::MissingDate,
            "Pick the order date",
        ),
    ] {
        let mut fixture = fixture();
        let stock_before = fixture.product_stock(PRODUCT);

        assert_eq!(fixture.save_purchase_order(OWNER, &incomplete), Err(error));

        assert_eq!(error.to_string(), message);
        assert!(fixture.purchase_orders(OWNER).unwrap().is_empty());
        assert!(fixture.purchase_suppliers(OWNER).unwrap().is_empty());
        assert_eq!(fixture.product_stock(PRODUCT), stock_before);
    }
}

#[test]
fn l07_01_and_l07_02_manager_is_denied_before_parse_or_validation() {
    let mut fixture = fixture();
    let mut draft = PurchaseDraft::default();

    assert_eq!(
        fixture.parse_purchase_paste(Role::Manager, &mut draft, ""),
        Err(PurchaseError::AccessDenied)
    );
    assert_eq!(
        fixture.save_purchase_order(Role::Manager, &draft),
        Err(PurchaseError::AccessDenied)
    );

    assert_eq!(draft, PurchaseDraft::default());
    assert!(fixture.purchase_orders(OWNER).unwrap().is_empty());
    assert!(fixture.purchase_suppliers(OWNER).unwrap().is_empty());
}
