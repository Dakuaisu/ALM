use std::borrow::Cow;

use crate::{BillDate, Money};

pub(super) fn date(text: &str) -> Option<BillDate> {
    text.parse().ok().or_else(|| {
        let mut parts = text.split(['-', '/', '.', ' ']);
        let (day, month, year) = (parts.next()?, parts.next()?, parts.next()?);
        if parts.next().is_some() {
            return None;
        }
        let day = decimal(day, 1..=2)?;
        let month = if month.bytes().all(|byte| byte.is_ascii_alphabetic()) {
            let prefix = month.get(..3)?;
            let index = [
                "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
            ]
            .iter()
            .position(|name| name.eq_ignore_ascii_case(prefix))?;
            u8::try_from(index + 1).ok()?
        } else if text.contains(' ') {
            return None;
        } else {
            decimal(month, 1..=2)?
        };
        let year = match year.len() {
            2 => 2000 + decimal::<u16>(year, 2..=2)?,
            4 => decimal(year, 4..=4)?,
            _ => return None,
        };
        BillDate::new(year, month, day).ok()
    })
}

fn decimal<T: std::str::FromStr>(text: &str, width: std::ops::RangeInclusive<usize>) -> Option<T> {
    if !width.contains(&text.len()) || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

#[derive(Debug)]
pub(super) struct InvalidMoney;

pub(super) fn money(text: &str) -> Result<Option<Money>, InvalidMoney> {
    if text.is_empty() {
        return Ok(None);
    }
    let formatting = |ch: char| matches!(ch, '₹' | ',') || ch.is_whitespace();
    let text = if text.contains(formatting) {
        Cow::Owned(text.replace(formatting, ""))
    } else {
        Cow::Borrowed(text)
    };
    let (sign, digits) = text
        .strip_prefix('-')
        .map_or((1, text.as_ref()), |rest| (-1, rest));
    let (whole, fraction) = digits.split_once('.').unwrap_or((digits, ""));
    if whole.is_empty()
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || !fraction.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(InvalidMoney);
    }
    let whole = i128::from(whole.parse::<u64>().map_err(|_| InvalidMoney)?);
    let fraction = match fraction.as_bytes() {
        [] => 0,
        [tenths] => i128::from(tenths - b'0') * 10,
        [tenths, hundredths] => i128::from(tenths - b'0') * 10 + i128::from(hundredths - b'0'),
        _ => return Err(InvalidMoney),
    };
    let minor = i64::try_from(sign * (whole * 100 + fraction)).map_err(|_| InvalidMoney)?;
    Ok(Some(Money::from_minor_units(minor)))
}
