use super::*;

#[path = "customer_receipts.rs"]
mod customer_receipts;

#[test]
fn registered_empty_accounts_preserve_exact_opening_and_ageing_readbacks() {
    let mut f = ConfirmationFixture::new(ProductId::new(1), 5, money(125)).unwrap();
    let target = customer("registered-target");
    let other = customer("registered-other");
    for id in [&target, &other, &target] {
        assert_eq!(f.register_customer(Role::Owner, id), Ok(()));
    }
    let unchanged = f.customer_ledger_with_opening(Role::Owner, &other).unwrap();
    let original_effects = (
        f.stock(),
        f.ledger,
        f.received_ledger,
        f.next_invoice,
        f.current_sales.clone(),
        f.requests.clone(),
        f.preview_payments.clone(),
    );
    for amount in [51, 50, -125] {
        let opening = OpeningBalance {
            amount: Some(money(amount)),
            date: Some(date("2024-03-01")),
        };
        f.save_customer_opening(Role::Owner, &target, opening)
            .unwrap();
        let ledger = f
            .customer_ledger_with_opening(Role::Owner, &target)
            .unwrap();
        let transactions = f.customer_ledger(Role::Owner, &target).unwrap();
        let report = f.customer_ageing(Role::Owner, date("2024-03-31")).unwrap();
        println!(
            "opening={amount} paise; opening ledger={ledger:?}; transaction ledger={transactions:?}; ageing={report:?}; other={unchanged:?}"
        );
        assert_eq!(
            ledger.entries,
            vec![OpeningLedgerEntry {
                date: opening.date,
                kind: OpeningLedgerKind::Opening,
                debit: money(amount),
                credit: money(0),
                balance: money(amount),
            }]
        );
        assert_eq!(ledger.closing, money(amount));
        assert_eq!(
            transactions,
            CustomerLedger {
                entries: vec![],
                closing: money(amount)
            }
        );
        if amount == 51 {
            assert_eq!(
                report.rows,
                vec![AgeingRow {
                    customer_id: target.clone(),
                    draft_id: None,
                    invoice: None,
                    date: None,
                    age: 999,
                    bucket: AgeingBucket::Old,
                    due: money(51)
                }]
            );
            assert_eq!(
                (report.d15, report.d30, report.d60, report.old),
                (money(0), money(0), money(0), money(51))
            );
        } else {
            assert!(report.rows.is_empty());
            assert_eq!(
                (report.d15, report.d30, report.d60, report.old),
                (money(0), money(0), money(0), money(0))
            );
        }
        assert_eq!(
            f.customer_ledger_with_opening(Role::Owner, &other),
            Ok(unchanged.clone())
        );
        assert_eq!(f.customer_due(target.value()), money(amount));
    }
    assert_eq!(
        unchanged,
        OpeningLedger {
            entries: vec![],
            closing: money(0)
        }
    );
    assert_eq!(
        (
            f.stock(),
            f.ledger,
            f.received_ledger,
            f.next_invoice,
            f.current_sales.clone(),
            f.requests.clone(),
            f.preview_payments.clone()
        ),
        original_effects
    );
}

