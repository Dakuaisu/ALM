use super::*;
use crate::DraftCustomerId;

mod cashbook_scope;

#[test]
fn customer_accounts_receipt_survives_bill_cancellation() {
    let mut f = fixture(3);
    let customer = DraftCustomerId::new("account-a".into());
    let bill_date = BillDate::new(2025, 2, 21).unwrap();
    let receipt_date = BillDate::new(2025, 3, 1).unwrap();
    f.register_draft(draft(1, 1).with_bill_date(Some(bill_date)).with_customer(
        DraftCustomer::Selected {
            id: customer.clone(),
            name: "Shop".into(),
        },
    ))
    .unwrap();
    let review = f
        .review_named(Role::Owner, DraftId::new(1), Money::from_minor_units(100))
        .unwrap();
    f.confirm(Role::Owner, RequestId::new(1), &review).unwrap();
    f.record_customer_receipt(
        Role::Owner,
        CustomerReceiptInput {
            customer_id: customer.clone(),
            date: receipt_date,
            amount: Money::from_minor_units(50),
            mode: ReceiptMode::Online,
        },
    )
    .unwrap();
    f.cancel_sale(Role::Owner, DraftId::new(1)).unwrap();
    assert_eq!(
        f.customer_ledger(Role::Owner, &customer)
            .unwrap()
            .closing
            .minor_units(),
        -50
    );
    assert_eq!(
        f.cashbook_day_close(Role::Owner, bill_date)
            .unwrap()
            .cash
            .minor_units(),
        0
    );
    assert_eq!(
        f.cashbook_day_close(Role::Owner, receipt_date)
            .unwrap()
            .online
            .minor_units(),
        50
    );
}

fn account_fixture() -> (ConfirmationFixture, DraftCustomerId, BillDate) {
    let mut f = fixture(8);
    let customer = DraftCustomerId::new("a".into());
    let date = BillDate::new(2025, 2, 21).unwrap();
    f.register_draft(draft(1, 1).with_bill_date(Some(date)).with_customer(
        DraftCustomer::Selected {
            id: customer.clone(),
            name: "Shop".into(),
        },
    ))
    .unwrap();
    let review = f
        .review_named(Role::Owner, DraftId::new(1), Money::from_minor_units(25))
        .unwrap();
    f.confirm(Role::Owner, RequestId::new(1), &review).unwrap();
    (f, customer, date)
}

#[test]
fn customer_accounts_receipt_updates_preview_and_account_without_touching_bill() {
    let (mut f, customer, date) = account_fixture();
    let before = f.received_ledger();
    f.record_customer_receipt(
        Role::Owner,
        CustomerReceiptInput {
            customer_id: customer.clone(),
            date,
            amount: Money::from_minor_units(150),
            mode: ReceiptMode::Online,
        },
    )
    .unwrap();
    assert_eq!(f.customer_due(customer.value()).minor_units(), -50);
    assert_eq!(f.received_ledger(), before);
    assert_eq!(
        f.customer_ledger(Role::Owner, &customer)
            .unwrap()
            .closing
            .minor_units(),
        -50
    );
    assert_eq!(
        f.bill_preview(Role::Owner, DraftId::new(1), BalanceMode::AsOnBillDate)
            .unwrap()
            .balance
            .minor_units(),
        100
    );
    assert_eq!(
        f.bill_preview(Role::Owner, DraftId::new(1), BalanceMode::Latest)
            .unwrap()
            .balance
            .minor_units(),
        0
    );
}

#[test]
fn customer_accounts_denial_invalid_and_overflow_are_atomic() {
    let (mut f, customer, date) = account_fixture();
    let input = |customer_id, amount| CustomerReceiptInput {
        customer_id,
        date,
        amount: Money::from_minor_units(amount),
        mode: ReceiptMode::Cash,
    };
    let before = f.clone();
    assert_eq!(
        f.record_customer_receipt(Role::Manager, input(customer.clone(), 5)),
        Err(ConfirmationError::AccessDenied)
    );
    assert_eq!(
        f.record_customer_receipt(Role::Owner, input(customer.clone(), 0)),
        Err(ConfirmationError::InvalidAmount)
    );
    assert_eq!(
        f.record_customer_receipt(
            Role::Owner,
            input(DraftCustomerId::new("missing".into()), 1)
        ),
        Err(ConfirmationError::InvalidDraft)
    );
    assert_eq!(
        f.record_customer_receipt(Role::Owner, input(customer.clone(), i64::MIN)),
        Err(ConfirmationError::Overflow)
    );
    assert_eq!(
        f.customer_ledger(Role::Manager, &customer),
        Err(ConfirmationError::AccessDenied)
    );
    assert_eq!(
        f.cashbook_day_close(Role::Manager, date),
        Err(ConfirmationError::AccessDenied)
    );
    assert_eq!(f, before);
}

