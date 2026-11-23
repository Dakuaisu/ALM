use super::{DashboardError, DashboardPeriod, DashboardWindow};
use crate::BillDate;

pub(in crate::confirmation) fn before(
    date: BillDate,
    days: u8,
) -> Result<BillDate, DashboardError> {
    (0..days).try_fold(date, |date, _| {
        let (year, month, day) = date.parts();
        if day > 1 {
            return BillDate::new(year, month, day - 1).map_err(|_| DashboardError::DateOutOfRange);
        }
        let (year, month) = if month == 1 {
            (
                year.checked_sub(1).ok_or(DashboardError::DateOutOfRange)?,
                12,
            )
        } else {
            (year, month - 1)
        };
        (28..=31)
            .rev()
            .find_map(|day| BillDate::new(year, month, day).ok())
            .ok_or(DashboardError::DateOutOfRange)
    })
}

pub(super) fn windows(
    today: BillDate,
    period: DashboardPeriod,
) -> Result<[DashboardWindow; 2], DashboardError> {
    let window = |start, end| DashboardWindow { start, end };
    match period {
        DashboardPeriod::Today => {
            let yesterday = before(today, 1)?;
            Ok([window(today, today), window(yesterday, yesterday)])
        }
        DashboardPeriod::Week => Ok([
            window(before(today, 6)?, today),
            window(before(today, 13)?, before(today, 7)?),
        ]),
        DashboardPeriod::Month => {
            let (year, month, _) = today.parts();
            let first =
                BillDate::new(year, month, 1).map_err(|_| DashboardError::DateOutOfRange)?;
            let end = before(first, 1)?;
            let (year, month, _) = end.parts();
            let start =
                BillDate::new(year, month, 1).map_err(|_| DashboardError::DateOutOfRange)?;
            Ok([window(first, today), window(start, end)])
        }
    }
}
