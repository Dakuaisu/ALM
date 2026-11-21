use super::*;

#[test]
fn backdated_later_receipt_uses_document_date_for_cashbook_not_preview_creation_order() {
    // Given a posted named bill and a receipt entered afterwards with an earlier date.
    let (mut fixture, customer, bill_date) = account_fixture();
    let receipt_date = BillDate::new(2025, 1, 10).unwrap();

    // When the Owner records an explicitly Online standalone receipt.
    fixture
        .record_customer_receipt(
            Role::Owner,
            CustomerReceiptInput {
                customer_id: customer.clone(),
                date: receipt_date,
                amount: Money::from_minor_units(50),
                mode: ReceiptMode::Online,
            },
        )
        .unwrap();

    // Then Cash Book and ledger use document dates, but As On retains creation order.
    let bill_day = fixture.cashbook_day_close(Role::Owner, bill_date).unwrap();
    let receipt_day = fixture
        .cashbook_day_close(Role::Owner, receipt_date)
        .unwrap();
    assert_eq!(
        (bill_day.cash.minor_units(), bill_day.online.minor_units()),
        (25, 0)
    );
    assert_eq!(
        (
            receipt_day.cash.minor_units(),
            receipt_day.online.minor_units()
        ),
        (0, 50)
    );
    let ledger = fixture.customer_ledger(Role::Owner, &customer).unwrap();
    assert_eq!(
        ledger
            .entries
            .iter()
            .map(|entry| entry.date)
            .collect::<Vec<_>>(),
        vec![receipt_date, bill_date]
    );
    assert_eq!(
        fixture
            .bill_preview(Role::Owner, DraftId::new(1), BalanceMode::AsOnBillDate)
            .unwrap()
            .balance
            .minor_units(),
        100
    );
    assert_eq!(
        fixture
            .bill_preview(Role::Owner, DraftId::new(1), BalanceMode::Latest)
            .unwrap()
            .balance
            .minor_units(),
        50
    );
}
