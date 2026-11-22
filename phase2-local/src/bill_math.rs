use crate::Money;

pub(crate) fn checked_line_amount(unit_price: Money, quantity: u32) -> Option<Money> {
    unit_price
        .minor_units()
        .checked_mul(i64::from(quantity))
        .map(Money::from_minor_units)
}

pub(crate) fn round_balance(paise: i64) -> Option<i64> {
    (paise.div_euclid(100) + i64::from(paise.rem_euclid(100) >= 50)).checked_mul(100)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ExactGstError {
    UnsupportedRounding,
    Overflow,
}

pub(crate) fn exact_taxable(inclusive: Money, gst_basis_points: u32) -> Result<i64, ExactGstError> {
    const BASIS_POINTS: i64 = 10_000;
    let scaled = inclusive
        .minor_units()
        .checked_mul(BASIS_POINTS)
        .ok_or(ExactGstError::Overflow)?;
    exact_divide(scaled, BASIS_POINTS + i64::from(gst_basis_points))
}

pub(crate) const fn exact_intra_half(tax: i64) -> Result<i64, ExactGstError> {
    exact_divide(tax, 2)
}

const fn exact_divide(value: i64, divisor: i64) -> Result<i64, ExactGstError> {
    if value % divisor != 0 {
        return Err(ExactGstError::UnsupportedRounding);
    }
    Ok(value / divisor)
}
