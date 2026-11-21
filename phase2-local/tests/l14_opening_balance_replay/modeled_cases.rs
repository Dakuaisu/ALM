use super::model::{
    AgeRow, Bill, Customer, DeleteGuard, LedgerRow, OpeningInput, Payment, Records,
};

fn age_row(date: Option<&'static str>, due: i64, band: (i64, &'static str)) -> AgeRow {
    AgeRow {
        date,
        due,
        age: band.0,
        bucket: band.1,
    }
}

fn fixture(same_name: bool) -> Records {
    let customer = |id, name, opening| Customer {
        id,
        name,
        opening,
        created: Some(1_738_368_000_000),
        created_date_utc: "2025-02-01",
        version: 7,
        modified: 1_738_368_000_000,
        device: "old-device",
    };
    Records {
        customers: vec![
            customer("acct-a", "Synthetic", 1_200),
            customer(
                "acct-b",
                if same_name { "Synthetic" } else { "Other" },
                1_100,
            ),
        ],
        bills: vec![
            Bill {
                id: "bill-a",
                customer_id: "acct-a",
                customer_name: "Synthetic",
                date: "2025-02-10",
                created: 10,
                total: 10_000,
                paid: 2_000,
                cancelled: false,
            },
            Bill {
                id: "cancelled-a",
                customer_id: "acct-a",
                customer_name: "Synthetic",
                date: "2025-02-11",
                created: 11,
                total: 90_000,
                paid: 100,
                cancelled: true,
            },
            Bill {
                id: "bill-b",
                customer_id: "acct-b",
                customer_name: if same_name { "Synthetic" } else { "Other" },
                date: "2025-02-10",
                created: 12,
                total: 8_000,
                paid: 1_000,
                cancelled: false,
            },
        ],
        payments: vec![
            Payment {
                id: "receipt-a",
                customer_id: "acct-a",
                customer_name: "Synthetic",
                date: "2025-02-12",
                created: 13,
                amount: 3_000,
            },
            Payment {
                id: "receipt-b",
                customer_id: "acct-b",
                customer_name: if same_name { "Synthetic" } else { "Other" },
                date: "2025-02-12",
                created: 14,
                amount: 500,
            },
        ],
    }
}

#[test]
fn opening_save_updates_stamp_and_separate_derived_views() {
    for (input, opening, closing, expected_age) in [
        (
            OpeningInput::Amount(4_500),
            4_500,
            9_500,
            vec![
                age_row(None, 1_500, (999, "old")),
                age_row(Some("2025-02-10"), 8_000, (15, "d15")),
            ],
        ),
        (OpeningInput::Amount(-7_500), -7_500, -2_500, vec![]),
        (
            OpeningInput::Amount(0),
            0,
            5_000,
            vec![age_row(Some("2025-02-10"), 5_000, (15, "d15"))],
        ),
        (
            OpeningInput::Blank,
            0,
            5_000,
            vec![age_row(Some("2025-02-10"), 5_000, (15, "d15"))],
        ),
    ] {
        let mut records = fixture(false);
        let before = records.clone();
        assert_eq!(before.customers[0].opening, 1_200);
        assert_eq!(before.customers[0].created, Some(20_120 * 86_400_000));
        assert!(records.save_opening(input));
        let target = &records.customers[0];
        assert_eq!(
            (
                target.opening,
                target.version,
                target.modified,
                target.device,
                target.created
            ),
            (
                opening,
                8,
                1_740_441_600_000,
                "synthetic-device",
                before.customers[0].created
            )
        );
        assert_eq!(target.modified, 20_144 * 86_400_000);
        assert_eq!(records.customers[1], before.customers[1]);
        assert_eq!(records.bills, before.bills);
        assert_eq!(records.payments, before.payments);
        assert_eq!(records.exact("acct-b"), before.exact("acct-b"));
        assert_eq!(records.rounded("acct-b"), before.rounded("acct-b"));
        assert_eq!(records.ledger("acct-b"), before.ledger("acct-b"));
        assert_eq!(records.ageing("acct-b"), before.ageing("acct-b"));
        assert_eq!(records.exact("acct-a"), closing);
        assert_eq!(records.rounded("acct-a"), closing / 100);
        assert_eq!(
            records.ledger("acct-a"),
            vec![
                LedgerRow {
                    date: "2025-02-01",
                    kind: "opening",
                    debit: opening,
                    credit: 0,
                    running: opening
                },
                LedgerRow {
                    date: "2025-02-10",
                    kind: "bill",
                    debit: 10_000,
                    credit: 2_000,
                    running: opening + 8_000
                },
                LedgerRow {
                    date: "2025-02-12",
                    kind: "payment",
                    debit: 0,
                    credit: 3_000,
                    running: closing
                },
            ]
        );
        assert_eq!(records.ageing("acct-a"), expected_age);
        println!(
            "input={input:?} stamp={target:?} exact_paise={} rounded_rupees={} ledger={:?} ageing={:?} acct_b_exact={} bills={:?} payments={:?}",
            records.exact("acct-a"),
            records.rounded("acct-a"),
            records.ledger("acct-a"),
            records.ageing("acct-a"),
            records.exact("acct-b"),
            records.bills,
            records.payments
        );
    }
}

#[test]
fn existing_same_name_accounts_are_isolated_but_editor_rejects_save() {
    let records = fixture(true);
    let before = records.clone();
    assert_eq!(
        (records.exact("acct-a"), records.exact("acct-b")),
        (6_200, 7_600)
    );
    assert_eq!(records.ledger("acct-a").len(), 3);
    assert_eq!(records.ledger("acct-b").len(), 3);
    assert_eq!(
        records.ageing("acct-a"),
        vec![age_row(Some("2025-02-10"), 6_200, (15, "d15"))]
    );
    assert_eq!(
        records.ageing("acct-b"),
        vec![
            age_row(None, 600, (999, "old")),
            age_row(Some("2025-02-10"), 7_000, (15, "d15"))
        ]
    );
    for input in [
        OpeningInput::Amount(4_500),
        OpeningInput::Amount(-7_500),
        OpeningInput::Amount(0),
        OpeningInput::Blank,
    ] {
        let mut copy = records.clone();
        assert!(!copy.save_opening(input));
        assert_eq!(copy, before);
        println!("same-name input={input:?} rejected; records unchanged: {copy:?}");
    }
}

#[test]
fn delete_guard_uses_rounded_balance_not_exact_balance() {
    for (opening, exact, rounded, expected_guard) in [
        (4_500, 9_500, 95, DeleteGuard::Debt(95)),
        (-7_500, -2_500, -25, DeleteGuard::Advance(25)),
        (-5_000, 0, 0, DeleteGuard::OfferConfirmation),
        (-4_975, 25, 0, DeleteGuard::OfferConfirmation),
        (-4_950, 50, 1, DeleteGuard::Debt(1)),
        (-5_050, -50, 0, DeleteGuard::OfferConfirmation),
    ] {
        let mut records = fixture(false);
        assert!(records.save_opening(OpeningInput::Amount(opening)));
        let before_guard = records.clone();
        let observed_exact = records.exact("acct-a");
        let observed_rounded = records.rounded("acct-a");
        let observed_guard = records.delete_guard("acct-a");
        assert_eq!((observed_exact, observed_rounded), (exact, rounded));
        assert_eq!(observed_guard, expected_guard);
        assert_eq!(records, before_guard);
        println!(
            "guard opening_paise={opening} exact_paise={observed_exact} rounded_rupees={observed_rounded} outcome={observed_guard:?}; no confirmation or deletion modeled"
        );
    }
}
