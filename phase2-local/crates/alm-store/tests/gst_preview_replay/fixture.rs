use alm_core::confirmation::{
    ConfirmationFixture, GstBillDraft, GstBillId, GstBillInvoice, GstBillLineInput, GstBillNumber,
};
use alm_core::{BillDate, Money, ProductId, Role};

pub fn reopened(count: usize) -> GstBillInvoice {
    let names: Vec<_> = (1..=count).map(|id| format!("ITEM-{id:03}")).collect();
    let lines: Vec<_> = names
        .iter()
        .map(|name| GstBillLineInput {
            item: name,
            quantity: 1,
            inclusive_rate: Money::from_minor_units(11_800),
            gst_basis_points: 1_800,
            ..GstBillLineInput::BLANK
        })
        .collect();
    let draft = GstBillDraft {
        id: GstBillId(91),
        number: GstBillNumber(47),
        customer: "Synthetic <Buyer> & \"Co\"",
        date: Some(BillDate::new(2025, 1, 2).unwrap()),
        gstin: None,
        eway_number: None,
        lines: &lines,
        ..GstBillDraft::default()
    };
    let mut fixture =
        ConfirmationFixture::new(ProductId::new(1), 0, Money::from_minor_units(0)).unwrap();
    let mut saved = None;
    fixture
        .save_gst_bill(Role::Owner, &draft, &mut |invoice| {
            saved = Some(invoice.clone());
        })
        .unwrap();
    let mut reopened = None;
    fixture
        .emit_gst_bill(Role::Owner, draft.id, &mut |invoice| {
            reopened = Some(invoice.clone());
        })
        .unwrap();
    assert_eq!(saved, reopened);
    let invoice = reopened.unwrap();
    assert_eq!(invoice.record.lines.len(), count);
    invoice
}
