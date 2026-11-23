//! Legacy case L06-02: numbering, delete, saved fields and the inert product reference.
use super::*;
use crate::confirmation::{GstBillLedger, GstUnit, ProductTerms};

const OWNER: Role = Role::Owner;

fn bill(id: u64, number: u64) -> GstBillDraft<'static> {
    GstBillDraft {
        id: GstBillId(id),
        number: GstBillNumber(number),
        ..gst_draft(LINES)
    }
}

fn save(fixture: &mut ConfirmationFixture, draft: &GstBillDraft<'_>) {
    fixture.save_gst_bill(OWNER, draft, &mut |_| {}).unwrap();
}

/// `nextGstNo` (HTML L5170) and `newGstDraft` (L5187-5214).
#[test]
fn standalone_gst_numbering_counts_cancelled_gst_bills_only_and_allows_overrides() {
    let mut fixture = gst_invoice::fixture(3_000);
    let before = fixture.clone();
    assert_eq!(fixture.next_gst_bill_number(OWNER), Ok(GstBillNumber(1)));
    assert_eq!(
        fixture.next_gst_bill_number(Role::Manager),
        Err(GstBillError::AccessDenied)
    );
    let today = BillDate::new(2025, 4, 1).unwrap();
    let first = fixture
        .new_gst_bill_draft(OWNER, GstBillId(1), today)
        .unwrap();
    assert_eq!(
        (first.number, first.date, first.created, first.lines.len()),
        (GstBillNumber(1), Some(today), 0, 1)
    );
    assert_eq!(
        (
            first.cust_state,
            first.cust_state_code,
            first.pos_state,
            first.pos_code
        ),
        ("Uttar Pradesh", "09", "Uttar Pradesh", "09")
    );
    assert_eq!((first.pay_mode, first.reverse_charge), ("Cash", "No"));
    let blank = first.lines[0];
    assert_eq!(
        (
            blank.item,
            blank.hsn,
            blank.unit,
            blank.quantity,
            blank.packet_qty
        ),
        ("", "", GstUnit::Case, 1, 1)
    );

    save(&mut fixture, &bill(1, 7));
    save(&mut fixture, &bill(2, 3));
    fixture.cancel_gst_bill(OWNER, GstBillId(1)).unwrap();
    assert_eq!(fixture.next_gst_bill_number(OWNER), Ok(GstBillNumber(8)));
    let next = fixture
        .new_gst_bill_draft(OWNER, GstBillId(3), today)
        .unwrap();
    assert_eq!(next.number, GstBillNumber(8));

    // The typed number is kept as entered, duplicates included, as the HTML never checks.
    save(&mut fixture, &bill(3, 3));
    assert_eq!(fixture.gst_bills[&GstBillId(3)].number, GstBillNumber(3));
    // Editing keeps the bill's own number.
    save(
        &mut fixture,
        &GstBillDraft {
            customer: "Edited buyer",
            ..bill(2, 3)
        },
    );
    assert_eq!(fixture.gst_bills[&GstBillId(2)].number, GstBillNumber(3));
    assert_eq!(fixture.next_gst_bill_number(OWNER), Ok(GstBillNumber(8)));
    assert_ordinary_unchanged(&fixture, &before);
}

/// `removeRec("gstBills", id)` (HTML L5281-5288, L994-998) frees the highest number again.
#[test]
fn standalone_gst_deleting_the_highest_number_makes_it_next_again() {
    let mut fixture = gst_invoice::fixture(3_000);
    let before = fixture.clone();
    save(&mut fixture, &bill(1, 4));
    save(&mut fixture, &bill(2, 5));
    assert_eq!(fixture.next_gst_bill_number(OWNER), Ok(GstBillNumber(6)));
    fixture.delete_gst_bill(OWNER, GstBillId(2)).unwrap();
    assert_eq!(fixture.next_gst_bill_number(OWNER), Ok(GstBillNumber(5)));
    fixture.delete_gst_bill(OWNER, GstBillId(1)).unwrap();
    assert_eq!(fixture.next_gst_bill_number(OWNER), Ok(GstBillNumber(1)));
    assert_ordinary_unchanged(&fixture, &before);
}

