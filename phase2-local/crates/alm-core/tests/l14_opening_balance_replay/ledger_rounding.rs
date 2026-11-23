use super::*;

#[test]
fn historical_and_latest_round_at_different_points_while_ledger_stays_exact() {
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
    assert_eq!(latest.balance, money(300));
    assert_eq!(ledger.closing, money(280));
    println!(
        "fractional control (paise): prior=155 rounded_prior=200 bill_due=125 historical={} latest={} ledger={}",
        historical.balance.minor_units(),
        latest.balance.minor_units(),
        ledger.closing.minor_units()
    );
}

#[test]
fn historical_due_can_overflow_when_latest_aggregate_rounds_down() {
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
        money(9_223_372_036_854_775_800)
    );
}

#[test]
fn historical_rounding_uses_math_round_ties_and_handles_limits() {
    let money = Money::from_minor_units;
    let date: BillDate = FEB10.parse().unwrap();
    for (prior, expected, expected_latest) in [
        (-151, -200, -100),
        (-150, -100, -100),
        (-50, 0, 0),
        (49, 0, 100),
        (50, 100, 100),
        (
            i64::MAX - 1,
            9_223_372_036_854_775_800,
            9_223_372_036_854_775_800,
        ),
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
        assert_eq!(
            fixture
                .bill_preview(Role::Owner, id, BalanceMode::Latest)
                .unwrap()
                .balance,
            money(expected_latest)
        );
    }
}

#[test]
fn latest_ties_round_aggregate_in_preview_and_inert_estimate_without_rounding_receipts() {
    let money = Money::from_minor_units;
    let date: BillDate = FEB10.parse().unwrap();
    let product = ProductId::new(1);
    let id = DraftId::new(1);
    let display = [EstimateLineDisplay {
        product,
        label: "Synthetic item",
    }];
    for (opening, exact, rounded, displayed) in
        [(75, 150, 200, "₹2.00"), (-225, -150, -100, "₹-1.00")]
    {
        let customer = DraftCustomerId::new("latest-tie".into());
        let mut fixture = ConfirmationFixture::new(product, 1, money(125)).unwrap();
        fixture.register_customer(Role::Owner, &customer).unwrap();
        fixture
            .save_customer_opening(
                Role::Owner,
                &customer,
                OpeningBalance {
                    amount: Some(money(opening)),
                    date: Some(date),
                },
            )
            .unwrap();
        let draft = ProvisionalDraft::new(id, vec![DraftLine::new(product, NonZeroU32::MIN)])
            .with_bill_date(Some(date))
            .with_customer(DraftCustomer::Selected {
                id: customer.clone(),
                name: "Tie".into(),
            });
        fixture.register_draft(draft).unwrap();
        let review = fixture.review_named(Role::Owner, id, money(0)).unwrap();
        fixture
            .confirm(Role::Owner, RequestId::new(1), &review)
            .unwrap();
        fixture
            .record_customer_receipt(
                Role::Owner,
                CustomerReceiptInput {
                    customer_id: customer.clone(),
                    date,
                    amount: money(50),
                    mode: ReceiptMode::Cash,
                },
            )
            .unwrap();

        let ledger = fixture
            .customer_ledger_with_opening(Role::Owner, &customer)
            .unwrap();
        let preview = fixture
            .bill_preview(Role::Owner, id, BalanceMode::Latest)
            .unwrap();
        assert_eq!(ledger.closing, money(exact));
        assert!(ledger.entries.iter().any(|entry| entry.credit == money(50)));
        assert_eq!(preview.balance, money(rounded));
        let request = EstimateRequest {
            role: Role::Owner,
            draft_id: id,
            balance_mode: BalanceMode::Latest,
            display: EstimateDisplay {
                today: date,
                lines: &display,
            },
        };
        let mut output = Vec::new();
        fixture
            .print_estimate(request, &mut |doc| output.push(doc))
            .unwrap();
        fixture
            .save_estimate_as_pdf(request, &mut |doc| output.push(doc))
            .unwrap();
        assert_eq!(output.len(), 2);
        for document in output {
            assert!(
                document.html.contains(&format!(
                    "<th>Total Due (as on 10-Feb-2025)</th><td>{displayed}</td>"
                )),
                "{}",
                document.html
            );
        }
    }
}
