use super::*;

#[test]
fn l05_02_private_fifo_costs_do_not_change_projection_or_captured_documents() {
    let first = l05_02::estimate_fixture_with_costs(&[
        (1, Money::from_minor_units(4_321)),
        (4, Money::from_minor_units(5_678)),
    ]);
    let second = l05_02::estimate_fixture_with_costs(&[
        (1, Money::from_minor_units(1_001)),
        (4, Money::from_minor_units(2_002)),
    ]);
    assert_ne!(
        first.catalog.allocated_cost(DraftId::new(1)),
        second.catalog.allocated_cost(DraftId::new(1))
    );
    let before_first = first.clone();
    let before_second = second.clone();
    for balance_mode in [BalanceMode::AsOnBillDate, BalanceMode::Latest] {
        assert_eq!(
            first.bill_preview(Role::Owner, DraftId::new(1), balance_mode),
            second.bill_preview(Role::Owner, DraftId::new(1), balance_mode)
        );
        let request = l05_02::request(Role::Owner, balance_mode);
        let mut first_print = Vec::new();
        let mut second_print = Vec::new();
        first
            .print_estimate(request, &mut |popup| first_print.push(popup))
            .unwrap();
        second
            .print_estimate(request, &mut |popup| second_print.push(popup))
            .unwrap();
        assert_eq!(first_print.len(), 1);
        assert_eq!(first_print, second_print);

        let mut first_pdf = Vec::new();
        let mut second_pdf = Vec::new();
        first
            .save_estimate_as_pdf(request, &mut |popup| first_pdf.push(popup))
            .unwrap();
        second
            .save_estimate_as_pdf(request, &mut |popup| second_pdf.push(popup))
            .unwrap();
        assert_eq!(first_pdf.len(), 1);
        assert_eq!(first_pdf, second_pdf);
    }
    assert_eq!(first, before_first);
    assert_eq!(second, before_second);
}
