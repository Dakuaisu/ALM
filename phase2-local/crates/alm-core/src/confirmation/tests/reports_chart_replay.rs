//! Legacy case L16-06.
use super::{date, monthly_reports_fixture};
use crate::confirmation::ConfirmationFixture;
use crate::confirmation::manual_cashbook::ManualCashBook;
use crate::{BillDate, DraftId, Money, Role};

type MonthKey = (u16, u8);
type MonthValues = (MonthKey, [i64; 2]);

const APRIL_BEFORE: [MonthValues; 6] = [
    ((2024, 11), [15_025, 10_025]),
    ((2024, 12), [0, 0]),
    ((2025, 1), [0, 0]),
    ((2025, 2), [1_049, 449]),
    ((2025, 3), [6_145, 2_445]),
    ((2025, 4), [15_025, 10_025]),
];
const APRIL_DELAYED: [MonthValues; 6] = [
    ((2024, 11), [15_025, 10_025]),
    ((2024, 12), [0, 0]),
    ((2025, 1), [0, 0]),
    ((2025, 2), [1_049, 449]),
    ((2025, 3), [6_145, 2_445]),
    ((2025, 4), [0, 0]),
];
const MAY_AFTER: [MonthValues; 6] = [
    ((2024, 12), [0, 0]),
    ((2025, 1), [0, 0]),
    ((2025, 2), [1_049, 449]),
    ((2025, 3), [6_145, 2_445]),
    ((2025, 4), [0, 0]),
    ((2025, 5), [0, 0]),
];
const DELAYED_CHANGES: [MonthValues; 1] = [((2025, 4), [-15_025, -10_025])];

fn schedule_chart(
    local_render_date: BillDate,
) -> impl Fn(&ConfirmationFixture) -> [MonthValues; 6] {
    // ALM_Business_Manager.html:8728,8961–69 capture local months at render, not at callback time.
    let (year, month, _) = local_render_date.parts();
    let mut key = (year, month);
    let mut months = [key; 6];
    for slot in months.iter_mut().rev() {
        *slot = key;
        key = match key {
            (year, 1) => (year - 1, 12),
            (year, month) => (year, month - 1),
        };
    }

    // Lines 8947–58 resample live, active bills; line 3029 derives profit from item rates and costs.
    move |fixture| {
        months.map(|key| {
            let mut sales = 0;
            let mut gross = 0;
            for (id, (review, receipt, _)) in &fixture.current_sales {
                let (year, month, _) = review.draft().bill_date().unwrap().parts();
                if (year, month) != key || fixture.lifecycle[id].0 {
                    continue;
                }
                let lines = review.draft().lines();
                assert!(review.draft().crate_rows().is_empty());
                assert_eq!(lines.len(), review.unit_prices().len());
                let line_sales: i64 = lines
                    .iter()
                    .zip(review.unit_prices())
                    .map(|(line, rate)| i64::from(line.quantity().get()) * rate.minor_units())
                    .sum();
                let frozen_cost = fixture.catalog.frozen_cost(*id).unwrap().unwrap();
                sales += receipt.total.minor_units();
                gross += line_sales - frozen_cost;
            }
            (key, [sales, gross])
        })
    }
}

fn monthly_pnl(
    fixture: &ConfirmationFixture,
    book: &ManualCashBook,
    at: BillDate,
) -> Vec<MonthValues> {
    // Lines 8756–62 fill P&L synchronously. Compare Sales and Gross Profit before display rounding.
    fixture
        .monthly_profit_report(Role::Owner, book, at)
        .unwrap()
        .into_iter()
        .map(|row| {
            (
                (row.year, row.month),
                [row.revenue.minor_units(), row.gross.minor_units()],
            )
        })
        .collect()
}

fn assert_values(stage: &str, expected: &[MonthValues], actual: &[MonthValues]) {
    println!(
        "L16-06 {stage}: (year/month, [Sales, Gross Profit]) raw paise expected={expected:?} actual={actual:?}"
    );
    assert_eq!(actual, expected, "{stage}");
}

#[test]
fn l16_06_reports_chart_retains_render_months_and_reads_live_sales() {
    let seed = monthly_reports_fixture(date("2024-11-01"), Money::from_minor_units(100));
    let (mut fixture, book) = (seed.fixture, seed.book);
    let april_30 = date("2025-04-30");
    let may_1 = date("2025-05-01");

    let april_chart = schedule_chart(april_30);
    let emitted_pnl = monthly_pnl(&fixture, &book, april_30);
    assert_values("April stable P&L", &APRIL_BEFORE, &emitted_pnl);
    assert_values("April stable chart", &emitted_pnl, &april_chart(&fixture));

    fixture.cancel_sale(Role::Owner, DraftId::new(80)).unwrap();
    let delayed = april_chart(&fixture);
    assert_values("April chart callback on May 1", &APRIL_DELAYED, &delayed);
    let changes: Vec<MonthValues> = delayed
        .into_iter()
        .zip(emitted_pnl)
        .filter_map(|((month, [sales, gross]), (_, [old_sales, old_gross]))| {
            let delta = [sales - old_sales, gross - old_gross];
            (delta != [0, 0]).then_some((month, delta))
        })
        .collect();
    assert_values(
        "Delayed chart minus emitted P&L",
        &DELAYED_CHANGES,
        &changes,
    );

    let may_chart = schedule_chart(may_1);
    let may_pnl = monthly_pnl(&fixture, &book, may_1);
    assert_values("May stable P&L", &MAY_AFTER, &may_pnl);
    assert_values("May stable chart", &may_pnl, &may_chart(&fixture));
}
