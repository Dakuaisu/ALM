use super::*;
use crate::DraftCustomerId;

#[path = "customer_ageing_checks.rs"]
mod checks;

fn money(value: i64) -> Money {
    Money::from_minor_units(value)
}
fn date(value: &str) -> BillDate {
    value.parse().unwrap()
}
fn customer(value: &str) -> DraftCustomerId {
    DraftCustomerId::new(value.into())
}

fn bill(f: &mut ConfirmationFixture, id: u64, who: &str, when: &str, total: i64) {
    f.set_unit_price(money(total)).unwrap();
    f.register_draft(draft(id, 1).with_bill_date(Some(date(when))).with_customer(
        DraftCustomer::Selected {
            id: customer(who),
            name: "Same name".into(),
        },
    ))
    .unwrap();
    let review = f
        .review_named(Role::Owner, DraftId::new(id), money(i64::from(id == 9)))
        .unwrap();
    f.confirm(Role::Owner, RequestId::new(id), &review).unwrap();
}

fn receipt(f: &mut ConfirmationFixture, who: &str, amount: i64) {
    f.record_customer_receipt(
        Role::Owner,
        CustomerReceiptInput {
            customer_id: customer(who),
            date: date("2024-03-31"),
            amount: money(amount),
            mode: ReceiptMode::Cash,
        },
    )
    .unwrap();
}

