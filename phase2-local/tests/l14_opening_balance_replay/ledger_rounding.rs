use super::*;

#[test]
fn historical_rounds_prior_before_adding_fractional_bill_when_latest_stays_exact() {
    let money = Money::from_minor_units;
    let date: BillDate = FEB10.parse().unwrap();
    let customer = DraftCustomerId::new("rounding-control".into());
    let product = ProductId::new(1);
    let id = DraftId::new(1);
    let mut fixture = ConfirmationFixture::new(product, 1, money(125)).unwrap();
    fixture.register_customer(Role::Owner, &customer).unwrap();
    fixture
        .save_customer_opening(
            Role::Owner,
            &customer,
            OpeningBalance {
                amount: Some(money(155)),
                date: Some(date),
            },
        )
        .unwrap();
    let draft = ProvisionalDraft::new(id, vec![DraftLine::new(product, NonZeroU32::MIN)])
        .with_bill_date(Some(date))
        .with_customer(DraftCustomer::Selected {
            id: customer.clone(),
            name: "Control".into(),
        });
    fixture.register_draft(draft).unwrap();
    let review = fixture.review_named(Role::Owner, id, money(0)).unwrap();
    fixture
        .confirm(Role::Owner, RequestId::new(1), &review)
        .unwrap();

    let historical = fixture
        .bill_preview(Role::Owner, id, BalanceMode::AsOnBillDate)
        .unwrap();
    let latest = fixture
        .bill_preview(Role::Owner, id, BalanceMode::Latest)
        .unwrap();
    let ledger = fixture
        .customer_ledger_with_opening(Role::Owner, &customer)
        .unwrap();
    assert_eq!(historical.previous_balance.unwrap().amount, money(200));
    assert_eq!(historical.balance, money(325));
    assert_eq!(latest.balance, money(280));
    assert_eq!(ledger.closing, money(280));
    println!(
        "fractional control (paise): prior=155 rounded_prior=200 bill_due=125 historical={} latest={} ledger={}",
        historical.balance.minor_units(),
        latest.balance.minor_units(),
        ledger.closing.minor_units()
    );
}

#[test]
fn historical_due_overflow_does_not_affect_exact_latest() {
    let money = Money::from_minor_units;
    let date: BillDate = FEB10.parse().unwrap();
    let customer = DraftCustomerId::new("rounding-overflow".into());
    let product = ProductId::new(1);
    let id = DraftId::new(1);
    let mut fixture = ConfirmationFixture::new(product, 1, money(57)).unwrap();
    fixture.register_customer(Role::Owner, &customer).unwrap();
    fixture
        .save_customer_opening(
            Role::Owner,
            &customer,
            OpeningBalance {
                amount: Some(money(i64::MAX - 57)),
                date: Some(date),
            },
        )
        .unwrap();
    let draft = ProvisionalDraft::new(id, vec![DraftLine::new(product, NonZeroU32::MIN)])
        .with_bill_date(Some(date))
        .with_customer(DraftCustomer::Selected {
            id: customer,
            name: "Overflow".into(),
        });
    fixture.register_draft(draft).unwrap();
    let review = fixture.review_named(Role::Owner, id, money(0)).unwrap();
    fixture
        .confirm(Role::Owner, RequestId::new(1), &review)
        .unwrap();
    assert_eq!(
        fixture.bill_preview(Role::Owner, id, BalanceMode::AsOnBillDate),
        Err(ConfirmationError::Overflow)
    );
    assert_eq!(
        fixture
            .bill_preview(Role::Owner, id, BalanceMode::Latest)
            .unwrap()
            .balance,
        money(i64::MAX)
    );
    println!("historical due overflow: rejected; latest={}", i64::MAX);
}

#[test]
fn historical_rounding_uses_math_round_ties_and_handles_limits() {
    let money = Money::from_minor_units;
    let date: BillDate = FEB10.parse().unwrap();
    for (prior, expected) in [
        (-151, -200),
        (-150, -100),
        (-50, 0),
        (49, 0),
        (50, 100),
        (i64::MAX - 1, 9_223_372_036_854_775_800),
    ] {
        let customer = DraftCustomerId::new("rounding-edge".into());
        let product = ProductId::new(1);
        let id = DraftId::new(1);
        let mut fixture = ConfirmationFixture::new(product, 1, money(1)).unwrap();
        fixture.register_customer(Role::Owner, &customer).unwrap();
        fixture
            .save_customer_opening(
                Role::Owner,
                &customer,
                OpeningBalance {
                    amount: Some(money(prior)),
                    date: Some(date),
                },
            )
            .unwrap();
        let draft = ProvisionalDraft::new(id, vec![DraftLine::new(product, NonZeroU32::MIN)])
            .with_bill_date(Some(date))
            .with_customer(DraftCustomer::Selected {
                id: customer,
                name: "Edge".into(),
            });
        fixture.register_draft(draft).unwrap();
        let review = fixture.review_named(Role::Owner, id, money(0)).unwrap();
        fixture
            .confirm(Role::Owner, RequestId::new(1), &review)
            .unwrap();
        let historical = fixture
            .bill_preview(Role::Owner, id, BalanceMode::AsOnBillDate)
            .unwrap();
        assert_eq!(historical.balance, money(expected + 1));
        let latest = fixture
            .bill_preview(Role::Owner, id, BalanceMode::Latest)
            .unwrap();
        assert_eq!(latest.balance, money(prior + 1));
        println!(
            "rounding edge (paise): prior={prior} rounded={expected:?} latest={}",
            latest.balance.minor_units()
        );
    }
}
