use super::*;
use alm_phase2_local::confirmation::{ConfirmationError, OpeningBalance, OpeningLedgerKind};

#[path = "opening_edges.rs"]
mod opening_edges;

#[test]
fn l14_opening_save_replaces_signed_value_for_only_the_selected_id() {
    let (mut fixture, customer, _, b2) = rust_fixture();
    let other = DraftCustomerId::new("same-name-other".into());
    let other_bill = DraftId::new(30);
    let draft = ProvisionalDraft::new(
        other_bill,
        vec![DraftLine::new(ProductId::new(1), NonZeroU32::MIN)],
    )
    .with_bill_date(Some(FEB10.parse().unwrap()))
    .with_customer(DraftCustomer::Selected {
        id: other.clone(),
        name: "Synthetic customer".into(),
    });
    fixture.register_draft(draft).unwrap();
    let review = fixture
        .review_named(Role::Owner, other_bill, Money::from_minor_units(1000))
        .unwrap();
    fixture
        .confirm(Role::Owner, RequestId::new(103), &review)
        .unwrap();
    let other_before = fixture.customer_ledger(Role::Owner, &other).unwrap();
    let other_due = fixture.customer_due(other.value());
    let facts_before = (
        fixture.ledger(),
        fixture.received_ledger(),
        fixture.posted_count(),
    );
    let before = fixture.customer_ledger(Role::Owner, &customer).unwrap();
    let transaction_facts: Vec<_> = before
        .entries
        .iter()
        .map(|entry| (entry.date, entry.kind.clone(), entry.debit, entry.credit))
        .collect();
    let paid_before = fixture
        .bill_preview(Role::Owner, b2, BalanceMode::Latest)
        .unwrap()
        .paid;
    let date = Some(FEB10.parse().unwrap());
    for (input, expected) in [
        (Some(2_000), 14_600),
        (Some(-2_000), 10_600),
        (Some(0), 12_600),
        (None, 12_600),
    ] {
        fixture
            .save_customer_opening(
                Role::Owner,
                &customer,
                OpeningBalance {
                    amount: input.map(Money::from_minor_units),
                    date,
                },
            )
            .unwrap();
        let due = fixture.customer_due(customer.value()).minor_units();
        assert_eq!(due, expected);
        let opening_ledger = fixture
            .customer_ledger_with_opening(Role::Owner, &customer)
            .unwrap();
        let opening = opening_ledger
            .entries
            .iter()
            .find(|entry| entry.kind == OpeningLedgerKind::Opening)
            .unwrap();
        assert_eq!(opening.debit.minor_units(), input.unwrap_or(0));
        assert_eq!(opening.credit.minor_units(), 0);
        assert_eq!(opening_ledger.closing.minor_units(), due);
        let actual_facts: Vec<_> = fixture
            .customer_ledger(Role::Owner, &customer)
            .unwrap()
            .entries
            .into_iter()
            .map(|entry| (entry.date, entry.kind, entry.debit, entry.credit))
            .collect();
        assert_eq!(actual_facts, transaction_facts);
        assert_eq!(
            fixture.customer_ledger(Role::Owner, &other).unwrap(),
            other_before
        );
        assert_eq!(fixture.customer_due(other.value()), other_due);
        assert_eq!(
            (
                fixture.ledger(),
                fixture.received_ledger(),
                fixture.posted_count()
            ),
            facts_before
        );
        assert_eq!(
            fixture
                .bill_preview(Role::Owner, b2, BalanceMode::Latest)
                .unwrap()
                .paid,
            paid_before
        );
        println!(
            "save customer={} other={} opening={input:?} due_paise={due}",
            customer.value(),
            other.value()
        );
    }
    assert_eq!(
        fixture.customer_ledger(Role::Owner, &customer).unwrap(),
        before
    );
    let unknown = DraftCustomerId::new("missing".into());
    let denied = fixture.save_customer_opening(
        Role::Manager,
        &unknown,
        OpeningBalance {
            amount: Some(Money::from_minor_units(7)),
            date,
        },
    );
    assert_eq!(denied, Err(ConfirmationError::AccessDenied));
    assert_eq!(
        fixture.customer_ledger(Role::Owner, &customer).unwrap(),
        before
    );
    println!(
        "manager denied for unknown id={}: {denied:?}",
        unknown.value()
    );
}