/// Delete works on active and cancelled bills and leaves a marker (HTML L5281-5288, L994-998).
#[test]
fn standalone_gst_delete_removes_active_and_cancelled_bills_and_records_markers() {
    let mut fixture = gst_invoice::fixture(3_000);
    let before = fixture.clone();
    for (id, number) in [(1, 1), (2, 2), (3, 3)] {
        save(&mut fixture, &bill(id, number));
    }
    fixture.cancel_gst_bill(OWNER, GstBillId(2)).unwrap();
    let kept = fixture.clone();
    assert_eq!(
        fixture.delete_gst_bill(Role::Manager, GstBillId(1)),
        Err(GstBillError::AccessDenied)
    );
    assert_eq!(
        fixture.delete_gst_bill(OWNER, GstBillId(9)),
        Err(GstBillError::NotFound)
    );
    assert_eq!(fixture, kept);

    fixture.delete_gst_bill(OWNER, GstBillId(1)).unwrap();
    fixture.delete_gst_bill(OWNER, GstBillId(2)).unwrap();
    assert_eq!(
        fixture.delete_gst_bill(OWNER, GstBillId(2)),
        Err(GstBillError::NotFound)
    );
    assert_eq!(
        fixture.emit_gst_bill(OWNER, GstBillId(1), &mut |_| panic!("deleted bill emitted")),
        Err(GstBillError::NotFound)
    );
    let ledger = fixture.gst_bill_ledger(OWNER).unwrap();
    assert_eq!(
        ledger
            .bills
            .iter()
            .map(|bill| (bill.id, bill.cancelled))
            .collect::<Vec<_>>(),
        [(GstBillId(3), false)]
    );
    assert_eq!(ledger.deleted, [GstBillId(1), GstBillId(2)]);
    let after_delete = fixture.clone();
    assert_eq!(
        fixture.save_gst_bill(OWNER, &bill(1, 1), &mut |_| panic!("deleted bill restored")),
        Err(GstBillError::Deleted)
    );
    assert_eq!(fixture, after_delete);
    assert_eq!(
        fixture.gst_bill_ledger(Role::Manager),
        Err(GstBillError::AccessDenied)
    );
    assert_ne!(ledger, GstBillLedger::default());
    assert_ordinary_unchanged(&fixture, &before);
}

/// `saveGstBill` (HTML L5786-5830): defaults, fallbacks, `created`, unit and pack size.
#[test]
fn standalone_gst_save_stores_every_html_field_with_its_default() {
    let mut fixture = gst_invoice::fixture(3_000);
    let before = fixture.clone();
    let lines = [
        GstBillLineInput {
            hsn: "22029090",
            unit: GstUnit::Piece,
            packet_qty: 0,
            ..COLA
        },
        WATER,
    ];
    let draft = GstBillDraft {
        customer_id: "",
        cust_state: "",
        cust_state_code: "",
        pos_state: "",
        pos_code: "",
        pay_mode: "",
        reverse_charge: "",
        now: 1_700_000_000_123,
        ..gst_draft(&lines)
    };
    save(&mut fixture, &draft);
    let saved = fixture.gst_bills[&ID].clone();
    assert_eq!(saved.customer_id, None);
    assert_eq!(
        [
            saved.cust_state.as_str(),
            &saved.cust_state_code,
            &saved.pos_state,
            &saved.pos_code,
            &saved.pay_mode,
            &saved.reverse_charge,
        ],
        ["Uttar Pradesh", "09", "Uttar Pradesh", "09", "Cash", "No"]
    );
    assert_eq!(saved.created, 1_700_000_000_123);
    assert_eq!(
        (
            saved.lines[0].hsn.as_str(),
            saved.lines[0].unit,
            saved.lines[0].packet_qty.get()
        ),
        ("22029090", GstUnit::Piece, 1)
    );
    assert_eq!(
        (
            saved.lines[1].hsn.as_str(),
            saved.lines[1].unit,
            saved.lines[1].product
        ),
        ("", GstUnit::Case, None)
    );

    // Place of supply falls back to the customer's state; text fields are kept verbatim.
    let full = GstBillDraft {
        customer_id: "cust-7",
        cust_addr: "12 Market Road",
        cust_phone: "9000000001",
        cust_email: "buyer@example.test",
        cust_state: "Maharashtra",
        cust_state_code: "27",
        pos_state: "",
        pos_code: "",
        pay_mode: "Credit",
        pay_terms: "15 days",
        reverse_charge: "Yes",
        transporter: "Synthetic Roadways",
        vehicle_no: "UP32AB1234",
        lr_no: "LR-9",
        terms_delivery: "Door delivery",
        dispatch: "Lucknow",
        created: 1_600_000_000_000,
        now: 1_800_000_000_000,
        ..gst_draft(LINES)
    };
    save(&mut fixture, &full);
    let edited = &fixture.gst_bills[&ID];
    assert_eq!(edited.customer_id.as_deref(), Some("cust-7"));
    assert_eq!(
        (edited.pos_state.as_str(), edited.pos_code.as_str()),
        ("Maharashtra", "27")
    );
    assert_eq!(edited.jurisdiction(), GstJurisdiction::Inter);
    assert_eq!(
        [
            edited.cust_addr.as_str(),
            &edited.cust_phone,
            &edited.cust_email,
            &edited.pay_mode,
            &edited.pay_terms,
            &edited.reverse_charge,
            &edited.transporter,
            &edited.vehicle_no,
            &edited.lr_no,
            &edited.terms_delivery,
            &edited.dispatch,
        ],
        [
            "12 Market Road",
            "9000000001",
            "buyer@example.test",
            "Credit",
            "15 days",
            "Yes",
            "Synthetic Roadways",
            "UP32AB1234",
            "LR-9",
            "Door delivery",
            "Lucknow",
        ]
    );
    // An edit keeps the first save's timestamp.
    assert_eq!(edited.created, 1_600_000_000_000);
    assert_ordinary_unchanged(&fixture, &before);
}

