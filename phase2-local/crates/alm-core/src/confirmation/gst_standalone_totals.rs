use super::{GstBillError, GstBillLine, GstBillTotals, GstHsnSummary, GstJurisdiction};
use crate::Money;

const BASIS: i64 = 10_000;
const OVERFLOW: GstBillError = GstBillError::Overflow;

#[derive(Clone, Copy)]
struct RawTaxable {
    numerator: i128,
    denominator: i128,
}

impl RawTaxable {
    fn from_inclusive(amount: i64, rate: u32) -> Result<Self, GstBillError> {
        if amount < 0 {
            return Err(GstBillError::InvalidAmount);
        }
        let numerator = amount.checked_mul(BASIS).ok_or(OVERFLOW)?;
        Ok(Self::reduced(
            i128::from(numerator),
            i128::from(BASIS + i64::from(rate)),
        ))
    }

    const fn reduced(numerator: i128, denominator: i128) -> Self {
        let factor = gcd(numerator, denominator);
        Self {
            numerator: numerator / factor,
            denominator: denominator / factor,
        }
    }

    fn add(self, rhs: Self) -> Result<Self, GstBillError> {
        let common = gcd(self.denominator, rhs.denominator);
        let lhs_factor = rhs.denominator / common;
        let rhs_factor = self.denominator / common;
        let numerator = self
            .numerator
            .checked_mul(lhs_factor)
            .and_then(|lhs| lhs.checked_add(rhs.numerator.checked_mul(rhs_factor)?))
            .ok_or(OVERFLOW)?;
        let denominator = self.denominator.checked_mul(lhs_factor).ok_or(OVERFLOW)?;
        Ok(Self::reduced(numerator, denominator))
    }

    fn tax(self, inclusive: i64) -> Result<Self, GstBillError> {
        let numerator = i128::from(inclusive)
            .checked_mul(self.denominator)
            .and_then(|amount| amount.checked_sub(self.numerator))
            .ok_or(OVERFLOW)?;
        Ok(Self::reduced(numerator, self.denominator))
    }

    fn scaled(self, numerator: i128, denominator: i128) -> Result<Self, GstBillError> {
        Ok(Self::reduced(
            self.numerator.checked_mul(numerator).ok_or(OVERFLOW)?,
            self.denominator.checked_mul(denominator).ok_or(OVERFLOW)?,
        ))
    }

    fn rounded(self, parts: i128) -> Result<Money, GstBillError> {
        let denominator = self.denominator.checked_mul(parts).ok_or(OVERFLOW)?;
        let remainder = self.numerator % denominator;
        let rounded =
            self.numerator / denominator + i128::from(remainder >= denominator - remainder);
        i64::try_from(rounded)
            .map(Money::from_minor_units)
            .map_err(|_| OVERFLOW)
    }
}

const fn gcd(mut a: i128, mut b: i128) -> i128 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

impl GstBillLine {
    pub fn taxable_amount(&self) -> Result<Money, GstBillError> {
        RawTaxable::from_inclusive(self.amount.minor_units(), self.gst_basis_points)?.rounded(1)
    }

    pub fn taxable_rate(&self) -> Result<Money, GstBillError> {
        RawTaxable::from_inclusive(self.inclusive_rate.minor_units(), self.gst_basis_points)?
            .rounded(1)
    }
}

pub(super) fn totals(
    lines: impl IntoIterator<Item = Result<(i64, u32), GstBillError>>,
    jurisdiction: GstJurisdiction,
) -> Result<GstBillTotals, GstBillError> {
    let mut inclusive = 0_i64;
    let mut taxable = RawTaxable {
        numerator: 0,
        denominator: 1,
    };
    for line in lines {
        let (amount, rate) = line?;
        taxable = taxable.add(RawTaxable::from_inclusive(amount, rate)?)?;
        inclusive = inclusive.checked_add(amount).ok_or(OVERFLOW)?;
    }
    if inclusive % 100 != 0 {
        return Err(GstBillError::UnsupportedRounding);
    }
    // Frozen GST form and invoice round each displayed amount after summing raw contributions.
    let raw_tax = taxable.tax(inclusive)?;
    let tax = raw_tax.rounded(1)?;
    let zero = Money::from_minor_units(0);
    let (cgst, sgst, igst) = match jurisdiction {
        GstJurisdiction::Intra => {
            let half = raw_tax.rounded(2)?;
            (half, half, zero)
        }
        GstJurisdiction::Inter => (zero, zero, tax),
    };
    Ok(GstBillTotals {
        taxable: taxable.rounded(1)?,
        tax,
        cgst,
        sgst,
        igst,
        total: Money::from_minor_units(inclusive),
    })
}

/// `hsnMap`: taxable value summed per HSN and rate, then each displayed amount rounded once.
pub(super) fn hsn_summary<'a>(
    lines: impl IntoIterator<Item = (&'a str, i64, u32)>,
    jurisdiction: GstJurisdiction,
) -> Result<Vec<GstHsnSummary>, GstBillError> {
    let mut groups: Vec<(&str, u32, RawTaxable)> = Vec::new();
    for (hsn, amount, rate) in lines {
        let raw = RawTaxable::from_inclusive(amount, rate)?;
        match groups
            .iter_mut()
            .find(|group| group.0 == hsn && group.1 == rate)
        {
            Some(group) => group.2 = group.2.add(raw)?,
            None => groups.push((hsn, rate, raw)),
        }
    }
    let zero = Money::from_minor_units(0);
    groups
        .into_iter()
        .map(|(hsn, rate, raw)| {
            let rate_parts = i128::from(rate);
            let total_tax = raw.scaled(rate_parts, i128::from(BASIS))?.rounded(1)?;
            let half_tax = raw.scaled(rate_parts, i128::from(BASIS) * 2)?.rounded(1)?;
            let (half_tax, integrated_tax) = match jurisdiction {
                GstJurisdiction::Intra => (half_tax, zero),
                GstJurisdiction::Inter => (zero, total_tax),
            };
            Ok(GstHsnSummary {
                hsn: hsn.to_owned(),
                gst_basis_points: rate,
                taxable: raw.rounded(1)?,
                half_tax,
                integrated_tax,
                total_tax,
            })
        })
        .collect()
}
