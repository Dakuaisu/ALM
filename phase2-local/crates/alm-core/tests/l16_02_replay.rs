// Source-model replay only: bounded to the supplied 2024 clock windows, not a TZ database.
const DAY: i64 = 86_400;
const NOV_1: i64 = 31 * DAY; // Seconds from 2024-10-01T00:00:00Z.
const NY_FALLBACK: i64 = NOV_1 + 2 * DAY + 6 * 3600;

#[derive(Clone, Copy, Debug)]
enum Zone {
    Kolkata,
    NewYork,
}

impl Zone {
    const fn offset(self, utc: i64) -> i64 {
        match self {
            Self::Kolkata => 5 * 3600 + 30 * 60,
            Self::NewYork if utc < NY_FALLBACK => -4 * 3600,
            Self::NewYork => -5 * 3600,
        }
    }
}

// Only October and November 2024 are needed by these fixtures.
fn iso_date(seconds: i64) -> String {
    let days = seconds.div_euclid(DAY);
    if days < 31 {
        format!("2024-10-{:02}", days + 1)
    } else {
        format!("2024-11-{:02}", days - 30)
    }
}

fn iso_instant(seconds: i64) -> String {
    let time = seconds.rem_euclid(DAY);
    format!(
        "{}T{:02}:{:02}:{:02}Z",
        iso_date(seconds),
        time / 3600,
        time % 3600 / 60,
        time % 60
    )
}

fn cutoff(clock: i64, zone: Zone) -> (String, String, String) {
    // todayISO reads the clock in local time. A date-only JS parse is UTC midnight.
    let today_day = (clock + zone.offset(clock)).div_euclid(DAY);
    let parsed_utc = today_day * DAY;
    let local_before = parsed_utc + zone.offset(parsed_utc);
    let shifted_local = local_before - 30 * DAY;
    // Both destination local times are unambiguous in the modeled October window.
    let october_offset = zone.offset(shifted_local);
    let shifted_utc = shifted_local - october_offset;
    (
        iso_date(parsed_utc),
        iso_instant(shifted_utc),
        iso_date(shifted_utc),
    )
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ProductId(u8);

#[derive(Clone, Copy)]
struct Product {
    id: ProductId,
    on_hand: i32,
}

#[derive(Clone, Copy)]
struct Sale {
    product: ProductId,
    date: Option<&'static str>,
    created: &'static str,
    cancelled: bool,
}

fn select(products: &[Product], sales: &[Sale], cutoff: &str) -> Vec<ProductId> {
    products
        .iter()
        .filter_map(|product| {
            let last = sales
                .iter()
                .filter(|sale| !sale.cancelled && sale.product == product.id)
                .filter_map(|sale| sale.date)
                .max();
            let selected = product.on_hand > 0 && last.is_none_or(|date| date < cutoff);
            println!(
                "candidate={:?} on_hand={} last={last:?} selected={selected}",
                product.id, product.on_hand
            );
            selected.then_some(product.id)
        })
        .collect()
}

#[test]
fn l16_02_local_clock_shift_and_stock_selection() {
    // Given: distinct products and document dates, independent of creation order.
    let products = [
        Product {
            id: ProductId(1),
            on_hand: 1,
        },
        Product {
            id: ProductId(2),
            on_hand: 1,
        },
        Product {
            id: ProductId(3),
            on_hand: 1,
        },
        Product {
            id: ProductId(4),
            on_hand: 1,
        },
        Product {
            id: ProductId(5),
            on_hand: 1,
        },
        Product {
            id: ProductId(6),
            on_hand: 0,
        },
        Product {
            id: ProductId(7),
            on_hand: -1,
        },
        Product {
            id: ProductId(8),
            on_hand: 1,
        },
        Product {
            id: ProductId(9),
            on_hand: 1,
        },
    ];
    let sales = [
        Sale {
            product: ProductId(1),
            date: Some("2024-10-03"),
            created: "2024-10-03",
            cancelled: false,
        },
        Sale {
            product: ProductId(2),
            date: Some("2024-10-04"),
            created: "2024-10-04",
            cancelled: false,
        },
        Sale {
            product: ProductId(2),
            date: Some("2024-10-01"),
            created: "2024-11-02",
            cancelled: false,
        },
        Sale {
            product: ProductId(3),
            date: Some("2024-10-05"),
            created: "2024-10-05",
            cancelled: false,
        },
        Sale {
            product: ProductId(9),
            date: Some("2024-10-06"),
            created: "2024-10-06",
            cancelled: false,
        },
        Sale {
            product: ProductId(5),
            date: Some("2024-11-01"),
            created: "2024-11-01",
            cancelled: true,
        },
        Sale {
            product: ProductId(8),
            date: None,
            created: "2024-11-02",
            cancelled: false,
        },
    ];
    for sale in sales {
        println!(
            "sale product={:?} date={:?} created={} cancelled={}",
            sale.product, sale.date, sale.created, sale.cancelled
        );
    }
    let cases = [
        (
            Zone::Kolkata,
            NOV_1 + 2 * DAY + 18 * 3600 + 29 * 60 + 59,
            "2024-11-03",
            "2024-10-04T00:00:00Z",
            "2024-10-04",
            vec![ProductId(1), ProductId(4), ProductId(5), ProductId(8)],
        ),
        (
            Zone::Kolkata,
            NOV_1 + 2 * DAY + 18 * 3600 + 30 * 60,
            "2024-11-04",
            "2024-10-05T00:00:00Z",
            "2024-10-05",
            vec![
                ProductId(1),
                ProductId(2),
                ProductId(4),
                ProductId(5),
                ProductId(8),
            ],
        ),
        (
            Zone::NewYork,
            NY_FALLBACK - 1,
            "2024-11-03",
            "2024-10-04T00:00:00Z",
            "2024-10-04",
            vec![ProductId(1), ProductId(4), ProductId(5), ProductId(8)],
        ),
        (
            Zone::NewYork,
            NY_FALLBACK,
            "2024-11-03",
            "2024-10-04T00:00:00Z",
            "2024-10-04",
            vec![ProductId(1), ProductId(4), ProductId(5), ProductId(8)],
        ),
        (
            Zone::NewYork,
            NOV_1 + 3 * DAY + 12 * 3600,
            "2024-11-04",
            "2024-10-04T23:00:00Z",
            "2024-10-04",
            vec![ProductId(1), ProductId(4), ProductId(5), ProductId(8)],
        ),
    ];
    for (zone, clock, expected_today, expected_instant, expected_cutoff, expected_ids) in cases {
        // When: the frozen date sequence and stock filter run on each source ordering.
        let (today, shifted, date) = cutoff(clock, zone);
        for reversed in [false, true] {
            let ordered: Vec<_> = if reversed {
                sales.iter().rev().copied().collect()
            } else {
                sales.to_vec()
            };
            let ids = select(&products, &ordered, &date);
            println!(
                "clock={} zone={zone:?} today={today} shifted={shifted} cutoff={date} reversed={reversed} IDs={ids:?}",
                iso_instant(clock)
            );
            // Then: compare independent literal dates and IDs, not derived expectations.
            assert_eq!(today, expected_today);
            assert_eq!(shifted, expected_instant);
            assert_eq!(date, expected_cutoff);
            assert_eq!(ids, expected_ids);
        }
    }
    // A naive fixed UTC subtraction would yield October 5, unlike local setDate.
    assert_eq!(iso_date((34 * DAY) - 30 * DAY), "2024-10-05");
}
