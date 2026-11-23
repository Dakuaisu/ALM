use super::*;
use crate::DraftCustomerId;

fn assert_rejected(f: &ConfirmationFixture, error: EstimateError) {
    let before = f.clone();
    let request = l05_02::request(Role::Owner, BalanceMode::Latest);
    let mut captured = Vec::new();
    assert_eq!(
        f.print_estimate(request, &mut |popup| captured.push(popup)),
        Err(error)
    );
    assert_eq!(
        f.save_estimate_as_pdf(request, &mut |popup| captured.push(popup)),
        Err(error)
    );
    assert!(captured.is_empty());
    assert_eq!(*f, before);
}

#[test]
fn l05_02_each_action_uses_its_explicit_synthetic_today() {
    let f = l05_02::estimate_fixture_with_costs(&[(5, Money::from_minor_units(1_001))]);
    let before = f.clone();
    let base = l05_02::request(Role::Owner, BalanceMode::Latest);
    let print = EstimateRequest {
        display: EstimateDisplay {
            today: BillDate::new(2025, 1, 2).unwrap(),
            ..base.display
        },
        ..base
    };
    let pdf = EstimateRequest {
        display: EstimateDisplay {
            today: BillDate::new(2026, 12, 31).unwrap(),
            ..base.display
        },
        ..base
    };
    let mut print_capture = Vec::new();
    let mut pdf_capture = Vec::new();
    f.print_estimate(print, &mut |popup| print_capture.push(popup))
        .unwrap();
    f.save_estimate_as_pdf(pdf, &mut |popup| pdf_capture.push(popup))
        .unwrap();
    assert_eq!(print_capture.len(), 1);
    assert_eq!(pdf_capture.len(), 1);
    assert!(
        print_capture[0]
            .html
            .contains("<th>Total Due (as on 02-Jan-2025)</th><td>₹90.00</td>")
    );
    assert!(
        pdf_capture[0]
            .html
            .contains("<th>Total Due (as on 31-Dec-2026)</th><td>₹90.00</td>")
    );
    assert_eq!(f, before);
}

#[test]
fn l05_02_missing_or_mismatched_line_labels_never_emit() {
    let f = l05_02::estimate_fixture_with_costs(&[(5, Money::from_minor_units(1_001))]);
    let before = f.clone();
    let wrong = [EstimateLineDisplay {
        product: ProductId::new(2),
        label: "Wrong",
    }];
    let blank = [EstimateLineDisplay {
        product: ProductId::new(1),
        label: "  ",
    }];
    let extra = [
        EstimateLineDisplay {
            product: ProductId::new(1),
            label: "Fixture",
        },
        EstimateLineDisplay {
            product: ProductId::new(1),
            label: "Extra",
        },
    ];
    let base = l05_02::request(Role::Owner, BalanceMode::Latest);
    let mut captured = Vec::new();
    for lines in [&[][..], &wrong[..], &blank[..], &extra[..]] {
        let request = EstimateRequest {
            display: EstimateDisplay {
                lines,
                ..base.display
            },
            ..base
        };
        assert_eq!(
            f.print_estimate(request, &mut |popup| captured.push(popup)),
            Err(EstimateError::InvalidLineDisplay)
        );
        assert_eq!(
            f.save_estimate_as_pdf(request, &mut |popup| captured.push(popup)),
            Err(EstimateError::InvalidLineDisplay)
        );
    }
    assert!(captured.is_empty());
    assert_eq!(f, before);
}

#[test]
fn l05_02_unexplained_total_difference_never_emits_zero_loading() {
    let mut f = l05_02::estimate_fixture_with_costs(&[(5, Money::from_minor_units(1_001))]);
    let (_, receipt, _) = f.current_sales.get_mut(&DraftId::new(1)).unwrap();
    receipt.total = Money::from_minor_units(26_000);
    assert_rejected(&f, EstimateError::Preview(ConfirmationError::InvalidDraft));
}

