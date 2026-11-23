mod readback;
use std::fmt::{Arguments, Debug};

use crate::BillDate;
use crate::confirmation::ReceiptMode::{Cash, Online};

use super::super::{DashboardPeriod as Period, DashboardSource, DashboardSummary};
use readback::Source::{Bill, Receipt};
use readback::{Detail, Replay, Source};

type Totals = (usize, i64, i64, i64, i64);
fn source(source: DashboardSource) -> Source {
    match source {
        DashboardSource::Bill { draft_id, invoice } => Bill(draft_id.value(), invoice),
        DashboardSource::Receipt(id) => Receipt(["r-jan25", "r-jan30", "r-feb01"][id.0]),
    }
}

fn totals(s: &DashboardSummary) -> Totals {
    (
        s.bill_count,
        s.sales.minor_units(),
        s.bill_paid.minor_units(),
        s.receipts.minor_units(),
        s.collected.minor_units(),
    )
}

struct Expectation {
    period: Period,
    windows: [[&'static str; 2]; 2],
    totals: [Totals; 2],
    current_bills: &'static [Source],
    current_details: &'static [Detail],
    prior_inputs: [&'static [Source]; 2],
    cash_online: [[i64; 2]; 2],
}

const FEB1_DETAILS: &[Detail] = &[
    (Bill(5, 8), Online, 251),
    (Receipt("r-feb01"), Cash, 249),
    (Bill(81, 7), Cash, 199),
];

const EXPECTED: [Expectation; 3] = [
    Expectation {
        period: Period::Today,
        windows: [["2024-02-01", "2024-02-01"], ["2024-01-31", "2024-01-31"]],
        totals: [(2, 3603, 450, 249, 699), (1, 2402, 449, 0, 449)],
        current_bills: &[Bill(5, 8), Bill(81, 7)],
        current_details: FEB1_DETAILS,
        prior_inputs: [&[Bill(42, 10)], &[]],
        cash_online: [[448, 251], [0, 449]],
    },
    Expectation {
        period: Period::Week,
        windows: [["2024-01-26", "2024-02-01"], ["2024-01-19", "2024-01-25"]],
        totals: [(4, 7206, 1248, 400, 1648), (2, 3603, 478, 51, 529)],
        current_bills: &[Bill(42, 10), Bill(5, 8), Bill(81, 7), Bill(26, 6)],
        current_details: &[
            (Bill(5, 8), Online, 251),
            (Receipt("r-feb01"), Cash, 249),
            (Bill(81, 7), Cash, 199),
            (Bill(42, 10), Online, 449),
            (Receipt("r-jan30"), Online, 151),
            (Bill(26, 6), Cash, 349),
        ],
        prior_inputs: [&[Bill(25, 5), Bill(19, 4)], &[Receipt("r-jan25")]],
        cash_online: [[797, 851], [278, 251]],
    },
    Expectation {
        period: Period::Month,
        windows: [["2024-02-01", "2024-02-01"], ["2024-01-01", "2024-01-31"]],
        totals: [(2, 3603, 450, 249, 699), (6, 9608, 1584, 202, 1786)],
        current_bills: &[Bill(5, 8), Bill(81, 7)],
        current_details: FEB1_DETAILS,
        prior_inputs: [
            &[
                Bill(42, 10),
                Bill(26, 6),
                Bill(25, 5),
                Bill(19, 4),
                Bill(83, 3),
                Bill(31, 2),
            ],
            &[Receipt("r-jan30"), Receipt("r-jan25")],
        ],
        cash_online: [[448, 251], [786, 1000]],
    },
];

fn compare<E: Debug>(label: Arguments<'_>, expected: E, observed: impl Debug + PartialEq<E>) {
    println!("{label}: expected={expected:?} observed={observed:?}");
    assert_eq!(observed, expected, "{label}");
}

#[test]
fn l02_02_l02_03_dashboard_replay() {
    // Given: frozen Feb 1, a separate opening account, and app-posted date controls.
    let today = "2024-02-01".parse::<BillDate>().unwrap();
    let replay = Replay::seed();
    let before = replay.fixture.clone();
    compare(
        format_args!("Manager Dashboard result (no payload)"),
        Err(super::super::DashboardError::AccessDenied),
        replay
            .fixture
            .dashboard(crate::Role::Manager, today, Period::Today),
    );
    let readback = replay.read();
    let cashbook = [
        ("2023-12-31", [111, 0]),
        ("2024-01-01", [0, 149]),
        ("2024-01-18", [159, 0]),
        ("2024-01-19", [227, 0]),
        ("2024-01-25", [51, 251]),
        ("2024-01-26", [349, 0]),
        ("2024-01-30", [0, 151]),
        ("2024-01-31", [0, 449]),
        ("2024-02-01", [448, 251]),
        ("2024-02-02", [811, 0]),
    ]
    .map(|(date, expected)| {
        let date = date.parse::<BillDate>().unwrap();
        let observed = replay.cashbook_paise(date, &readback);
        compare(
            format_args!("{date} Cash Book (Cash, Online) paise"),
            expected,
            observed,
        );
        (date, observed)
    });

    // When: replay each period through the source-derived selection and drilldown.
    println!(
        "Totals: (bill count, sales, bill paid, standalone receipts, collected); money in paise"
    );
    for expected in EXPECTED {
        let period = expected.period;
        let dashboard = replay
            .fixture
            .dashboard(crate::Role::Owner, today, period)
            .unwrap();
        let windows = [dashboard.current.window, dashboard.prior.window];
        compare(
            format_args!("{period:?} inclusive windows (current, prior)"),
            expected.windows,
            windows.map(|w| [w.start.to_string(), w.end.to_string()]),
        );
        let totals = [totals(&dashboard.current), totals(&dashboard.prior)];
        let details: Vec<_> = dashboard
            .collected_detail
            .iter()
            .map(|row| (source(row.source), row.mode, row.amount.minor_units()))
            .collect();
        let modes = [
            dashboard.current.cash.minor_units(),
            dashboard.current.online.minor_units(),
        ];
        let prior_modes = [
            dashboard.prior.cash.minor_units(),
            dashboard.prior.online.minor_units(),
        ];
        let prior_inputs = [
            &dashboard.prior.bill_inputs,
            &dashboard.prior.receipt_inputs,
        ]
        .map(|items| items.iter().copied().map(source).collect::<Vec<_>>());
        let book_modes = windows.map(|window| {
            cashbook
                .iter()
                .filter(|(date, _)| window.contains(*date))
                .fold([0, 0], |[cash, online], (_, [day_cash, day_online])| {
                    [cash + day_cash, online + day_online]
                })
        });

        // Then: compare literal metrics, current modal rows and prior inputs separately.
        compare(
            format_args!("{period:?} totals (current, prior)"),
            expected.totals,
            totals,
        );
        compare(
            format_args!("{period:?} current Sales bill inputs"),
            expected.current_bills,
            dashboard
                .sales_ids
                .iter()
                .copied()
                .map(source)
                .collect::<Vec<_>>()
                .as_slice(),
        );
        compare(
            format_args!("{period:?} current drilldown (source, mode, paise)"),
            expected.current_details,
            details.as_slice(),
        );
        compare(
            format_args!("{period:?} current drilldown document dates"),
            match period {
                Period::Today | Period::Month => &["2024-02-01"; 3][..],
                Period::Week => &[
                    "2024-02-01",
                    "2024-02-01",
                    "2024-02-01",
                    "2024-01-31",
                    "2024-01-30",
                    "2024-01-26",
                ],
            },
            dashboard
                .collected_detail
                .iter()
                .map(|row| row.date.to_string())
                .collect::<Vec<_>>()
                .as_slice(),
        );
        compare(
            format_args!("{period:?} prior inputs (bills, receipts)"),
            expected.prior_inputs,
            prior_inputs.each_ref().map(Vec::as_slice),
        );
        compare(
            format_args!("{period:?} Cash Book Cash/Online paise (current, prior)"),
            expected.cash_online,
            book_modes,
        );
        compare(
            format_args!("{period:?} current drilldown Cash/Online paise"),
            expected.cash_online[0],
            modes,
        );
        compare(
            format_args!("{period:?} comparison Cash/Online paise"),
            expected.cash_online[1],
            prior_modes,
        );
        for ([cash, online], total) in book_modes.into_iter().zip(totals) {
            assert_eq!(cash + online, total.4);
        }
    }

    let jan30 = "2024-01-30".parse().unwrap();
    let receipt_only = replay
        .fixture
        .dashboard(crate::Role::Owner, jan30, Period::Today)
        .unwrap();
    compare(
        format_args!("Jan 30 receipt-only totals"),
        (0, 0, 0, 151, 151),
        totals(&receipt_only.current),
    );
    compare(
        format_args!("Jan 30 receipt-only inputs"),
        [vec![], vec![Receipt("r-jan30")]],
        [
            &receipt_only.current.bill_inputs,
            &receipt_only.current.receipt_inputs,
        ]
        .map(|items| items.iter().copied().map(source).collect::<Vec<_>>()),
    );
    compare(
        format_args!("Dashboard reads left the fixture unchanged"),
        true,
        replay.fixture == before,
    );
}
