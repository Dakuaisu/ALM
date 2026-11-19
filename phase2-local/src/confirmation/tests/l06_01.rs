use super::*;
use crate::DraftCustomerId;

pub(super) fn fixture(cost: i64) -> ConfirmationFixture {
    let mut f = ConfirmationFixture::with_ordered_lots(
        ProductId::new(1),
        &[(5, Money::from_minor_units(cost))],
        Money::from_minor_units(10_500),
    )
    .unwrap();
    for (id, price) in [(2, 12_000), (3, 11_200)] {
        f.add_product(ProductId::new(id), 5, Money::from_minor_units(price))
            .unwrap();
        f.catalog
            .seed_lots(
                ProductId::new(id),
                &[(
                    5,
                    Money::from_minor_units(cost + i64::try_from(id).unwrap()),
                )],
            )
            .unwrap();
    }
    let lines = [(1, 2), (2, 1), (3, 1)]
        .into_iter()
        .map(|(id, qty)| DraftLine::new(ProductId::new(id), NonZeroU32::new(qty).unwrap()))
        .collect();
    let draft = dated_named_draft(DraftId::new(1), lines).with_customer(DraftCustomer::Selected {
        id: DraftCustomerId::new("customer".into()),
        name: "Fixture buyer".into(),
    });
    f.register_draft(draft).unwrap();
    let review = f
        .review_named(
            Role::Owner,
            DraftId::new(1),
            Money::from_minor_units(10_000),
        )
        .unwrap();
    f.confirm(Role::Owner, RequestId::new(1), &review).unwrap();
    f
}

pub(super) fn metadata() -> [GstLineMetadata<'static>; 3] {
    [
        GstLineMetadata {
            product: ProductId::new(1),
            name: "Water",
            product_rate: None,
            product_cess: 0,
            product_hsn: None,
            line_rate: None,
            line_hsn: None,
        },
        GstLineMetadata {
            product: ProductId::new(2),
            name: "Product",
            product_rate: Some(1800),
            product_cess: 200,
            product_hsn: Some("22029920"),
            line_rate: None,
            line_hsn: None,
        },
        GstLineMetadata {
            product: ProductId::new(3),
            name: "Override",
            product_rate: Some(1800),
            product_cess: 500,
            product_hsn: Some("22029090"),
            line_rate: Some(1200),
            line_hsn: Some("21069099"),
        },
    ]
}

