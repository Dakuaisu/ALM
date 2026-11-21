use super::*;

fn header() -> GstCalculationHeader<'static> {
    GstCalculationHeader {
        invoice: 1,
        date: BillDate::new(2025, 2, 21).unwrap(),
        customer: "Synthetic Buyer",
        cancelled: true,
        jurisdiction: GstJurisdiction::Intra,
        line_count: 2,
        items_total: Money::from_minor_units(41_000),
        loading: Money::from_minor_units(0),
        total: Money::from_minor_units(41_000),
    }
}

#[test]
fn resolved_lines_produce_exact_groups_in_order() {
    let lines = [
        GstResolvedLine {
            product: ProductId::new(1),
            name: "Synthetic A",
            quantity: NonZeroU32::new(2).unwrap(),
            hsn: "22029920",
            rate: 1_800,
            cess: 0,
            price: Money::from_minor_units(17_700),
            amount: Money::from_minor_units(35_400),
        },
        GstResolvedLine {
            product: ProductId::new(2),
            name: "Synthetic B",
            quantity: NonZeroU32::new(1).unwrap(),
            hsn: "21069099",
            rate: 1_200,
            cess: 0,
            price: Money::from_minor_units(5_600),
            amount: Money::from_minor_units(5_600),
        },
    ];
    let invoice = calculate_gst_invoice(header(), lines.into_iter().map(Ok)).unwrap();
    assert_eq!(
        invoice
            .groups
            .iter()
            .map(|group| (
                group.hsn.as_str(),
                group.combined_rate,
                group.taxable.minor_units(),
                group.tax.minor_units()
            ))
            .collect::<Vec<_>>(),
        [
            ("22029920", 1_800, 30_000, 5_400),
            ("21069099", 1_200, 5_000, 600)
        ]
    );
    assert_eq!(invoice.taxable.minor_units(), 35_000);
    assert_eq!(invoice.cgst.minor_units(), 3_000);
    assert_eq!(invoice.sgst.minor_units(), 3_000);
    assert!(invoice.cancelled);
}

#[test]
fn header_failures_precede_line_resolution() {
    let mut invalid = header();
    invalid.loading = Money::from_minor_units(1);
    invalid.total = Money::from_minor_units(1);
    let inputs = [Err(GstError::Overflow), Err(GstError::Overflow)];
    assert_eq!(
        calculate_gst_invoice(invalid, inputs.into_iter()),
        Err(GstError::InvalidMetadata)
    );
    let mut rounding = header();
    rounding.total = Money::from_minor_units(1);
    let inputs = [Err(GstError::Overflow), Err(GstError::Overflow)];
    assert_eq!(
        calculate_gst_invoice(rounding, inputs.into_iter()),
        Err(GstError::UnsupportedRounding)
    );
}
