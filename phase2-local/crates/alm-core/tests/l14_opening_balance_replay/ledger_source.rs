use alm_core::BillDate;

const CUSTOMER_CREATED: &str = "2025-02-11T00:15:00+05:30";
const FEB9: &str = "2025-02-09";
const FEB10: &str = "2025-02-10";
const OPENING: i64 = 2_000;

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
    let dated = source_rows(Some(CUSTOMER_CREATED), &transactions);
    let blank = source_rows(None, &transactions);
    let before = OPENING
        + transactions
            .iter()
            .filter(|tx| tx.created < b2.created)
            .map(|tx| tx.delta)
            .sum::<i64>();
    let as_on = before + b2.delta;
    let latest = OPENING + transactions.iter().map(|tx| tx.delta).sum::<i64>();
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
