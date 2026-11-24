use std::{fmt, str::FromStr};

/// A Gregorian calendar date, without a time or time zone.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "demo-fixture", derive(serde::Serialize, serde::Deserialize))]
pub struct BillDate {
    year: u16,
    month: u8,
    day: u8,
}

/// Not a valid calendar date in the supported four-digit year range.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidBillDate;

impl BillDate {
    /// Constructs a calendar date in years 0001 through 9999.
    ///
    /// # Errors
    /// Returns [`InvalidBillDate`] for out-of-range years, months, or days.
    pub const fn new(year: u16, month: u8, day: u8) -> Result<Self, InvalidBillDate> {
        let leap =
            year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
        let last_day = match month {
            4 | 6 | 9 | 11 => 30,
            2 if leap => 29,
            2 => 28,
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            _ => return Err(InvalidBillDate),
        };
        if year == 0 || year > 9999 || day == 0 || day > last_day {
            return Err(InvalidBillDate);
        }
        Ok(Self { year, month, day })
    }

    pub const fn parts(self) -> (u16, u8, u8) {
        (self.year, self.month, self.day)
    }

    /// `dDisp(iso)`: "05-Apr-2025". Forms keep the ISO text from `Display`.
    pub fn display(self) -> String {
        format!(
            "{:02}-{}-{:04}",
            self.day,
            MONTHS[usize::from(self.month - 1)],
            self.year
        )
    }

    /// The first day of this date's month (`monthKey(today) + "-01"`).
    pub const fn first_of_month(self) -> Self {
        Self { day: 1, ..self }
    }
}

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// `monthName(mk)`: `toLocaleDateString("en-IN", {month: "short", year: "2-digit"})`, e.g.
/// "Apr 25". Returns `None` for a month outside 1–12.
pub fn month_label(year: u16, month: u8) -> Option<String> {
    let name = MONTHS.get(usize::from(month).checked_sub(1)?)?;
    Some(format!("{name} {:02}", year % 100))
}

#[cfg(test)]
mod display_tests {
    use super::{BillDate, month_label};

    #[test]
    fn display_helpers_follow_ddisp_and_month_name() {
        let date = BillDate::new(2025, 4, 5).unwrap();
        assert_eq!(date.display(), "05-Apr-2025");
        assert_eq!(
            BillDate::new(2026, 12, 31).unwrap().display(),
            "31-Dec-2026"
        );
        assert_eq!(date.first_of_month(), BillDate::new(2025, 4, 1).unwrap());
        assert_eq!(month_label(2025, 4).as_deref(), Some("Apr 25"));
        assert_eq!(month_label(2009, 1).as_deref(), Some("Jan 09"));
        assert_eq!(month_label(2025, 13), None);
        assert_eq!(date.to_string(), "2025-04-05");
    }
}

impl fmt::Display for BillDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

impl FromStr for BillDate {
    type Err = InvalidBillDate;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let [y1, y2, y3, y4, b'-', m1, m2, b'-', d1, d2] = *value.as_bytes() else {
            return Err(InvalidBillDate);
        };
        let digit = |byte: u8| {
            byte.is_ascii_digit()
                .then(|| byte - b'0')
                .ok_or(InvalidBillDate)
        };
        let year = u16::from(digit(y1)?) * 1000
            + u16::from(digit(y2)?) * 100
            + u16::from(digit(y3)?) * 10
            + u16::from(digit(y4)?);
        Self::new(
            year,
            digit(m1)? * 10 + digit(m2)?,
            digit(d1)? * 10 + digit(d2)?,
        )
    }
}

impl fmt::Display for InvalidBillDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("expected a valid calendar date in YYYY-MM-DD form")
    }
}

impl std::error::Error for InvalidBillDate {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l03_03_bill_date_requires_a_real_canonical_calendar_date() {
        for valid in ["0001-01-01", "2000-02-29", "2024-02-29", "9999-12-31"] {
            assert_eq!(valid.parse::<BillDate>().unwrap().to_string(), valid);
        }
        for invalid in [
            "0000-01-01",
            "1900-02-29",
            "2025-02-29",
            "2024-04-31",
            "2024-00-01",
            "2024-01-00",
            "2024-2-09",
            "2024-+2-09",
            "2024-02-29T00:00",
            "",
        ] {
            assert_eq!(
                invalid.parse::<BillDate>(),
                Err(InvalidBillDate),
                "{invalid}"
            );
        }
        assert_eq!(BillDate::new(10_000, 1, 1), Err(InvalidBillDate));
    }
}
