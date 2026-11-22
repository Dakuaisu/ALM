mod readback;
mod window;

use std::cmp::Reverse;
use std::fmt::{Arguments, Debug};

use crate::BillDate;
use crate::confirmation::ReceiptMode::{Cash, Online};

use readback::Source::{Bill, Receipt};
use readback::{Detail, Event, Readback, Replay, Sale, Source};
use window::{Period, Window, dashboard_windows};

// Legacy dashStats (2498–2514), limited to sales and collection inputs.
struct Stats<'a> {
    bills: Vec<&'a Sale>,
    pays: Vec<&'a Event>,
}

type Totals = (usize, i64, i64, i64, i64);

impl<'a> Stats<'a> {
    fn project(readback: &'a Readback, window: Window) -> Self {
        let bills = readback
            .sales
            .iter()
            .filter(|sale| !sale.cancelled && window.contains(sale.payment.date))
            .collect();
        let pays = readback
            .receipts
            .iter()
            .filter(|pay| window.contains(pay.date))
            .collect();
        Self { bills, pays }
    }

    fn totals(&self) -> Totals {
        let sales = self.bills.iter().map(|sale| sale.total).sum();
        let paid: i64 = self.bills.iter().map(|sale| sale.payment.detail.2).sum();
        let receipts: i64 = self.pays.iter().map(|pay| pay.detail.2).sum();
        (self.bills.len(), sales, paid, receipts, paid + receipts)
    }

    fn inputs(&self) -> [Vec<Source>; 2] {
        [
            self.bills
                .iter()
                .map(|sale| sale.payment.detail.0)
                .collect(),
            self.pays.iter().map(|pay| pay.detail.0).collect(),
        ]
    }

    // At 2793–2797 only current inputs go to collectedBreakdown (3151–3245).
    fn collected_detail(&self) -> (Vec<Detail>, [i64; 2]) {
        let mut rows: Vec<_> = self
            .bills
            .iter()
            .filter(|sale| sale.payment.detail.2 > 0)
            .map(|sale| &sale.payment)
            .chain(self.pays.iter().copied())
            .collect();
        rows.sort_by_key(|row| Reverse((row.date.parts(), row.created)));
        let details: Vec<_> = rows.into_iter().map(|row| row.detail).collect();
        let mut modes = [0; 2];
        for &(_, mode, paid) in &details {
            let bucket = match mode {
                Cash => &mut modes[0],
                Online => &mut modes[1],
            };
            *bucket += paid;
        }
        (details, modes)
    }
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
        let windows = dashboard_windows(period, today);
        compare(
            format_args!("{period:?} inclusive windows (current, prior)"),
            expected.windows,
            windows.map(Window::iso),
        );
        let [current, prior] = windows.map(|window| Stats::project(&readback, window));
        let totals = [current.totals(), prior.totals()];
        let (details, modes) = current.collected_detail();
        let prior_inputs = prior.inputs();
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
            current.inputs()[0].as_slice(),
        );
        compare(
            format_args!("{period:?} current drilldown (source, mode, paise)"),
            expected.current_details,
            details.as_slice(),
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
        for ([cash, online], total) in book_modes.into_iter().zip(totals) {
            assert_eq!(cash + online, total.4);
        }
    }

    let jan30 = "2024-01-30".parse().unwrap();
    let receipt_only = Stats::project(&readback, Window(jan30, jan30));
    compare(
        format_args!("Jan 30 receipt-only totals"),
        (0, 0, 0, 151, 151),
        receipt_only.totals(),
    );
    compare(
        format_args!("Jan 30 receipt-only inputs"),
        [vec![], vec![Receipt("r-jan30")]],
        receipt_only.inputs(),
    );
    assert_eq!(
        replay.fixture, before,
        "Dashboard reads must leave the fixture unchanged"
    );
}