#[test]
fn customer_accounts_cashbook_rounds_signed_bucket_after_summing() {
    let (mut f, customer, date) = account_fixture();
    for (mode, amount) in [
        (ReceiptMode::Cash, 25),
        (ReceiptMode::Online, -25),
        (ReceiptMode::Online, -25),
    ] {
        f.record_customer_receipt(
            Role::Owner,
            CustomerReceiptInput {
                customer_id: customer.clone(),
                date,
                amount: Money::from_minor_units(amount),
                mode,
            },
        )
        .unwrap();
    }
    let day = f.cashbook_day_close(Role::Owner, date).unwrap();
    assert_eq!(
        (day.cash.minor_units(), day.online.minor_units()),
        (50, -50)
    );
    assert_eq!((day.cash_rupees, day.online_rupees, day.count), (1, 0, 4));
}

#[test]
fn customer_accounts_ledger_orders_same_date_by_creation_and_separates_ids() {
    let (mut f, customer, date) = account_fixture();
    f.record_customer_receipt(
        Role::Owner,
        CustomerReceiptInput {
            customer_id: customer.clone(),
            date,
            amount: Money::from_minor_units(50),
            mode: ReceiptMode::Cash,
        },
    )
    .unwrap();
    for (number, id) in [(2, customer.clone()), (3, DraftCustomerId::new("b".into()))] {
        f.register_draft(draft(number, 1).with_bill_date(Some(date)).with_customer(
            DraftCustomer::Selected {
                id,
                name: "Shop".into(),
            },
        ))
        .unwrap();
        let review = f
            .review_named(
                Role::Owner,
                DraftId::new(number),
                Money::from_minor_units(25),
            )
            .unwrap();
        f.confirm(Role::Owner, RequestId::new(number), &review)
            .unwrap();
    }
    let ledger = f.customer_ledger(Role::Owner, &customer).unwrap();
    assert_eq!(
        ledger
            .entries
            .iter()
            .map(|row| row.balance.minor_units())
            .collect::<Vec<_>>(),
        vec![100, 50, 150]
    );
    assert_eq!(ledger.closing, f.customer_due(customer.value()));
}

#[test]
fn customer_accounts_ledger_sorts_backdated_receipts_by_document_date() {
    let (mut f, customer, date) = account_fixture();
    let earlier = BillDate::new(2025, 1, 10).unwrap();
    f.record_customer_receipt(
        Role::Owner,
        CustomerReceiptInput {
            customer_id: customer.clone(),
            date: earlier,
            amount: Money::from_minor_units(-20),
            mode: ReceiptMode::Online,
        },
    )
    .unwrap();
    let ledger = f.customer_ledger(Role::Owner, &customer).unwrap();
    assert_eq!(
        ledger
            .entries
            .iter()
            .map(|row| (row.date, row.balance.minor_units()))
            .collect::<Vec<_>>(),
        vec![(earlier, 20), (date, 120)]
    );
    assert_eq!(ledger.closing, f.customer_due(customer.value()));
    assert_eq!(
        f.cashbook_day_close(Role::Owner, earlier)
            .unwrap()
            .online
            .minor_units(),
        -20
    );
}

#[test]
fn customer_accounts_cashbook_rejects_unknown_receipt_mode() {
    let (mut f, customer, date) = account_fixture();
    f.preview_payments.push(PreviewPayment {
        customer_id: customer.value().into(),
        date,
        amount: Money::from_minor_units(25),
        after_invoice: 1,
        mode: None,
        metadata: None,
    });
    assert_eq!(
        f.cashbook_day_close(Role::Owner, date),
        Err(ConfirmationError::InvalidDraft)
    );
}
