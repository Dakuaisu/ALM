use std::{fmt, str::FromStr};

/// A Gregorian calendar date, without a time or time zone.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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

    pub(crate) const fn parts(self) -> (u16, u8, u8) {
        (self.year, self.month, self.day)
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
