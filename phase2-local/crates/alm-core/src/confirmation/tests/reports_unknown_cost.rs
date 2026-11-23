//! Legacy case L16-01.
use super::*;

#[test]
fn l16_01_unknown_frozen_cost_is_distinguished_while_ranking_remains_available() {
    let product = ProductId::new(1);
    let id = DraftId::new(17);
    let mut fixture = ConfirmationFixture::new(product, 1, Money::from_minor_units(500)).unwrap();
    fixture
        .register_draft(
            ProvisionalDraft::new(
                id,
                vec![DraftLine::new(product, NonZeroU32::new(1).unwrap())],
            )
            .with_customer(DraftCustomer::Selected {
                id: DraftCustomerId::new("buyer".into()),
                name: "Synthetic Buyer".into(),
            })
            .with_bill_date(Some(date("2025-04-01"))),
        )
        .unwrap();
    let review = fixture
        .review_named(Role::Owner, id, Money::from_minor_units(0))
        .unwrap();
    fixture
        .confirm(Role::Owner, RequestId::new(17), &review)
        .unwrap();
    let before = fixture.clone();
    let result =
        fixture.monthly_profit_report(Role::Owner, &ManualCashBook::default(), date("2025-04-15"));
    assert_eq!(result, Err(ReportsError::UnknownFrozenCost(id)));
    let ranking = fixture.customer_ranking(Role::Owner).unwrap();
    assert_eq!(
        (
            ranking[0].name.as_str(),
            ranking[0].total.minor_units(),
            &ranking[0].bills
        ),
        ("Synthetic Buyer", 500, &vec![(id, Some("buyer".into()))])
    );
    assert_eq!(fixture, before);
}
