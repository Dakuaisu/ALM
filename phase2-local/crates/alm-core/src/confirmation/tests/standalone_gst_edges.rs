//! Legacy case L06-02.
use super::*;

#[test]
fn standalone_gst_rejects_fractional_rupee_without_writes_or_emission() {
    let mut fixture = gst_invoice::fixture(3_000);
    let before = fixture.clone();
    let lines = [GstBillLineInput {
        inclusive_rate: Money::from_minor_units(101),
        gst_basis_points: 0,
        quantity: 1,
        ..WATER
    }];
    let draft = gst_draft(&lines);
    assert_eq!(
        fixture.gst_bill_form(Role::Owner, &draft),
        Err(GstBillError::UnsupportedRounding)
    );
    assert_eq!(
        fixture.save_gst_bill(Role::Owner, &draft, &mut |_| panic!("unexpected emission")),
        Err(GstBillError::UnsupportedRounding)
    );
    assert_eq!(fixture, before);
}

#[test]
fn standalone_gst_view_calculates_from_saved_amount_and_rate() {
    let mut fixture = gst_invoice::fixture(3_000);
    fixture
        .save_gst_bill(Role::Owner, &gst_draft(LINES), &mut |_| {})
        .unwrap();
    let record = fixture.gst_bills.get_mut(&ID).unwrap();
    record.lines[0].amount = Money::from_minor_units(42_000);
    record.lines[0].gst_basis_points = 4_000;
    let mut emitted = Vec::new();
    fixture
        .emit_gst_bill(Role::Owner, ID, &mut |invoice| {
            emitted.push(invoice.clone())
        })
        .unwrap();
    assert_eq!(amounts(emitted[0].totals), [40_000, 0, 0, 12_500, 52_500]);
    assert_eq!(emitted[0].record.lines[0].quantity.get(), 2);
}