#[test]
fn l05_02_checkout_with_nonzero_loading_remains_unsupported() {
    let mut f = l05_01::preview_fixture(5);
    f.register_draft(draft(1, 2).with_customer(DraftCustomer::Selected {
        id: DraftCustomerId::new("a".into()),
        name: "Fixture customer".into(),
    }))
    .unwrap();
    f.set_checkout_product(
        ProductId::new(1),
        ProductTerms {
            pack: NonZeroU32::new(1).unwrap(),
            piece_price: Money::from_minor_units(12_500),
            case_price: Money::from_minor_units(12_500),
        },
    )
    .unwrap();
    f.set_loading_rate(Money::from_minor_units(350)).unwrap();
    let review = f
        .review_checkout(
            Role::Owner,
            DraftId::new(1),
            &[SaleUnit::Case],
            Money::from_minor_units(10_000),
        )
        .unwrap();
    assert_eq!(
        review.amounts().unwrap().loading,
        Money::from_minor_units(700)
    );
    f.confirm(Role::Owner, RequestId::new(1), &review).unwrap();
    assert_rejected(&f, EstimateError::Preview(ConfirmationError::InvalidDraft));
}

#[test]
fn l05_02_mixed_lines_use_ordered_labels_and_saved_item_totals() {
    let mut f = l05_02::estimate_fixture_with_costs(&[(5, Money::from_minor_units(1_001))]);
    f.add_product(ProductId::new(2), 3, Money::from_minor_units(9_000))
        .unwrap();
    let edit = f
        .begin_edit(
            Role::Owner,
            DraftId::new(1),
            EditChanges {
                retained_price: Money::from_minor_units(15_000),
                appended_line: DraftLine::new(ProductId::new(2), NonZeroU32::new(1).unwrap()),
                appended_price: Money::from_minor_units(9_000),
            },
        )
        .unwrap();
    f.save_edit(Role::Owner, &edit).unwrap();
    let before = f.clone();
    let lines = [
        EstimateLineDisplay {
            product: ProductId::new(1),
            label: "First fixture item",
        },
        EstimateLineDisplay {
            product: ProductId::new(2),
            label: "Second fixture item",
        },
    ];
    for mode in [BalanceMode::AsOnBillDate, BalanceMode::Latest] {
        let base = l05_02::request(Role::Owner, mode);
        let request = EstimateRequest {
            display: EstimateDisplay {
                lines: &lines,
                ..base.display
            },
            ..base
        };
        let mut print = Vec::new();
        let mut pdf = Vec::new();
        f.print_estimate(request, &mut |popup| print.push(popup))
            .unwrap();
        f.save_estimate_as_pdf(request, &mut |popup| pdf.push(popup))
            .unwrap();
        assert_eq!(print.len(), 1);
        assert_eq!(pdf.len(), 1);
        assert_eq!(print[0].html, pdf[0].html);
        assert!(
            print[0]
                .html
                .contains("<td>First fixture item</td><td>2</td><td>₹150.00</td><td>₹300.00</td>")
        );
        assert!(
            print[0]
                .html
                .contains("<td>Second fixture item</td><td>1</td><td>₹90.00</td><td>₹90.00</td>")
        );
        assert!(
            print[0]
                .html
                .contains("<th colspan=\"2\">Items Total</th><td>3</td><td>₹390.00</td>")
        );
        assert!(
            print[0]
                .html
                .contains("<th>Loading/Delivery</th><td>₹0.00</td>")
        );
    }
    let base = l05_02::request(Role::Owner, BalanceMode::Latest);
    let reversed = [lines[1], lines[0]];
    let request = EstimateRequest {
        display: EstimateDisplay {
            lines: &reversed,
            ..base.display
        },
        ..base
    };
    let mut captured = Vec::new();
    assert_eq!(
        f.print_estimate(request, &mut |popup| captured.push(popup)),
        Err(EstimateError::InvalidLineDisplay)
    );
    assert!(captured.is_empty());
    assert_eq!(f, before);
}