#[test]
fn registered_accounts_reject_invalid_access() {
    let mut f = ConfirmationFixture::new(ProductId::new(1), 5, money(125)).unwrap();
    let id = customer("registered");
    let unknown = customer("unknown");
    let opening = OpeningBalance {
        amount: Some(money(51)),
        date: None,
    };
    let before = f.clone();
    assert_eq!(
        f.register_customer(Role::Owner, &customer(" \t")),
        Err(ConfirmationError::InvalidDraft)
    );
    assert_eq!(
        f.register_customer(Role::Manager, &id),
        Err(ConfirmationError::AccessDenied)
    );
    assert_eq!(f, before);
    assert_eq!(
        f.customer_ledger(Role::Owner, &unknown),
        Err(ConfirmationError::InvalidDraft)
    );
    assert_eq!(
        f.customer_ledger_with_opening(Role::Owner, &unknown),
        Err(ConfirmationError::InvalidDraft)
    );
    assert_eq!(
        f.save_customer_opening(Role::Owner, &unknown, opening),
        Err(ConfirmationError::InvalidDraft)
    );
    f.register_customer(Role::Owner, &id).unwrap();
    let before = f.clone();
    assert_eq!(f.register_customer(Role::Owner, &id), Ok(()));
    assert_eq!(
        f.customer_ledger(Role::Manager, &id),
        Err(ConfirmationError::AccessDenied)
    );
    assert_eq!(
        f.customer_ledger_with_opening(Role::Manager, &id),
        Err(ConfirmationError::AccessDenied)
    );
    assert_eq!(
        f.customer_ageing(Role::Manager, date("2024-03-31")),
        Err(ConfirmationError::AccessDenied)
    );
    assert_eq!(
        f.save_customer_opening(Role::Manager, &id, opening),
        Err(ConfirmationError::AccessDenied)
    );
    assert_eq!(
        f.record_customer_receipt(
            Role::Owner,
            CustomerReceiptInput {
                customer_id: unknown,
                date: date("2024-03-31"),
                amount: money(1),
                mode: ReceiptMode::Cash,
            }
        ),
        Err(ConfirmationError::InvalidDraft)
    );
    assert_eq!(f, before);
}

#[test]
fn registered_sold_account_is_included_once_in_ageing() {
    let mut f = fixture(2);
    bill(&mut f, 1, "sold", "2024-03-16", 100);
    f.register_customer(Role::Owner, &customer("sold")).unwrap();
    let report = f.customer_ageing(Role::Owner, date("2024-03-31")).unwrap();
    println!("registered sold account ageing={report:?}");
    assert_eq!(report.rows.len(), 1);
    assert_eq!(
        (report.rows[0].invoice, report.rows[0].due, report.d15),
        (Some(1), money(100), money(100))
    );
}

#[test]
fn customer_ageing_applies_negative_receipt_pool_to_oldest_debt() {
    let mut f = fixture(2);
    bill(&mut f, 1, "negative", "2024-03-16", 100);
    receipt(&mut f, "negative", -60);
    let before = f.clone();
    let report = f.customer_ageing(Role::Owner, date("2024-03-31")).unwrap();
    println!(
        "negative receipt pool -60, customer negative bill 1 100 -> 160; report={report:?}; balance={:?}",
        f.customer_due("negative")
    );
    assert_eq!(
        (report.rows[0].invoice, report.rows[0].due, report.d15),
        (Some(1), money(160), money(160))
    );
    assert_eq!(f.customer_due("negative"), money(160));
    assert_eq!(f, before);
}

pub(super) fn check_report(
    f: &ConfirmationFixture,
    report: &AgeingReport,
    expected: [(i64, i64); 3],
) {
    for row in &report.rows {
        let (draft, bucket) = match row.invoice {
            None => (None, AgeingBucket::Old),
            Some(1) => (Some(DraftId::new(1)), AgeingBucket::Old),
            Some(2) => (Some(DraftId::new(2)), AgeingBucket::D60),
            Some(3) => (Some(DraftId::new(3)), AgeingBucket::D60),
            Some(4) => (Some(DraftId::new(4)), AgeingBucket::D30),
            Some(5) => (Some(DraftId::new(5)), AgeingBucket::D30),
            Some(6) => (Some(DraftId::new(6)), AgeingBucket::D15),
            Some(11) => (Some(DraftId::new(11)), AgeingBucket::D15),
            other => panic!("unexpected invoice {other:?}"),
        };
        assert_eq!(row.draft_id, draft);
        assert_eq!(row.bucket, bucket);
    }
    for (id, (visible_expected, due_expected)) in ["a", "b", "c"].into_iter().zip(expected) {
        let visible: i64 = report
            .rows
            .iter()
            .filter(|row| row.customer_id.value() == id)
            .map(|row| row.due.minor_units())
            .sum();
        let due = f
            .customer_ledger_with_opening(Role::Owner, &customer(id))
            .unwrap()
            .closing
            .minor_units();
        println!("customer {id}: visible={visible} paise, ledger due={due} paise");
        assert_eq!(visible, visible_expected);
        assert_eq!(due, due_expected);
        assert_eq!(f.customer_due(id).minor_units(), due);
    }
}
