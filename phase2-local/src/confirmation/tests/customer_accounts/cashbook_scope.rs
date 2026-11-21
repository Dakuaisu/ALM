use super::*;

#[test]
fn named_bill_modes_replay_and_cancel_with_separately_dated_receipt() {
    for mode in [ReceiptMode::Cash, ReceiptMode::Online] {
        let mut f = fixture(3);
        let customer = DraftCustomerId::new("paired".into());
        let bill_date = BillDate::new(2025, 2, 21).unwrap();
        let receipt_date = BillDate::new(2025, 3, 1).unwrap();
        let id = DraftId::new(1);
        let paid = Money::from_minor_units(75);
        f.register_draft(draft(1, 1).with_bill_date(Some(bill_date)).with_customer(
            DraftCustomer::Selected {
                id: customer.clone(),
                name: "Shop".into(),
            },
        ))
        .unwrap();
        assert_eq!(
            f.review_named_with_mode(Role::Manager, id, paid, mode),
            Err(ConfirmationError::AccessDenied)
        );
        assert_eq!(
            f.review_named(Role::Owner, id, paid)
                .unwrap()
                .payment_mode(),
            ReceiptMode::Cash
        );
        let review = f
            .review_named_with_mode(Role::Owner, id, paid, mode)
            .unwrap();
        assert_eq!(review.payment_mode(), mode);
        let request = RequestId::new(1);
        assert_eq!(
            f.confirm(Role::Manager, request, &review),
            Err(ConfirmationError::AccessDenied)
        );
        let posted = f.confirm(Role::Owner, request, &review).unwrap();
        assert_eq!(
            f.current_sale(Role::Owner, id).unwrap().0.payment_mode(),
            mode
        );
        assert_eq!(
            f.current_sale(Role::Manager, id),
            Err(ConfirmationError::AccessDenied)
        );
        assert_eq!(f.confirm(Role::Owner, request, &review).unwrap(), posted);
        let other = f
            .review_named_with_mode(
                Role::Owner,
                id,
                paid,
                match mode {
                    ReceiptMode::Cash => ReceiptMode::Online,
                    ReceiptMode::Online => ReceiptMode::Cash,
                },
            )
            .unwrap();
        assert_eq!(
            f.confirm(Role::Owner, request, &other),
            Err(ConfirmationError::RequestConflict)
        );
        let input = CustomerReceiptInput {
            customer_id: customer.clone(),
            date: receipt_date,
            amount: Money::from_minor_units(50),
            mode: ReceiptMode::Online,
        };
        assert_eq!(
            f.record_customer_receipt(Role::Manager, input.clone()),
            Err(ConfirmationError::AccessDenied)
        );
        f.record_customer_receipt(Role::Owner, input).unwrap();
        let day = f.cashbook_day_close(Role::Owner, bill_date).unwrap();
        let expected = match mode {
            ReceiptMode::Cash => (75, 0, 1, 0),
            ReceiptMode::Online => (0, 75, 0, 1),
        };
        let observed = (
            day.cash.minor_units(),
            day.online.minor_units(),
            day.cash_rupees,
            day.online_rupees,
        );
        println!("{mode:?} bill expected {expected:?}, observed {observed:?}");
        assert_eq!(observed, expected);
        assert_eq!(
            day.entries,
            vec![CashBookEntry {
                kind: CashBookKind::Bill {
                    draft_id: id,
                    invoice: posted.invoice
                },
                mode,
                amount: paid
            }]
        );
        assert_eq!(
            f.cashbook_day_close(Role::Manager, bill_date),
            Err(ConfirmationError::AccessDenied)
        );
        assert_eq!(
            f.customer_ledger(Role::Manager, &customer),
            Err(ConfirmationError::AccessDenied)
        );
        assert_eq!(
            f.cancel_sale(Role::Manager, id),
            Err(ConfirmationError::AccessDenied)
        );
        let receipt_before = f.cashbook_day_close(Role::Owner, receipt_date).unwrap();
        let ledger_before = f.customer_ledger(Role::Owner, &customer).unwrap();
        assert_eq!(ledger_before.entries.len(), 2);
        assert_eq!(ledger_before.entries[0].date, bill_date);
        assert_eq!(
            ledger_before.entries[0].kind,
            CustomerLedgerKind::Bill {
                draft_id: id,
                invoice: posted.invoice
            }
        );
        let bill_credit = ledger_before.entries[0].credit.minor_units();
        let closing = ledger_before.closing.minor_units();
        println!(
            "{mode:?} before cancel bill credit expected 75, observed {bill_credit}; ledger closing expected 0, observed {closing}; receipt day {receipt_before:?}"
        );
        assert_eq!((bill_credit, closing), (75, 0));
        f.cancel_sale(Role::Owner, id).unwrap();
        let cancelled = f.cashbook_day_close(Role::Owner, bill_date).unwrap();
        let receipt = f.cashbook_day_close(Role::Owner, receipt_date).unwrap();
        let ledger = f.customer_ledger(Role::Owner, &customer).unwrap();
        assert_eq!(receipt_before, receipt);
        println!(
            "{mode:?} cancelled bill expected (0, 0), observed ({}, {}); receipt expected 50, observed {}; ledger expected -50, observed {}",
            cancelled.cash.minor_units(),
            cancelled.online.minor_units(),
            receipt.online.minor_units(),
            ledger.closing.minor_units()
        );
        assert!(cancelled.entries.is_empty());
        assert_eq!(
            (cancelled.cash.minor_units(), cancelled.online.minor_units()),
            (0, 0)
        );
        assert_eq!(
            receipt.entries,
            vec![CashBookEntry {
                kind: CashBookKind::Receipt,
                mode: ReceiptMode::Online,
                amount: Money::from_minor_units(50)
            }]
        );
        assert_eq!(
            (receipt.cash.minor_units(), receipt.online.minor_units()),
            (0, 50)
        );
        assert_eq!(ledger.entries.len(), 1);
        assert_eq!(ledger.entries[0].date, receipt_date);
        assert_eq!(
            ledger.entries[0].kind,
            CustomerLedgerKind::Receipt {
                mode: ReceiptMode::Online
            }
        );
        assert_eq!(ledger.closing.minor_units(), -50);
    }
}

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