#[test]
fn l14_opening_rejected_on_overflow_or_manager_for_known_customer() {
    let (mut fixture, customer, _, b2) = rust_fixture();
    fixture
        .save_customer_opening(
            Role::Owner,
            &customer,
            OpeningBalance {
                amount: Some(Money::from_minor_units(2_000)),
                date: None,
            },
        )
        .unwrap();
    let before = fixture
        .customer_ledger_with_opening(Role::Owner, &customer)
        .unwrap();
    let preview = fixture
        .bill_preview(Role::Owner, b2, BalanceMode::Latest)
        .unwrap();
    let overflow = OpeningBalance {
        amount: Some(Money::from_minor_units(i64::MAX)),
        date: None,
    };
    assert_eq!(
        fixture.save_customer_opening(Role::Owner, &customer, overflow),
        Err(ConfirmationError::Overflow)
    );
    let denied = fixture.save_customer_opening(
        Role::Manager,
        &customer,
        OpeningBalance {
            amount: Some(Money::from_minor_units(3)),
            date: None,
        },
    );
    assert_eq!(denied, Err(ConfirmationError::AccessDenied));
    println!(
        "manager denied for known id={}: {denied:?}",
        customer.value()
    );
    assert_eq!(
        fixture
            .customer_ledger_with_opening(Role::Owner, &customer)
            .unwrap(),
        before
    );
    assert_eq!(
        fixture
            .bill_preview(Role::Owner, b2, BalanceMode::Latest)
            .unwrap(),
        preview
    );
}

#[test]
fn l14_actual_opening_replay_preserves_document_and_posting_orders() {
    for opening_date in [Some(FEB10.parse().unwrap()), None] {
        let (mut fixture, customer, b1, b2) = rust_fixture();
        fixture
            .save_customer_opening(
                Role::Owner,
                &customer,
                OpeningBalance {
                    amount: Some(Money::from_minor_units(2_000)),
                    date: opening_date,
                },
            )
            .unwrap();
        let ledger = fixture
            .customer_ledger_with_opening(Role::Owner, &customer)
            .unwrap();
        assert_eq!(ledger.closing.minor_units(), 14_600);
        assert_eq!(fixture.customer_due(customer.value()).minor_units(), 14_600);
        let rows: Vec<_> = ledger
            .entries
            .iter()
            .map(|entry| (entry.date, entry.kind.clone(), entry.balance.minor_units()))
            .collect();
        let cash = OpeningLedgerKind::Transaction(CustomerLedgerKind::Receipt {
            mode: ReceiptMode::Cash,
        });
        let online = OpeningLedgerKind::Transaction(CustomerLedgerKind::Receipt {
            mode: ReceiptMode::Online,
        });
        let bill = |draft_id, invoice| {
            OpeningLedgerKind::Transaction(CustomerLedgerKind::Bill { draft_id, invoice })
        };
        let feb9 = Some(FEB9.parse().unwrap());
        let feb10 = Some(FEB10.parse().unwrap());
        let prefix = if opening_date.is_some() {
            vec![
                (feb9, cash.clone(), -1_000),
                (feb10, OpeningLedgerKind::Opening, 1_000),
            ]
        } else {
            vec![
                (None, OpeningLedgerKind::Opening, 2_000),
                (feb9, cash.clone(), 1_000),
            ]
        };
        assert_eq!(
            rows,
            [
                prefix,
                vec![
                    (feb10, bill(b1, 1), 9_000),
                    (feb10, cash, 6_000),
                    (feb10, bill(b2, 2), 15_000),
                    (feb10, online, 14_600)
                ]
            ]
            .concat()
        );
        let as_on = fixture
            .bill_preview(Role::Owner, b2, BalanceMode::AsOnBillDate)
            .unwrap()
            .balance
            .minor_units();
        let latest = fixture
            .bill_preview(Role::Owner, b2, BalanceMode::Latest)
            .unwrap()
            .balance
            .minor_units();
        assert_eq!((as_on, rows[4].2, latest), (16_000, 15_000, 14_600));
        println!(
            "customer={} opening_date={opening_date:?} actual rows={rows:?} B2 as_on={as_on} latest={latest}",
            customer.value()
        );
    }
}
