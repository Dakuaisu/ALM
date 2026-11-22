use super::model::{CUTOFF, Item, Line, Product, Row, Sale, report};

#[test]
fn helper_only_historical_name_fallback_uses_exact_current_text_and_strict_cutoff() {
    let product = Product {
        id: "live",
        name: "Current label",
        packet_qty: 10.0,
        opening: 5.0,
        buy: Some(11.0),
        layers: vec![(2.0, 3.0), (3.0, 7.0)],
    };
    for (pid, name, sale_date, key, on_hand, last) in [
        (None, "Current label", CUTOFF, "live", 4.0, Some(CUTOFF)),
        (
            None,
            "Current label",
            "2024-10-03",
            "live",
            4.0,
            Some("2024-10-03"),
        ),
        (None, "current label", CUTOFF, "n:current label", 5.0, None),
        (
            Some("retired"),
            "Current label",
            CUTOFF,
            "retired",
            5.0,
            None,
        ),
        (
            Some("live"),
            "Former label",
            CUTOFF,
            "live",
            4.0,
            Some(CUTOFF),
        ),
    ] {
        let sale = Sale {
            id: Some("seeded history"),
            date: sale_date,
            cancelled: false,
            items: vec![Item::Line(Line {
                pid,
                name,
                qty: 1.0,
                pieces: false,
                packet_qty: None,
            })],
        };
        let actual = report(std::slice::from_ref(&product), &[sale]);
        assert_eq!(actual.keys.len(), 1);
        assert_eq!(actual.keys.get(key), Some(&(1.0, sale_date)));
        assert_eq!(actual.candidates["live"], (on_hand, last));
        match last {
            Some(CUTOFF) => assert!(actual.rows.is_empty()),
            Some("2024-10-03") => assert_eq!(
                actual.rows,
                [Row {
                    id: "live",
                    on_hand: 4.0,
                    last,
                    raw: 44.0,
                    displayed: "₹44".into(),
                    fifo: 24.0,
                }]
            ),
            None => assert_eq!(
                actual.rows,
                [Row {
                    id: "live",
                    on_hand: 5.0,
                    last,
                    raw: 55.0,
                    displayed: "₹55".into(),
                    fifo: 27.0,
                }]
            ),
            Some(unexpected) => panic!("unexpected fixture date {unexpected}"),
        }
        println!(
            "Helper-only legacy identity pid={pid:?} name={name:?} sale_date={sale_date} key={key:?} candidate={:?}",
            actual.candidates["live"]
        );
    }
}
