use super::*;

#[test]
fn l14_opening_date_does_not_become_previous_transaction_date() {
    let (mut fixture, customer, b1, b2) = rust_fixture();
    fixture
        .save_customer_opening(
            Role::Owner,
            &customer,
            OpeningBalance {
                amount: Some(Money::from_minor_units(2_000)),
                date: Some("2025-02-11".parse().unwrap()),
            },
        )
        .unwrap();
    let first = fixture
        .bill_preview(Role::Owner, b1, BalanceMode::AsOnBillDate)
        .unwrap();
    let second = fixture
        .bill_preview(Role::Owner, b2, BalanceMode::AsOnBillDate)
        .unwrap();
    let first_previous = first.previous_balance.unwrap();
    let second_previous = second.previous_balance.unwrap();
    assert_eq!(
        (first_previous.amount.minor_units(), first_previous.date),
        (2_000, None)
    );
    assert_eq!(
        (second_previous.amount.minor_units(), second_previous.date),
        (7_000, Some(FEB10.parse().unwrap()))
    );
    assert_eq!(second.balance.minor_units(), 16_000);
}

#[test]
fn l14_opening_replace_accepts_representable_result_after_wide_intermediate() {
    let (mut fixture, customer, _, _) = rust_fixture();
    fixture
        .save_customer_opening(
            Role::Owner,
            &customer,
            OpeningBalance {
                amount: Some(Money::from_minor_units(-20_000)),
                date: None,
            },
        )
        .unwrap();
    fixture
        .record_customer_receipt(
            Role::Owner,
            CustomerReceiptInput {
                customer_id: customer.clone(),
                date: FEB10.parse().unwrap(),
                amount: Money::from_minor_units(-i64::MAX),
                mode: ReceiptMode::Cash,
            },
        )
        .unwrap();
    fixture
        .save_customer_opening(
            Role::Owner,
            &customer,
            OpeningBalance {
                amount: Some(Money::from_minor_units(-20_001)),
                date: None,
            },
        )
        .unwrap();
    assert_eq!(
        fixture.customer_due(customer.value()).minor_units(),
        i64::MAX - 7_401
    );
}
