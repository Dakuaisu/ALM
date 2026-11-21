//! L14-01/L14-04, PHASE1_CASES.md:487. Legacy HTML is read as source only.
use std::num::NonZeroU32;

use alm_phase2_local::confirmation::{
    BalanceMode, ConfirmationFixture, CustomerLedgerKind, CustomerReceiptInput, ReceiptMode,
    RequestId,
};
use alm_phase2_local::{
    BillDate, DraftCustomer, DraftCustomerId, DraftId, DraftLine, Money, ProductId,
    ProvisionalDraft, Role,
};

#[path = "l14_opening_balance_replay/opening_actual.rs"]
mod opening_actual;

const CUSTOMER_CREATED: &str = "2025-02-11T00:15:00+05:30";
const FEB9: &str = "2025-02-09";
const FEB10: &str = "2025-02-10";
const OPENING: i64 = 2_000;

// Fixture-only conversion: minute-aligned instants within February 2025.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct UtcMinute(i32);

impl UtcMinute {
    fn parse(iso: &str) -> Self {
        fn minutes(hhmm: &str) -> i32 {
            let (hour, minute) = hhmm.split_once(':').unwrap();
            let (hour, minute) = (hour.parse::<i32>().unwrap(), minute.parse::<i32>().unwrap());
            assert!((0..24).contains(&hour) && (0..60).contains(&minute));
            hour * 60 + minute
        }

        let (date, time) = iso.split_once('T').unwrap();
        let _ = date.parse::<BillDate>().unwrap();
        let day = date
            .strip_prefix("2025-02-")
            .unwrap()
            .parse::<i32>()
            .unwrap();
        let (clock, zone) = time.split_at(8);
        let local = minutes(clock.strip_suffix(":00").unwrap());
        let offset = match zone {
            "Z" => 0,
            offset => minutes(offset.strip_prefix('+').unwrap()),
        };
        let utc = day * 1440 + local - offset;
        assert!((1440..29 * 1440).contains(&utc));
        Self(utc)
    }

    fn iso(self) -> String {
        let (day, minute) = (self.0 / 1440, self.0 % 1440);
        format!("2025-02-{day:02}T{:02}:{:02}:00Z", minute / 60, minute % 60)
    }
}

#[test]
fn l14_source_customer_created_crosses_midnight_when_converted_to_utc() {
    let utc = UtcMinute::parse(CUSTOMER_CREATED);
    let converted = utc.iso();
    assert_eq!(converted, "2025-02-10T18:45:00Z");
    assert_eq!(utc, UtcMinute::parse("2025-02-10T18:45:00Z"));
    println!("L14 source model: created={CUSTOMER_CREATED} -> utc={converted}");
}

#[derive(Clone, Copy)]
struct SourceTransaction {
    id: &'static str,
    date: &'static str,
    created: UtcMinute,
    delta: i64,
}

// HTML:7367-7398 sorts by document date first, then creation order (opening uses zero).
fn source_rows(
    created: Option<&str>,
    transactions: &[SourceTransaction],
) -> Vec<(&'static str, String, i64)> {
    let opening_date = created
        .map(|iso| UtcMinute::parse(iso).iso()[..10].to_owned())
        .unwrap_or_default();
    let mut entries = vec![("opening", opening_date, UtcMinute(0), OPENING)];
    entries.extend(
        transactions
            .iter()
            .map(|tx| (tx.id, tx.date.to_owned(), tx.created, tx.delta)),
    );
    entries.sort_by(|a, b| (&a.1, a.2).cmp(&(&b.1, b.2)));
    let mut running = 0;
    entries
        .into_iter()
        .map(|(id, date, _, delta)| {
            running += delta;
            (id, date, running)
        })
        .collect()
}

#[test]
fn l14_source_ledger_date_order_differs_from_estimate_creation_order() {
    // Given: active records for one customer, with each collection inserted in reverse creation order.
    let [b1, r1, b2, r2, r3] = [
        ("B1", FEB10, "19:00", 8_000),
        ("R1", FEB10, "19:01", -3_000),
        ("B2", FEB10, "19:02", 9_000),
        ("R2", FEB10, "19:03", -400),
        ("R3", FEB9, "19:04", -1_000),
    ]
    .map(|(id, date, clock, delta)| SourceTransaction {
        id,
        date,
        created: UtcMinute::parse(&format!("{FEB10}T{clock}:00Z")),
        delta,
    });
    let transactions = [b2, b1, r3, r2, r1];

    // When: replaying the ledger with both opening dates and the estimate for B2.
    let dated = source_rows(Some(CUSTOMER_CREATED), &transactions);
    let blank = source_rows(None, &transactions);
    // HTML:4322-4334 uses a strict creation cutoff, independent of document date.
    let before = OPENING
        + transactions
            .iter()
            .filter(|tx| tx.created < b2.created)
            .map(|tx| tx.delta)
            .sum::<i64>();
    let as_on = before + b2.delta;
    let latest = OPENING + transactions.iter().map(|tx| tx.delta).sum::<i64>();

    // Then: assert the exact rows, plus three distinct B2/closing balances.
    let expected_tail = [
        ("B1", FEB10, 9_000),
        ("R1", FEB10, 6_000),
        ("B2", FEB10, 15_000),
        ("R2", FEB10, 14_600),
    ];
    for (label, rows, head) in [
        (
            "dated",
            &dated,
            [("R3", FEB9, -1_000), ("opening", FEB10, 1_000)],
        ),
        (
            "blank",
            &blank,
            [("opening", "", 2_000), ("R3", FEB9, 1_000)],
        ),
    ] {
        let observed: Vec<_> = rows
            .iter()
            .map(|(id, date, running)| (*id, date.as_str(), *running))
            .collect();
        assert_eq!(
            observed,
            [head.as_slice(), &expected_tail].concat(),
            "{label}"
        );
        println!("L14 source model {label} (id, date, running p): {observed:?}");
    }
    let b2_row = dated.iter().find(|row| row.0 == "B2").unwrap().2;
    assert_eq!((as_on, b2_row, latest), (16_000, 15_000, 14_600));
    println!("L14 source model B2 (p): as-on={as_on}, row={b2_row}, latest={latest}");
}