/// `pid` is stored (HTML L5819) but no stock, lot or ledger code reads it.
#[test]
fn standalone_gst_product_reference_leaves_stock_and_lots_unchanged() {
    let mut fixture = gst_invoice::fixture(3_000);
    let product = ProductId::new(2);
    let mut entry = fixture.open_product_entry(OWNER, product).unwrap();
    entry.item = "Stocked Cola".into();
    fixture.save_product_entry(OWNER, &entry).unwrap();
    let before = fixture.clone();
    let stock = fixture.product_stock(product);
    let lots = fixture.open_stock_lots(OWNER, product).unwrap();
    let lines = [
        GstBillLineInput {
            item: "Stocked Cola",
            product: Some(product),
            quantity: 5,
            ..COLA
        },
        // `pid` is filled from the product name when the line has none.
        GstBillLineInput {
            item: "Stocked Cola",
            quantity: 2,
            ..COLA
        },
    ];
    save(&mut fixture, &gst_draft(&lines));
    fixture.cancel_gst_bill(OWNER, ID).unwrap();
    let saved = &fixture.gst_bills[&ID];
    assert_eq!(
        saved
            .lines
            .iter()
            .map(|line| line.product)
            .collect::<Vec<_>>(),
        [Some(product), Some(product)]
    );
    assert_eq!(fixture.product_stock(product), stock);
    assert_eq!(fixture.open_stock_lots(OWNER, product).unwrap(), lots);
    assert_eq!(
        fixture.catalog.lot_snapshot(product),
        before.catalog.lot_snapshot(product)
    );
    fixture.delete_gst_bill(OWNER, ID).unwrap();
    assert_eq!(fixture.product_stock(product), stock);
    assert_ordinary_unchanged(&fixture, &before);
}

/// `fillGstItem` (HTML L5216-5228) with `gstInfo` (L4872-4922) for HSN and rate.
#[test]
fn standalone_gst_fill_item_reads_pack_size_sell_price_and_gst_info() {
    let mut fixture = gst_invoice::fixture(3_000);
    let cola = ProductId::new(1);
    let mut entry = fixture.open_product_entry(OWNER, cola).unwrap();
    entry.item = "Cola".into();
    entry.tax.hsn = Some("22021010".into());
    fixture.save_product_entry(OWNER, &entry).unwrap();
    fixture
        .set_checkout_product(
            cola,
            ProductTerms {
                pack: NonZeroU32::new(24).unwrap(),
                piece_price: Money::from_minor_units(500),
                case_price: Money::from_minor_units(10_500),
            },
        )
        .unwrap();
    let before = fixture.clone();

    let picked = fixture.fill_gst_item(OWNER, "Cola", None).unwrap();
    assert_eq!(
        (
            picked.product,
            picked.packet_qty,
            picked.sell_price,
            picked.hsn.as_str(),
            picked.gst_basis_points
        ),
        (
            Some(cola),
            Some(24),
            Some(Money::from_minor_units(10_500)),
            "22021010",
            4_000
        )
    );
    let blank = picked.apply("Cola", GstBillLineInput::BLANK);
    assert_eq!(
        (
            blank.inclusive_rate.minor_units(),
            blank.packet_qty,
            blank.hsn
        ),
        (10_500, 24, "22021010")
    );
    let priced = picked.apply("Cola", COLA);
    assert_eq!(priced.inclusive_rate, COLA.inclusive_rate);

    // A picked product ID wins for pack and price; `gstInfo` still reads the typed name.
    let by_id = fixture
        .fill_gst_item(OWNER, "Custom name", Some(ProductId::new(2)))
        .unwrap();
    assert_eq!(
        (
            by_id.product,
            by_id.packet_qty,
            by_id.hsn.as_str(),
            by_id.gst_basis_points
        ),
        (Some(ProductId::new(2)), Some(1), "22029090", 4_000)
    );
    let unknown = fixture
        .fill_gst_item(OWNER, "Synthetic Water", None)
        .unwrap();
    assert_eq!(
        (unknown.product, unknown.packet_qty, unknown.sell_price),
        (None, None, None)
    );
    assert_eq!(
        (unknown.hsn.as_str(), unknown.gst_basis_points),
        ("22011010", 500)
    );
    assert_eq!(
        fixture.fill_gst_item(Role::Manager, "Cola", None),
        Err(GstBillError::AccessDenied)
    );
    assert_ordinary_unchanged(&fixture, &before);
    assert_eq!(fixture, before);
}