#[test]
fn l06_01_emits_exact_posted_invoice_in_both_jurisdictions_without_private_costs() {
    let f = fixture(4321);
    let other = fixture(9876);
    let before = f.clone();
    let other_before = other.clone();
    for jurisdiction in [GstJurisdiction::Intra, GstJurisdiction::Inter] {
        let request = GstRequest {
            role: Role::Owner,
            draft_id: DraftId::new(1),
            jurisdiction,
            lines: &metadata(),
        };
        let mut output = Vec::new();
        let mut other_output = Vec::new();
        f.emit_gst_invoice(request, &mut |invoice| output.push(invoice))
            .unwrap();
        other
            .emit_gst_invoice(request, &mut |invoice| other_output.push(invoice))
            .unwrap();
        assert_eq!((output.len(), other_output.len()), (1, 1));
        assert_eq!(output, other_output);
        let invoice = &output[0];
        assert_eq!(invoice.jurisdiction, jurisdiction);
        assert!(!invoice.cancelled);
        assert_eq!(
            invoice
                .lines
                .iter()
                .map(|l| (
                    l.product,
                    l.name.as_str(),
                    l.quantity.get(),
                    l.inclusive_rate.minor_units()
                ))
                .collect::<Vec<_>>(),
            vec![
                (ProductId::new(1), "Water", 2, 10_500),
                (ProductId::new(2), "Product", 1, 12_000),
                (ProductId::new(3), "Override", 1, 11_200)
            ]
        );
        assert_eq!(
            (invoice.invoice, invoice.customer.as_str(), invoice.date),
            (1, "Fixture buyer", BillDate::new(2024, 2, 29).unwrap())
        );
        assert_eq!(
            (
                invoice.total.minor_units(),
                invoice.taxable.minor_units(),
                invoice.tax.minor_units()
            ),
            (44_200, 40_000, 4_200)
        );
        assert_eq!(
            (
                invoice.cgst.minor_units(),
                invoice.sgst.minor_units(),
                invoice.igst.minor_units()
            ),
            match jurisdiction {
                GstJurisdiction::Intra => (2100, 2100, 0),
                GstJurisdiction::Inter => (0, 0, 4200),
            }
        );
        assert_eq!(
            invoice
                .lines
                .iter()
                .map(|l| (
                    l.hsn.as_str(),
                    l.rate,
                    l.cess,
                    l.taxable.minor_units(),
                    l.inclusive.minor_units(),
                    l.taxable_rate.minor_units()
                ))
                .collect::<Vec<_>>(),
            vec![
                ("22011010", 500, 0, 20_000, 21_000, 10_000),
                ("22029920", 1800, 200, 10_000, 12_000, 10_000),
                ("21069099", 1200, 0, 10_000, 11_200, 10_000)
            ]
        );
        assert_eq!(
            invoice
                .groups
                .iter()
                .map(|g| (
                    g.hsn.as_str(),
                    g.combined_rate,
                    g.taxable.minor_units(),
                    g.tax.minor_units()
                ))
                .collect::<Vec<_>>(),
            vec![
                ("22011010", 500, 20_000, 1000),
                ("22029920", 2000, 10_000, 2000),
                ("21069099", 1200, 10_000, 1200)
            ]
        );
    }
    assert_eq!(f, before);
    assert_eq!(other, other_before);
    assert_eq!(
        (
            f.product_stock(ProductId::new(1)),
            f.product_stock(ProductId::new(2)),
            f.product_stock(ProductId::new(3))
        ),
        (Some(3), Some(4), Some(4))
    );
    assert_eq!(
        (
            f.ledger().minor_units(),
            f.received_ledger().minor_units(),
            f.customer_balances.get("customer")
        ),
        (44_200, 10_000, Some(&34_200))
    );
}

#[test]
fn l06_01_same_hsn_with_different_rates_remains_separate() {
    let f = fixture(4321);
    let mut lines = metadata();
    lines[1].product_hsn = Some("22011010");
    let request = GstRequest {
        role: Role::Owner,
        draft_id: DraftId::new(1),
        jurisdiction: GstJurisdiction::Inter,
        lines: &lines,
    };
    let mut output = Vec::new();
    f.emit_gst_invoice(request, &mut |invoice| output.push(invoice))
        .unwrap();
    assert_eq!(output.len(), 1);
    assert_eq!(
        output[0]
            .groups
            .iter()
            .map(|g| (g.hsn.as_str(), g.combined_rate))
            .collect::<Vec<_>>(),
        vec![("22011010", 500), ("22011010", 2000), ("21069099", 1200)]
    );
}

#[test]
fn l06_01_manager_denied_before_sink() {
    let f = fixture(4321);
    let before = f.clone();
    let mut output = Vec::new();
    let request = GstRequest {
        role: Role::Manager,
        draft_id: DraftId::new(1),
        jurisdiction: GstJurisdiction::Intra,
        lines: &metadata(),
    };
    assert_eq!(
        f.emit_gst_invoice(request, &mut |invoice| output.push(invoice)),
        Err(GstError::Preview(ConfirmationError::AccessDenied))
    );
    assert!(output.is_empty());
    assert_eq!(f, before);
}

#[test]
fn l06_01_unrepresentable_taxable_rate_emits_nothing() {
    let f = fixture(4321);
    let mut lines = metadata();
    lines[0].line_rate = Some(600);
    let mut output = Vec::new();
    let request = GstRequest {
        role: Role::Owner,
        draft_id: DraftId::new(1),
        jurisdiction: GstJurisdiction::Inter,
        lines: &lines,
    };
    assert_eq!(
        f.emit_gst_invoice(request, &mut |invoice| output.push(invoice)),
        Err(GstError::UnsupportedRounding)
    );
    assert!(output.is_empty());
}
