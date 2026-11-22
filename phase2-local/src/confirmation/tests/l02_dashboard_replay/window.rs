use crate::BillDate;

#[derive(Clone, Copy, Debug)]
pub(super) enum Period {
    Today,
    Week,
    Month,
}

#[derive(Clone, Copy)]
pub(super) struct Window(pub(super) BillDate, pub(super) BillDate);

impl Window {
    pub(super) fn contains(self, date: BillDate) -> bool {
        self.0.parts() <= date.parts() && date.parts() <= self.1.parts()
    }

    pub(super) fn iso(self) -> [String; 2] {
        [self.0.to_string(), self.1.to_string()]
    }
}

fn days_before(today: BillDate, days: u8) -> BillDate {
    (0..days).fold(today, |date, _| {
        let (year, month, day) = date.parts();
        if day > 1 {
            BillDate::new(year, month, day - 1).unwrap()
        } else {
            let (year, month) = if month == 1 {
                (year - 1, 12)
            } else {
                (year, month - 1)
            };
            (28..=31)
                .rev()
                .find_map(|day| BillDate::new(year, month, day).ok())
                .unwrap()
        }
    })
}

// Legacy dashRange (HTML 2466-2496); date-only, with no wall clock or timezone.
pub(super) fn dashboard_windows(period: Period, today: BillDate) -> [Window; 2] {
    match period {
        Period::Today => {
            let yesterday = days_before(today, 1);
            [Window(today, today), Window(yesterday, yesterday)]
        }
        Period::Week => [
            Window(days_before(today, 6), today),
            Window(days_before(today, 13), days_before(today, 7)),
        ],
        Period::Month => {
            let (year, month, _) = today.parts();
            let first = BillDate::new(year, month, 1).unwrap();
            let previous_end = days_before(first, 1);
            let (year, month, _) = previous_end.parts();
            let previous_first = BillDate::new(year, month, 1).unwrap();
            [Window(first, today), Window(previous_first, previous_end)]
        }
    }
}
