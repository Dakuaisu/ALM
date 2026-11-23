use super::{Capture, Records, decode};

#[derive(Clone, Copy, Debug)]
pub(super) enum Surface {
    Customers,
    Ledger,
    Udhaar,
    Debtors,
    Ageing,
}

#[derive(Clone, Copy)]
pub(super) struct Row {
    pub(super) surface: Surface,
    pub(super) index: usize,
    pub(super) ageing_due_paise: i64,
}

pub(super) fn visible(surface: Surface, mobile: &str, balance: i64, ageing_due_paise: i64) -> bool {
    match surface {
        Surface::Customers | Surface::Ledger | Surface::Udhaar => !mobile.is_empty() && balance > 0,
        Surface::Debtors => balance > 0 && !mobile.is_empty(),
        Surface::Ageing => ageing_due_paise > 50 && !mobile.is_empty(),
    }
}

pub(super) fn click_customer(capture: &mut Capture, records: &Records, row: Row) -> bool {
    let customer = &records.customers[row.index];
    let shown = visible(
        row.surface,
        customer.mobile,
        records.balance(customer.id),
        row.ageing_due_paise,
    );
    if shown {
        capture.customer(records, customer);
    }
    shown
}

pub(super) fn exercise_surfaces(records: &Records, expected: &str) {
    let surfaces = [
        Surface::Customers,
        Surface::Ledger,
        Surface::Udhaar,
        Surface::Debtors,
        Surface::Ageing,
    ];
    for surface in surfaces {
        let mut good = Capture::default();
        let mut bad = Capture::default();
        let mut missing = Capture::default();
        let mut advance = Capture::default();
        let row = |index, ageing_due_paise| Row {
            surface,
            index,
            ageing_due_paise,
        };
        let observed = [
            click_customer(&mut good, records, row(0, 25_000)),
            click_customer(&mut bad, records, row(1, 10_000)),
            click_customer(&mut missing, records, row(2, 10_000)),
            click_customer(&mut advance, records, row(3, 0)),
        ];
        assert_eq!(observed, [true, true, false, false]);
        assert_eq!(missing, Capture::default());
        assert_eq!(advance, Capture::default());
        assert_eq!(bad.warnings, [("No valid mobile for A & B".into(), "warn")]);
        assert!(bad.opens.is_empty() && bad.attachments.is_empty());
        assert!(good.warnings.is_empty() && good.attachments.is_empty());
        assert_eq!(good.opens.len(), 1);
        let (url, target) = &good.opens[0];
        assert_eq!(*target, "_blank");
        let encoded = url
            .strip_prefix("https://wa.me/910000000001?text=")
            .unwrap();
        assert_eq!(decode(encoded), expected);
        assert_eq!(
            encoded,
            "Dear%20A%20%26%20B%2C%0AThis%20is%20a%20gentle%20reminder%20from%20*Synthetic%20%26%20Co*.%0AYour%20pending%20balance%20is%20*%E2%82%B9740*.%0AKindly%20clear%20at%20your%20convenience.%20Thank%20you!%20%F0%9F%99%8F"
        );
        println!(
            "{surface:?} visibility [valid, malformed, missing, advance] expected=[true, true, false, false] observed={observed:?}; valid=one open/no warning; malformed=warn/no open"
        );
    }
    assert!(visible(Surface::Ageing, "0000000001", -100, 25_000));
    assert!(!visible(Surface::Debtors, "0000000001", -100, 25_000));
    for ageing_due_paise in [49, 50] {
        let mut capture = Capture::default();
        assert!(!click_customer(
            &mut capture,
            records,
            Row {
                surface: Surface::Ageing,
                index: 0,
                ageing_due_paise
            }
        ));
        assert_eq!(capture, Capture::default());
    }
}