#[test]
fn customer_ageing_replays_openings_thresholds_and_identity() {
    let mut f = fixture(30);
    for (id, when) in [
        (1, "2024-01-30"),
        (2, "2024-01-31"),
        (3, "2024-02-29"),
        (4, "2024-03-01"),
        (5, "2024-03-15"),
        (6, "2024-03-16"),
    ] {
        bill(&mut f, id, "a", when, 1000);
    }
    bill(&mut f, 7, "a", "2024-03-20", 50);
    bill(&mut f, 8, "a", "2024-03-21", 100);
    let entry = f.current_sales.get_mut(&DraftId::new(8)).unwrap();
    entry.1.received = Some(money(200));
    entry.1.due = Some(money(-100));
    *f.customer_balances.get_mut("a").unwrap() -= 200;
    bill(&mut f, 9, "a", "2024-01-01", 1000);
    f.cancel_sale(Role::Owner, DraftId::new(9)).unwrap();
    receipt(&mut f, "a", 200);
    receipt(&mut f, "a", -50);
    bill(&mut f, 10, "b", "2024-03-16", 100);
    receipt(&mut f, "b", 50);
    bill(&mut f, 11, "c", "2024-03-16", 101);
    receipt(&mut f, "c", 50);
    assert_eq!(
        f.review_named(Role::Owner, DraftId::new(11), money(102)),
        Err(ConfirmationError::InvalidAmount)
    );
    let history = (
        f.current_sales.clone(),
        f.preview_payments.clone(),
        f.lifecycle.clone(),
    );
    let before = f.clone();
    let baseline = f.customer_ageing(Role::Owner, date("2024-03-31")).unwrap();
    println!("before first opening: {baseline:?}");
    assert_eq!(
        baseline
            .rows
            .iter()
            .map(|row| (row.invoice, row.due.minor_units()))
            .collect::<Vec<_>>(),
        vec![
            (Some(1), 750),
            (Some(2), 1000),
            (Some(3), 1000),
            (Some(4), 1000),
            (Some(5), 1000),
            (Some(6), 1000),
            (Some(11), 51)
        ]
    );
    assert_eq!(
        (baseline.d15, baseline.d30, baseline.d60, baseline.old),
        (money(1051), money(2000), money(2000), money(750))
    );
    checks::check_report(&f, &baseline, [(5750, 5800), (0, 50), (51, 51)]);
    assert_eq!(f, before);
    f.save_customer_opening(
        Role::Owner,
        &customer("a"),
        OpeningBalance {
            amount: Some(money(500)),
            date: Some(date("2024-03-30")),
        },
    )
    .unwrap();
    assert_eq!(
        (
            f.current_sales.clone(),
            f.preview_payments.clone(),
            f.lifecycle.clone()
        ),
        history
    );
    let before = f.clone();
    let report = f.customer_ageing(Role::Owner, date("2024-03-31")).unwrap();
    println!(
        "opening 500, pool 250 (receipts +200 -50, overpaid bill 8 +100): allocation opening 250, remaining 250; report={report:?}; a due={:?}",
        f.customer_due("a")
    );
    assert_eq!(
        (
            report.rows[0].customer_id.value(),
            report.rows[0].invoice,
            report.rows[0].date,
            report.rows[0].age,
            report.rows[0].due
        ),
        ("a", None, None, 999, money(250))
    );
    assert_eq!(
        (report.d15, report.d30, report.d60, report.old),
        (money(1051), money(2000), money(2000), money(1250))
    );
    assert_eq!(f.customer_due("a"), money(6300));
    checks::check_report(&f, &report, [(6250, 6300), (0, 50), (51, 51)]);
    assert_eq!(f, before);
    f.save_customer_opening(
        Role::Owner,
        &customer("a"),
        OpeningBalance {
            amount: Some(money(300)),
            date: None,
        },
    )
    .unwrap();
    assert_eq!(
        (
            f.current_sales.clone(),
            f.preview_payments.clone(),
            f.lifecycle.clone()
        ),
        history
    );
    let before = f.clone();
    let report = f.customer_ageing(Role::Owner, date("2024-03-31")).unwrap();
    println!(
        "positive opening 300: pool 250 allocated to opening, residual 50 hidden; bill 7 initial 50 hidden; b bill 10 100 - receipt 50 = 50 hidden; c bill 11 101 - receipt 50 = 51 visible; report: {report:?}; a due={:?}; b due={:?}; c due={:?}",
        f.customer_due("a"),
        f.customer_due("b"),
        f.customer_due("c")
    );
    assert_eq!(
        report
            .rows
            .iter()
            .map(|r| (r.customer_id.value(), r.invoice, r.age, r.due.minor_units()))
            .collect::<Vec<_>>(),
        vec![
            ("a", Some(1), 61, 1000),
            ("a", Some(2), 60, 1000),
            ("a", Some(3), 31, 1000),
            ("a", Some(4), 30, 1000),
            ("a", Some(5), 16, 1000),
            ("a", Some(6), 15, 1000),
            ("c", Some(11), 15, 51)
        ]
    );
    assert_eq!(
        (report.d15, report.d30, report.d60, report.old),
        (money(1051), money(2000), money(2000), money(1000))
    );
    assert_eq!(f.customer_due("a"), money(6100));
    checks::check_report(&f, &report, [(6000, 6100), (0, 50), (51, 51)]);
    assert_eq!(
        f.customer_ledger_with_opening(Role::Owner, &customer("a"))
            .unwrap()
            .closing,
        money(6100)
    );
    assert_eq!(
        f.customer_ageing(Role::Manager, date("2024-03-31")),
        Err(ConfirmationError::AccessDenied)
    );
    assert_eq!(f, before);
    let positive_rows = report.rows.clone();
    f.save_customer_opening(
        Role::Owner,
        &customer("a"),
        OpeningBalance {
            amount: Some(money(-100)),
            date: None,
        },
    )
    .unwrap();
    let before = f.clone();
    let report = f.customer_ageing(Role::Owner, date("2024-03-31")).unwrap();
    println!(
        "negative opening -100: pool 350 allocated to bill 1 (1000 -> 650); report: {report:?}; a due={:?}",
        f.customer_due("a")
    );
    assert_eq!(
        (report.rows[0].invoice, report.rows[0].due),
        (Some(1), money(650))
    );
    assert_eq!(&report.rows[1..], &positive_rows[1..]);
    assert_eq!(
        (
            f.current_sales.clone(),
            f.preview_payments.clone(),
            f.lifecycle.clone()
        ),
        history
    );
    assert_eq!(
        (report.d15, report.d30, report.d60, report.old),
        (money(1051), money(2000), money(2000), money(650))
    );
    assert_eq!(f.customer_due("a"), money(5700));
    checks::check_report(&f, &report, [(5650, 5700), (0, 50), (51, 51)]);
    assert_eq!(
        f.customer_ledger_with_opening(Role::Owner, &customer("a"))
            .unwrap()
            .closing,
        money(5700)
    );
    assert_eq!(f, before);
}
