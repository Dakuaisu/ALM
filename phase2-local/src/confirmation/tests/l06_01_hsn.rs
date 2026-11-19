use super::*;

#[test]
fn l06_01_hsn_groups_emit_jurisdiction_tax_amounts() {
    let fixture = l06_01::fixture(4321);
    for (jurisdiction, expected) in [
        (
            GstJurisdiction::Intra,
            [(500, 500, 0), (1000, 1000, 0), (600, 600, 0)],
        ),
        (
            GstJurisdiction::Inter,
            [(0, 0, 1000), (0, 0, 2000), (0, 0, 1200)],
        ),
    ] {
        let request = GstRequest {
            role: Role::Owner,
            draft_id: DraftId::new(1),
            jurisdiction,
            lines: &l06_01::metadata(),
        };
        let mut output = Vec::new();
        fixture
            .emit_gst_invoice(request, &mut |invoice| output.push(invoice))
            .unwrap();
        assert_eq!(output.len(), 1);
        let actual = output[0]
            .groups
            .iter()
            .map(|group| {
                (
                    group.cgst.minor_units(),
                    group.sgst.minor_units(),
                    group.igst.minor_units(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(actual, expected);
    }
}

#[test]
fn l06_01_hsn_inexact_group_split_rejects_before_emission() {
    let mut fixture =
        ConfirmationFixture::new(ProductId::new(1), 1, Money::from_minor_units(101)).unwrap();
    fixture
        .add_product(ProductId::new(2), 1, Money::from_minor_units(199))
        .unwrap();
    let lines = [1, 2]
        .into_iter()
        .map(|id| DraftLine::new(ProductId::new(id), NonZeroU32::new(1).unwrap()))
        .collect();
    let draft = dated_named_draft(DraftId::new(1), lines)
        .with_customer(DraftCustomer::Typed("Local - Neighbor".into()));
    fixture.register_draft(draft).unwrap();
    let review = fixture
        .review_walk_in_cash(Role::Owner, DraftId::new(1))
        .unwrap();
    fixture
        .confirm(Role::Owner, RequestId::new(1), &review)
        .unwrap();
    let mut lines = l06_01::metadata();
    lines[0].line_rate = Some(100);
    lines[1].line_rate = Some(9900);
    let request = GstRequest {
        role: Role::Owner,
        draft_id: DraftId::new(1),
        jurisdiction: GstJurisdiction::Intra,
        lines: &lines[..2],
    };
    let before = fixture.clone();
    let mut output = Vec::new();
    assert_eq!(
        fixture.emit_gst_invoice(request, &mut |invoice| output.push(invoice)),
        Err(GstError::UnsupportedRounding)
    );
    assert!(output.is_empty());
    assert_eq!(fixture, before);
}
