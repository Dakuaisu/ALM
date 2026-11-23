//! Legacy case L06-02: `gstInvoiceHTML` (HTML L4989-5134) values worked out by hand.
use super::*;
use crate::confirmation::{GstCustomerContact, GstHsnSummary, GstInvoiceLine, GstUnit};

const OWNER: Role = Role::Owner;

fn line(
    hsn: &str,
    unit: GstUnit,
    quantity: u32,
    pieces: Option<u64>,
    rates: [i64; 3],
) -> GstInvoiceLine {
    let [inclusive, taxable_rate, taxable] = rates.map(Money::from_minor_units);
    GstInvoiceLine {
        hsn: hsn.into(),
        unit,
        quantity,
        pieces,
        inclusive_rate: inclusive,
        taxable_rate,
        taxable_amount: taxable,
    }
}

fn summary(hsn: &str, rate: u32, amounts: [i64; 4]) -> GstHsnSummary {
    let [taxable, half_tax, integrated_tax, total_tax] = amounts.map(Money::from_minor_units);
    GstHsnSummary {
        hsn: hsn.into(),
        gst_basis_points: rate,
        taxable,
        half_tax,
        integrated_tax,
        total_tax,
    }
}

/// Product 1 is "Cola" with an HSN override, so a blank Cola HSN prints `22021010`.
fn named_fixture() -> ConfirmationFixture {
    let mut fixture = gst_invoice::fixture(3_000);
    let mut entry = fixture
        .open_product_entry(OWNER, ProductId::new(1))
        .unwrap();
    entry.item = "Cola".into();
    entry.tax.hsn = Some("22021010".into());
    fixture.save_product_entry(OWNER, &entry).unwrap();
    fixture
}

const GOLDEN: [GstBillLineInput<'static>; 4] = [
    GstBillLineInput {
        item: "Cola",
        quantity: 2,
        inclusive_rate: Money::from_minor_units(14_000),
        gst_basis_points: 4_000,
        packet_qty: 24,
        ..GstBillLineInput::BLANK
    },
    GstBillLineInput {
        item: "Pepsi",
        quantity: 1,
        inclusive_rate: Money::from_minor_units(7_000),
        gst_basis_points: 4_000,
        packet_qty: 12,
        ..GstBillLineInput::BLANK
    },
    GstBillLineInput {
        item: "Water",
        unit: GstUnit::Piece,
        quantity: 1,
        inclusive_rate: Money::from_minor_units(10_500),
        gst_basis_points: 500,
        ..GstBillLineInput::BLANK
    },
    GstBillLineInput {
        item: "Limca",
        hsn: "22021010",
        quantity: 1,
        inclusive_rate: Money::from_minor_units(7_000),
        gst_basis_points: 4_000,
        packet_qty: 0,
        ..GstBillLineInput::BLANK
    },
];

fn golden_draft(pos_state: &'static str, pos_code: &'static str) -> GstBillDraft<'static> {
    GstBillDraft {
        customer_id: "cust-1",
        pay_mode: "Credit",
        transporter: "Synthetic Roadways",
        vehicle_no: "UP32AB1234",
        lr_no: "LR-9",
        eway_number: Some("1234-5678-9012"),
        terms_delivery: "Door delivery",
        pos_state,
        pos_code,
        lines: &GOLDEN,
        ..gst_draft(&GOLDEN)
    }
}

fn emitted(fixture: &mut ConfirmationFixture, draft: &GstBillDraft<'_>) -> GstBillInvoice {
    let mut saved = None;
    fixture
        .save_gst_bill(OWNER, draft, &mut |invoice| saved = Some(invoice.clone()))
        .unwrap();
    let mut reprinted = None;
    fixture
        .emit_gst_bill(OWNER, draft.id, &mut |invoice| {
            reprinted = Some(invoice.clone())
        })
        .unwrap();
    assert_eq!(saved, reprinted);
    saved.unwrap()
}

fn golden_lines() -> Vec<GstInvoiceLine> {
    vec![
        // 140.00 / 1.40 = 100.00; 280.00 / 1.40 = 200.00; 2 cases x 24 = 48 PCS.
        line(
            "22021010",
            GstUnit::Case,
            2,
            Some(48),
            [14_000, 10_000, 20_000],
        ),
        line(
            "22029090",
            GstUnit::Case,
            1,
            Some(12),
            [7_000, 5_000, 5_000],
        ),
        // 105.00 / 1.05 = 100.00; a Piece line prints only "1 PCS".
        line(
            "22011010",
            GstUnit::Piece,
            1,
            None,
            [10_500, 10_000, 10_000],
        ),
        // A packet quantity of 0 is saved as 1.
        line("22021010", GstUnit::Case, 1, Some(1), [7_000, 5_000, 5_000]),
    ]
}

