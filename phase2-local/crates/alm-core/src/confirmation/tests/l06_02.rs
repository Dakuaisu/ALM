use super::*;

#[path = "l06_02_edges.rs"]
mod edges;

const ID: GstBillId = GstBillId(1);
const NUMBER: GstBillNumber = GstBillNumber(700);
const COLA: GstBillLineInput<'static> = GstBillLineInput {
    item: "Cola",
    quantity: 2,
    inclusive_rate: Money::from_minor_units(14_000),
    gst_basis_points: 4_000,
};
const WATER: GstBillLineInput<'static> = GstBillLineInput {
    item: "Water",
    quantity: 1,
    inclusive_rate: Money::from_minor_units(10_500),
    gst_basis_points: 500,
};
const LINES: &[GstBillLineInput<'static>] = &[COLA, WATER];

fn gst_draft<'a>(lines: &'a [GstBillLineInput<'a>]) -> GstBillDraft<'a> {
    GstBillDraft {
        id: ID,
        number: NUMBER,
        customer: "Standalone buyer",
        date: test_bill_date(),
        gstin: None,
        eway_number: None,
        jurisdiction: GstJurisdiction::Inter,
        lines,
    }
}

fn amounts(totals: GstBillTotals) -> [i64; 5] {
    [
        totals.taxable,
        totals.cgst,
        totals.sgst,
        totals.igst,
        totals.total,
    ]
    .map(Money::minor_units)
}

fn assert_ordinary_unchanged(fixture: &ConfirmationFixture, before: &ConfirmationFixture) {
    let mut actual = fixture.clone();
    actual.gst_bills.clone_from(&before.gst_bills);
    assert_eq!(&actual, before);
}

#[test]
fn standalone_gst_saved_edited_and_cancelled_without_ordinary_effects() {
    let mut fixture = l06_01::fixture(3_000);
    let before = fixture.clone();
    let lines = [COLA, WATER, GstBillLineInput { item: "", ..WATER }];
    let draft = gst_draft(&lines);
    let mut emitted = Vec::new();

    assert_eq!(
        amounts(fixture.gst_bill_form(Role::Owner, &draft).unwrap()),
        [40_000, 4_500, 4_500, 0, 49_000]
    );
    assert_eq!(fixture, before);
    fixture
        .save_gst_bill(Role::Owner, &draft, &mut |bill| emitted.push(bill.clone()))
        .unwrap();
    assert_ordinary_unchanged(&fixture, &before);
    let record = fixture.gst_bills.get(&ID).unwrap();
    assert_eq!(record.lines.len(), 2);
    assert_eq!(record.lines[0].amount.minor_units(), 28_000);
    assert_eq!(record.lines[0].gst_basis_points, 4_000);
    assert_eq!(record.lines[1].amount.minor_units(), 10_500);
    assert_eq!(record.lines[1].gst_basis_points, 500);
    assert_eq!(emitted.len(), 1);
    let saved = &emitted[0];
    assert_eq!(record, &saved.record);
    assert_eq!(
        (saved.record.id, saved.record.number, saved.record.cancelled),
        (ID, NUMBER, false)
    );
    assert_eq!(saved.record.jurisdiction, GstJurisdiction::Inter);
    assert_eq!(amounts(saved.totals), [30_000, 0, 0, 8_500, 38_500]);
    assert_eq!(
        (
            saved.record.lines[0].item.as_str(),
            saved.record.lines[0].quantity.get()
        ),
        ("Cola", 2)
    );

    let edited_lines = [
        GstBillLineInput {
            quantity: 3,
            ..COLA
        },
        WATER,
    ];
    let edit = gst_draft(&edited_lines);
    assert_eq!(
        amounts(fixture.gst_bill_form(Role::Owner, &edit).unwrap()),
        [40_000, 6_250, 6_250, 0, 52_500]
    );
    fixture
        .save_gst_bill(Role::Owner, &edit, &mut |bill| emitted.push(bill.clone()))
        .unwrap();
    assert_ordinary_unchanged(&fixture, &before);
    assert_eq!(fixture.gst_bills.len(), 1);
    assert_eq!(emitted.len(), 2);
    let edited = &emitted[1];
    assert_eq!(fixture.gst_bills.get(&ID), Some(&edited.record));
    assert_eq!(
        (
            edited.record.id,
            edited.record.number,
            edited.record.cancelled
        ),
        (ID, NUMBER, false)
    );
    assert_eq!(amounts(edited.totals), [40_000, 0, 0, 12_500, 52_500]);
    assert_eq!(edited.record.lines[0].quantity.get(), 3);
    assert_eq!(edited.record.lines[1], emitted[0].record.lines[1]);

    fixture.cancel_gst_bill(Role::Owner, ID).unwrap();
    assert_ordinary_unchanged(&fixture, &before);
    assert_eq!(fixture.gst_bills.len(), 1);
    let mut cancelled = edited.clone();
    cancelled.record.cancelled = true;
    assert_eq!(fixture.gst_bills.get(&ID), Some(&cancelled.record));
    fixture
        .emit_gst_bill(Role::Owner, ID, &mut |bill| emitted.push(bill.clone()))
        .unwrap();
    assert_eq!(emitted.len(), 3);
    assert_eq!(emitted[2], cancelled);
    let after_cancel = fixture.clone();
    assert_eq!(
        fixture.save_gst_bill(Role::Owner, &draft, &mut |_| panic!(
            "cancelled edit emitted"
        )),
        Err(GstBillError::Cancelled)
    );
    assert_eq!(fixture, after_cancel);
}

#[test]
fn standalone_gst_invalid_save_warns_without_writing_or_emitting() {
    let mut fixture = l06_01::fixture(3_000);
    let draft = gst_draft(LINES);
    fixture
        .save_gst_bill(Role::Owner, &draft, &mut |_| {})
        .unwrap();
    let before = fixture.clone();
    let no_item_or_qty = [
        GstBillLineInput { item: "", ..COLA },
        GstBillLineInput {
            quantity: 0,
            ..WATER
        },
    ];
    for (invalid, warning) in [
        (gst_draft(&[]), "Add at least one item with quantity"),
        (
            gst_draft(&no_item_or_qty),
            "Add at least one item with quantity",
        ),
        (
            GstBillDraft {
                customer: "",
                ..draft
            },
            "Enter the customer name",
        ),
        (
            GstBillDraft {
                date: None,
                ..draft
            },
            "Pick the invoice date",
        ),
        (
            GstBillDraft {
                gstin: Some("INVALID"),
                ..draft
            },
            "GSTIN must be exactly 15 characters",
        ),
        (
            GstBillDraft {
                eway_number: Some("1234-5678-901X"),
                ..draft
            },
            "E-Way Bill number must be 12 digits",
        ),
    ] {
        let error = fixture
            .save_gst_bill(Role::Owner, &invalid, &mut |_| {
                panic!("invalid edit emitted")
            })
            .unwrap_err();
        assert_eq!(error.warning(), Some(warning));
        assert_eq!(fixture, before);
    }
}

#[test]
fn standalone_gst_is_owner_only_and_intrastate_invoice_splits_tax() {
    let mut fixture = l06_01::fixture(3_000);
    let draft = GstBillDraft {
        jurisdiction: GstJurisdiction::Intra,
        gstin: Some(" 09ABCDE1234F1Z5 "),
        eway_number: Some("1234-5678-9012"),
        ..gst_draft(LINES)
    };
    fixture
        .save_gst_bill(Role::Owner, &draft, &mut |bill| {
            assert_eq!(amounts(bill.totals), [30_000, 4_250, 4_250, 0, 38_500]);
        })
        .unwrap();
    let before = fixture.clone();
    assert_eq!(
        fixture.gst_bill_form(Role::Manager, &draft),
        Err(GstBillError::AccessDenied)
    );
    assert_eq!(
        fixture.save_gst_bill(Role::Manager, &draft, &mut |_| panic!(
            "manager save emitted"
        )),
        Err(GstBillError::AccessDenied)
    );
    assert_eq!(
        fixture.emit_gst_bill(Role::Manager, ID, &mut |_| panic!("manager read emitted")),
        Err(GstBillError::AccessDenied)
    );
    assert_eq!(
        fixture.cancel_gst_bill(Role::Manager, ID),
        Err(GstBillError::AccessDenied)
    );
    assert_eq!(fixture, before);
}