fn rust_fixture() -> (ConfirmationFixture, DraftCustomerId, DraftId, DraftId) {
    // Given: a zero-start public fixture, without opening data or caller-set timestamps.
    let feb9: BillDate = FEB9.parse().unwrap();
    let feb10: BillDate = FEB10.parse().unwrap();
    let money = Money::from_minor_units;
    let customer_id = DraftCustomerId::new("l14-customer".into());
    let product = ProductId::new(1);
    let b1 = DraftId::new(20);
    let b2 = DraftId::new(10); // Inverted draft IDs distinguish key order from invoice order.
    let mut fixture = ConfirmationFixture::new(product, 3, money(10_000)).unwrap();
    let receipt = |date, amount, mode| CustomerReceiptInput {
        customer_id: customer_id.clone(),
        date,
        amount: money(amount),
        mode,
    };

    // When: post B1, R1, B2, R2, then the backdated R3 through the real APIs.
    for (request, id, paid, payment) in [
        (
            RequestId::new(101),
            b1,
            2_000,
            receipt(feb10, 3_000, ReceiptMode::Cash),
        ),
        (
            RequestId::new(102),
            b2,
            1_000,
            receipt(feb10, 400, ReceiptMode::Online),
        ),
    ] {
        let draft = ProvisionalDraft::new(id, vec![DraftLine::new(product, NonZeroU32::MIN)])
            .with_bill_date(Some(feb10))
            .with_customer(DraftCustomer::Selected {
                id: customer_id.clone(),
                name: "Synthetic customer".into(),
            });
        fixture.register_draft(draft).unwrap();
        let review = fixture.review_named(Role::Owner, id, money(paid)).unwrap();
        fixture.confirm(Role::Owner, request, &review).unwrap();
        fixture
            .record_customer_receipt(Role::Owner, payment)
            .unwrap();
    }
    fixture
        .record_customer_receipt(Role::Owner, receipt(feb9, 1_000, ReceiptMode::Cash))
        .unwrap();

    (fixture, customer_id, b1, b2)
}

#[test]
fn l14_rust_preview_uses_posting_order_when_receipt_is_backdated() {
    let (fixture, customer_id, b1, b2) = rust_fixture();
    let feb9: BillDate = FEB9.parse().unwrap();
    let feb10: BillDate = FEB10.parse().unwrap();
    let money = Money::from_minor_units;

    // Then: the ledger is document-date ordered; the preview is bounded by posting order.
    let ledger = fixture.customer_ledger(Role::Owner, &customer_id).unwrap();
    let rows: Vec<_> = ledger
        .entries
        .iter()
        .map(|entry| {
            (
                entry.date,
                &entry.kind,
                entry.debit.minor_units(),
                entry.credit.minor_units(),
                entry.balance.minor_units(),
            )
        })
        .collect();
    let bill_kind = |draft_id, invoice| CustomerLedgerKind::Bill { draft_id, invoice };
    let cash = CustomerLedgerKind::Receipt {
        mode: ReceiptMode::Cash,
    };
    let online = CustomerLedgerKind::Receipt {
        mode: ReceiptMode::Online,
    };
    assert_eq!(
        rows,
        [
            (feb9, &cash, 0, 1_000, -1_000),
            (feb10, &bill_kind(b1, 1), 10_000, 2_000, 7_000),
            (feb10, &cash, 0, 3_000, 4_000),
            (feb10, &bill_kind(b2, 2), 10_000, 1_000, 13_000),
            (feb10, &online, 0, 400, 12_600),
        ]
    );
    assert_eq!(ledger.closing, money(12_600));
    let [as_on, latest] = [BalanceMode::AsOnBillDate, BalanceMode::Latest]
        .map(|mode| fixture.bill_preview(Role::Owner, b2, mode).unwrap());
    assert_eq!(
        (as_on.invoice, as_on.date, as_on.balance_mode),
        (2, feb10, BalanceMode::AsOnBillDate)
    );
    assert_eq!(
        (latest.invoice, latest.date, latest.balance_mode),
        (2, feb10, BalanceMode::Latest)
    );
    let b2_row = rows[3].4;
    let (as_on, latest) = (as_on.balance.minor_units(), latest.balance.minor_units());
    assert_eq!((as_on, b2_row, latest), (14_000, 13_000, 12_600));
    println!(
        "L14 Rust API ledger (no opening or created timestamps; date, kind, debit p, credit p, running p):"
    );
    for (date, kind, debit, credit, running) in &rows {
        println!("  {date} {kind:?} {debit} {credit} {running}");
    }
    println!("L14 Rust API bill_preview B2 (p): as-on={as_on}, row={b2_row}, latest={latest}");
}