#[test]
fn standalone_gst_intrastate_invoice_prints_html_rows_summary_and_meta() {
    let mut fixture = named_fixture();
    let before = fixture.clone();
    let invoice = emitted(&mut fixture, &golden_draft("", ""));
    assert_eq!(invoice.jurisdiction(), GstJurisdiction::Intra);
    assert_eq!(invoice.invoice_number(), "ALM/23-24/700");
    assert_eq!(invoice.dated(), "29-Feb-2024");
    assert_eq!(invoice.lines().unwrap(), golden_lines());
    // hsnMap groups "22021010|40" (200 + 50) before "22029090|40" and "22011010|5".
    assert_eq!(
        invoice.hsn_summary().unwrap(),
        [
            summary("22021010", 4_000, [25_000, 5_000, 0, 10_000]),
            summary("22029090", 4_000, [5_000, 1_000, 0, 2_000]),
            summary("22011010", 500, [10_000, 250, 0, 500]),
        ]
    );
    assert_eq!(amounts(invoice.totals), [40_000, 6_250, 6_250, 0, 52_500]);
    assert_eq!(invoice.total_quantity(), 5);
    assert_eq!(
        invoice.meta(),
        [
            ("Mode of Payment", "Credit".to_owned()),
            ("Reverse Charge", "No".to_owned()),
            ("Place of Supply", "Uttar Pradesh (09)".to_owned()),
            ("Transporter", "Synthetic Roadways".to_owned()),
            ("Vehicle No.", "UP32AB1234".to_owned()),
            ("LR / GR No.", "LR-9".to_owned()),
            ("E-Way Bill No.", "1234-5678-9012".to_owned()),
        ]
    );
    assert_eq!(invoice.record.terms_delivery, "Door delivery");
    assert_eq!(invoice.record.lines[0].hsn, "");
    assert_eq!(invoice.record.lines[0].product, Some(ProductId::new(1)));
    assert_ordinary_unchanged(&fixture, &before);
}

#[test]
fn standalone_gst_place_of_supply_outside_09_prints_igst() {
    let mut fixture = named_fixture();
    let before = fixture.clone();
    let invoice = emitted(&mut fixture, &golden_draft("Maharashtra", "27"));
    assert_eq!(invoice.jurisdiction(), GstJurisdiction::Inter);
    assert_eq!(invoice.lines().unwrap(), golden_lines());
    assert_eq!(
        invoice.hsn_summary().unwrap(),
        [
            summary("22021010", 4_000, [25_000, 0, 10_000, 10_000]),
            summary("22029090", 4_000, [5_000, 0, 2_000, 2_000]),
            summary("22011010", 500, [10_000, 0, 500, 500]),
        ]
    );
    assert_eq!(amounts(invoice.totals), [40_000, 0, 0, 12_500, 52_500]);
    assert_eq!(
        invoice.meta()[2],
        ("Place of Supply", "Maharashtra (27)".to_owned())
    );
    // Only the transport rows that are set are printed.
    let bare = emitted(
        &mut fixture,
        &GstBillDraft {
            id: GstBillId(2),
            ..gst_draft(LINES)
        },
    );
    assert_eq!(
        bare.meta(),
        [
            ("Mode of Payment", "Cash".to_owned()),
            ("Reverse Charge", "No".to_owned()),
            ("Place of Supply", "Maharashtra (27)".to_owned()),
        ]
    );
    // The form preview stays CGST/SGST only (`renderGstTot`).
    assert_eq!(
        amounts(
            fixture
                .gst_bill_form(OWNER, &golden_draft("Maharashtra", "27"))
                .unwrap()
        ),
        [40_000, 6_250, 6_250, 0, 52_500]
    );
    assert_ordinary_unchanged(&fixture, &before);
}

/// `cAddr`, `cPh` and `cGst` fall back to the selected customer (HTML L5062-5068).
#[test]
fn standalone_gst_buyer_block_falls_back_to_the_selected_customer() {
    let mut fixture = named_fixture();
    let before = fixture.clone();
    let invoice = emitted(&mut fixture, &golden_draft("", ""));
    let contact = GstCustomerContact {
        id: "cust-1".into(),
        addr1: "12 Market Road".into(),
        addr2: "Lucknow".into(),
        mobile: "9000000001".into(),
        gstin: "09ABCDE1234F1Z5".into(),
    };
    let party = invoice.party(Some(&contact));
    assert_eq!(
        [
            party.name.as_str(),
            &party.address,
            &party.phone,
            &party.email,
            &party.gstin,
            &party.state,
            &party.state_code,
        ],
        [
            "Standalone buyer",
            "12 Market Road, Lucknow",
            "9000000001",
            "",
            "09ABCDE1234F1Z5",
            "Uttar Pradesh",
            "09",
        ]
    );
    let other = GstCustomerContact {
        id: "cust-2".into(),
        ..contact.clone()
    };
    let unmatched = invoice.party(Some(&other));
    assert_eq!(
        (
            unmatched.address.as_str(),
            unmatched.phone.as_str(),
            unmatched.gstin.as_str()
        ),
        ("", "", "")
    );
    let own = emitted(
        &mut fixture,
        &GstBillDraft {
            id: GstBillId(3),
            cust_addr: "Own address",
            cust_phone: "9111111111",
            gstin: Some("27ABCDE1234F1Z5"),
            ..golden_draft("", "")
        },
    );
    let party = own.party(Some(&contact));
    assert_eq!(
        (
            party.address.as_str(),
            party.phone.as_str(),
            party.gstin.as_str()
        ),
        ("Own address", "9111111111", "27ABCDE1234F1Z5")
    );
    assert_ordinary_unchanged(&fixture, &before);
}
