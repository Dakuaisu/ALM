use super::*;

#[test]
fn keyed_ledger_and_estimate_replay_conflicting_orders() {
    let money = Money::from_minor_units;
    let day9: BillDate = FEB9.parse().unwrap();
    let day10: BillDate = FEB10.parse().unwrap();
    let day11: BillDate = "2025-02-11".parse().unwrap();
    let customer = DraftCustomerId::new("keyed-a".into());
    let other = DraftCustomerId::new("keyed-b".into());
    let product = ProductId::new(1);
    let mut fixture = ConfirmationFixture::new(product, 8, money(10_000)).unwrap();
    for id in [&customer, &other] {
        fixture.register_customer(Role::Owner, id).unwrap();
    }
    fixture
        .save_customer_opening(
            Role::Owner,
            &customer,
            OpeningBalance {
                amount: Some(money(2_000)),
                date: Some(day10),
            },
        )
        .unwrap();
    let post = |fixture: &mut ConfirmationFixture, number, date, paid, key| {
        let id = DraftId::new(number);
        let draft = ProvisionalDraft::new(id, vec![DraftLine::new(product, NonZeroU32::MIN)])
            .with_bill_date(Some(date))
            .with_customer(DraftCustomer::Selected {
                id: customer.clone(),
                name: "Shared".into(),
            });
        fixture.register_draft(draft).unwrap();
        let review = fixture.review_named(Role::Owner, id, money(paid)).unwrap();
        fixture
            .confirm_with_creation_key(Role::Owner, RequestId::new(number), &review, key)
            .unwrap();
    };
    let target = DraftId::new(10);
    let early = DraftId::new(20);
    let unkeyed = DraftId::new(30);
    post(&mut fixture, 10, day10, 1_000, Some(CreationKey::new(30)));
    post(&mut fixture, 20, day11, 2_000, Some(CreationKey::new(10)));
    post(&mut fixture, 30, day10, 0, None);
    let customers = [
        Customer {
            id: customer.clone(),
            name: "Shared".into(),
        },
        Customer {
            id: other.clone(),
            name: "Shared".into(),
        },
    ];
    for (which, date, amount, mode, key) in [
        (0, day10, 333, ReceiptMode::Online, 40),
        (0, day10, 222, ReceiptMode::Cash, 30),
        (0, day9, 1_001, ReceiptMode::Online, 20),
        (1, day9, 777, ReceiptMode::Cash, 1),
    ] {
        let mut receive = ReceivePayment::with_clock(move || Ok(CreationKey::new(key)));
        receive.input("Shared", &customers);
        receive.pick(which);
        receive
            .save(Role::Owner, &mut fixture, date, money(amount), mode)
            .unwrap();
    }
    fixture
        .record_customer_receipt(
            Role::Owner,
            CustomerReceiptInput {
                customer_id: customer.clone(),
                date: day10,
                amount: money(444),
                mode: ReceiptMode::Cash,
            },
        )
        .unwrap();
    let ledger = fixture
        .customer_ledger_with_opening(Role::Owner, &customer)
        .unwrap();
    let bill = |draft_id, invoice| {
        OpeningLedgerKind::Transaction(CustomerLedgerKind::Bill { draft_id, invoice })
    };
    let payment = |mode| OpeningLedgerKind::Transaction(CustomerLedgerKind::Receipt { mode });
    let rows: Vec<_> = ledger
        .entries
        .iter()
        .map(|entry| {
            (
                entry.date,
                entry.kind.clone(),
                entry.debit.minor_units(),
                entry.credit.minor_units(),
                entry.balance.minor_units(),
            )
        })
        .collect();
    assert_eq!(
        rows,
        vec![
            (Some(day9), payment(ReceiptMode::Online), 0, 1_001, -1_001),
            (Some(day10), OpeningLedgerKind::Opening, 2_000, 0, 999),
            (Some(day10), bill(unkeyed, 3), 10_000, 0, 10_999),
            (Some(day10), payment(ReceiptMode::Cash), 0, 444, 10_555),
            (Some(day10), bill(target, 1), 10_000, 1_000, 19_555),
            (Some(day10), payment(ReceiptMode::Cash), 0, 222, 19_333),
            (Some(day10), payment(ReceiptMode::Online), 0, 333, 19_000),
            (Some(day11), bill(early, 2), 10_000, 2_000, 27_000),
        ]
    );
    assert_eq!(ledger.closing, money(27_000));
    assert_eq!(
        fixture
            .customer_ledger(Role::Owner, &other)
            .unwrap()
            .closing,
        money(-777)
    );
    let as_on = fixture
        .bill_preview(Role::Owner, target, BalanceMode::AsOnBillDate)
        .unwrap();
    let latest = fixture
        .bill_preview(Role::Owner, target, BalanceMode::Latest)
        .unwrap();
    assert_eq!(as_on.previous_balance.unwrap().amount, money(18_600));
    assert_eq!(as_on.previous_balance.unwrap().date, Some(day11));
    assert_eq!(as_on.balance, money(27_600));
    assert_eq!(latest.balance, money(27_000));
    assert_eq!(latest.previous_balance, None);
    let display_lines = [EstimateLineDisplay {
        product,
        label: "Synthetic",
    }];
    let request = EstimateRequest {
        role: Role::Owner,
        draft_id: target,
        balance_mode: BalanceMode::AsOnBillDate,
        display: EstimateDisplay {
            today: day11,
            lines: &display_lines,
        },
    };
    let mut documents = Vec::new();
    fixture
        .print_estimate(request, &mut |doc| documents.push(doc))
        .unwrap();
    assert!(
        documents[0]
            .html
            .contains("Prev Balance (as on 11-Feb-2025)</th><td>₹186.00")
    );
    assert!(documents[0].html.contains("Balance Due</th><td>₹276.00"));
    println!("keyed ledger rows (date, kind, debit, credit, running paise): {rows:?}");
    println!(
        "keyed preview previous={:?} as_on={:?} latest={:?}",
        as_on.previous_balance, as_on.balance, latest.balance
    );
    assert_eq!(
        fixture.customer_ledger_with_opening(Role::Manager, &customer),
        Err(ConfirmationError::AccessDenied)
    );
    // DEC-034: a Manager reads the posted bill; the ledger above stays the Owner's.
    assert_eq!(
        fixture.bill_preview(Role::Manager, target, BalanceMode::Latest),
        fixture.bill_preview(Role::Owner, target, BalanceMode::Latest)
    );
    let mut receive = ReceivePayment::with_clock(|| Ok(CreationKey::new(5)));
    receive.input("Shared", &customers);
    receive.pick(0);
    assert_eq!(
        fixture
            .customer_ledger_with_opening(Role::Owner, &customer)
            .unwrap(),
        ledger
    );
    assert_eq!(
        fixture
            .bill_preview(Role::Owner, target, BalanceMode::AsOnBillDate)
            .unwrap(),
        as_on
    );
    // DEC-034: a Manager records the payment too.
    assert_eq!(
        receive.save(
            Role::Manager,
            &mut fixture,
            day10,
            money(1),
            ReceiptMode::Cash
        ),
        Ok(())
    );
    assert_ne!(
        fixture
            .customer_ledger_with_opening(Role::Owner, &customer)
            .unwrap(),
        ledger
    );
}
