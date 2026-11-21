use super::*;

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
