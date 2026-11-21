//! Synthetic source-derived Reports replay, not a production report or legacy execution.
use super::l04_sales_history_replay::seed::history_fixture;
use super::l15_cashbook_numeric_replay::saved_l15_row;
use super::*;
use crate::DraftCustomerId;
#[path = "l16_reports_projection.rs"]
mod projection;
use projection::{report, round};

fn date(iso: &str) -> BillDate {
    iso.parse().unwrap()
}

#[test]
fn l16_01_monthly_reports_synthetic_replay() {
    let mut fixture = history_fixture();
    let product = ProductId::new(16);
    fixture
        .add_product(product, 0, Money::from_minor_units(15_025))
        .unwrap();
    fixture
        .catalog
        .seed_lots(product, &[(2, Money::from_minor_units(5_000))])
        .unwrap();
    for (id, day, customer_id, name) in [
        (80, "2025-04-25", "same-name", "Synthetic Grove"),
        (81, "2024-10-20", "ancestor", "Synthetic Ancestor"),
    ] {
        let draft_id = DraftId::new(id);
        fixture
            .register_draft(
                ProvisionalDraft::new(
                    draft_id,
                    vec![DraftLine::new(product, NonZeroU32::new(1).unwrap())],
                )
                .with_customer(DraftCustomer::Selected {
                    id: DraftCustomerId::new(customer_id.into()),
                    name: name.into(),
                })
                .with_bill_date(Some(date(day))),
            )
            .unwrap();
        let review = fixture
            .review_named(Role::Owner, draft_id, Money::from_minor_units(0))
            .unwrap();
        fixture
            .confirm(Role::Owner, RequestId::new(id), &review)
            .unwrap();
    }
    fixture
        .record_customer_receipt(
            Role::Owner,
            CustomerReceiptInput {
                customer_id: DraftCustomerId::new("same-name".into()),
                date: date("2025-01-10"),
                amount: Money::from_minor_units(100),
                mode: ReceiptMode::Cash,
            },
        )
        .unwrap();

    let (mut book, original) = saved_l15_row();
    // Rebase the original L15 saved row; preserve its counted cash and online amounts.
    let mut feb = original.draft();
    feb.date = Some(date("2025-02-28"));
    feb.amounts.goods = Money::from_minor_units(1_201);
    feb.amounts.misc = Money::from_minor_units(2_302);
    feb.amounts.salary = Money::from_minor_units(3_403);
    feb.amounts.debt = Money::from_minor_units(4_504);
    let feb = book.save(Role::Owner, &feb).unwrap();
    let mut april = crate::confirmation::manual_cashbook::ManualCashBookDraft::new(
        Some(date("2025-04-10")),
        feb.amounts,
    );
    april.amounts.goods = Money::from_minor_units(999);
    april.amounts.misc = Money::from_minor_units(25);
    april.amounts.salary = Money::from_minor_units(25);
    let april = book.save(Role::Owner, &april).unwrap();
    let rows = [feb, april].map(|row| book.row(Role::Owner, row.id).unwrap().unwrap());
    assert_eq!(
        (
            rows[0].amounts.cash,
            rows[0].amounts.paytm,
            rows[0].amounts.phonepe
        ),
        (
            original.amounts.cash,
            original.amounts.paytm,
            original.amounts.phonepe
        )
    );
    assert_eq!(
        format!("{:?}", rows.map(|row| row.id)),
        "[ManualCashBookId(0), ManualCashBookId(1)]"
    );
    let before = fixture.clone();
    let before_rows = rows.map(|row| book.row(Role::Owner, row.id).unwrap());
    let result = report(&fixture, (&rows, date("2025-04-15")), Role::Owner).unwrap();
    #[rustfmt::skip]
    let expected = [
        ((2024, 11), vec![], vec![], vec![], [0, 0, 0, 0, 0, 0]),
        ((2024, 12), vec![], vec![], vec![], [0, 0, 0, 0, 0, 0]),
        ((2025, 1), vec![], vec![], vec![8], [0, 0, 0, 0, 0, 0]),
        ((2025, 2), vec![12], vec![feb.id], vec![3], [1049, 600, 449, 1201, 5705, -5256]),
        ((2025, 3), vec![36, 41, 58, 73], vec![], vec![1, 2, 4, 5, 6, 7], [6145, 3700, 2445, 0, 0, 2445]),
        ((2025, 4), vec![80], vec![april.id], vec![], [15025, 5000, 10025, 999, 50, 9975]),
    ];
    for (row, (key, bills, cash_rows, receipts, paise)) in result.months.iter().zip(expected) {
        assert_eq!(
            (
                row.key,
                &row.bills,
                &row.cash_rows,
                &row.receipts_excluded,
                row.paise
            ),
            (key, &bills, &cash_rows, &receipts, paise)
        );
        let rounded = row.paise.map(round);
        println!(
            "L16 month={:?} bills={:?} saved_cash_ids={:?} excluded_receipts={:?} paise(revenue,cost,gross,goods,other,net)={:?} misc_salary_paise={:?} reconciliation_rupees={rounded:?} display={:?}",
            row.key,
            row.bills,
            row.cash_rows,
            row.receipts_excluded,
            row.paise,
            row.misc_salary,
            row.display()
        );
    }
    assert_eq!(result.months.len(), 6);
    assert_eq!(
        result
            .months
            .iter()
            .map(|row| row.misc_salary)
            .collect::<Vec<_>>(),
        [[0, 0], [0, 0], [0, 0], [2302, 3403], [0, 0], [25, 25]]
    );
    assert_eq!(
        result
            .months
            .iter()
            .map(|row| row.display())
            .collect::<Vec<_>>(),
        [
            ["0", "₹0", "₹0", "—", "₹0", "₹0"],
            ["0", "₹0", "₹0", "—", "₹0", "₹0"],
            ["0", "₹0", "₹0", "—", "₹0", "₹0"],
            ["1", "₹10", "₹4", "₹12", "₹57", "₹-53"],
            ["4", "₹61", "₹24", "—", "₹0", "₹24"],
            ["1", "₹150", "₹100", "₹10", "₹1", "₹100"],
        ]
    );
    assert_eq!(
        result
            .months
            .iter()
            .map(|row| row.paise.map(round))
            .collect::<Vec<_>>(),
        [
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [10, 6, 4, 12, 57, -53],
            [61, 37, 24, 0, 0, 24],
            [150, 50, 100, 10, 1, 100],
        ]
    );
    assert_eq!(round(10025) - round(50), 99);
    assert_eq!(
        result.ranking,
        [
            (
                "Synthetic Grove".into(),
                19671,
                vec![
                    (36, Some("grove".into())),
                    (41, Some("grove".into())),
                    (73, Some("grove".into())),
                    (80, Some("same-name".into()))
                ]
            ),
            (
                "Synthetic Ancestor".into(),
                15025,
                vec![(81, Some("ancestor".into()))]
            ),
            ("Synthetic Guest".into(), 1499, vec![(58, None)]),
            (
                "Synthetic Orchard".into(),
                1049,
                vec![(12, Some("orchard".into()))]
            )
        ]
    );
    println!(
        "L16 all-time ranking name/paise={:?}; cancelled bill 24 excluded; bill 81 outside table included in ranking",
        result.ranking
    );
    let ranked_display: Vec<_> = result
        .ranking
        .iter()
        .map(|(name, amount, ids)| (name.as_str(), format!("₹{}", round(*amount)), ids))
        .collect();
    assert_eq!(
        ranked_display
            .iter()
            .map(|(name, amount, _)| (*name, amount.as_str()))
            .collect::<Vec<_>>(),
        [
            ("Synthetic Grove", "₹197"),
            ("Synthetic Ancestor", "₹150"),
            ("Synthetic Guest", "₹15"),
            ("Synthetic Orchard", "₹10")
        ]
    );
    println!("L16 all-time rounded ranking name/display/(bill,customer ID)={ranked_display:?}");
    for (id, expected) in [("grove", 3849), ("same-name", 14925)] {
        let ledger = fixture
            .customer_ledger(Role::Owner, &DraftCustomerId::new(id.into()))
            .unwrap();
        assert_eq!(ledger.closing.minor_units(), expected);
        assert_eq!(fixture.customer_due(id).minor_units(), expected);
        println!(
            "L16 separate account {id}: ledger={}p due={}p",
            ledger.closing.minor_units(),
            fixture.customer_due(id).minor_units()
        );
    }
    assert_eq!(
        report(&fixture, (&rows, date("2025-04-15")), Role::Manager),
        Err(ConfirmationError::AccessDenied)
    );
    println!("L16 Manager denied before projection; no payload");
    assert_eq!(fixture, before);
    assert_eq!(
        rows.map(|row| book.row(Role::Owner, row.id).unwrap()),
        before_rows
    );
}
