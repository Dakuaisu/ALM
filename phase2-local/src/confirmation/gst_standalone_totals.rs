use super::{GstBillError, GstBillTotals, GstJurisdiction};
use crate::Money;

pub(super) fn totals(
    lines: impl IntoIterator<Item = Result<(i64, u32), GstBillError>>,
    jurisdiction: GstJurisdiction,
) -> Result<GstBillTotals, GstBillError> {
    let mut inclusive = 0_i64;
    let mut taxable = 0_i64;
    for line in lines {
        let (amount, rate) = line?;
        let numerator = amount.checked_mul(10_000).ok_or(GstBillError::Overflow)?;
        let denominator = 10_000_i64 + i64::from(rate);
        if numerator % denominator != 0 {
            return Err(GstBillError::UnsupportedRounding);
        }
        taxable = taxable
            .checked_add(numerator / denominator)
            .ok_or(GstBillError::Overflow)?;
        inclusive = inclusive
            .checked_add(amount)
            .ok_or(GstBillError::Overflow)?;
    }
    if inclusive % 100 != 0 {
        return Err(GstBillError::UnsupportedRounding);
    }
    let tax = inclusive - taxable;
    let (cgst, sgst, igst) = match jurisdiction {
        GstJurisdiction::Intra => {
            if tax % 2 != 0 {
                return Err(GstBillError::UnsupportedRounding);
            }
            (tax / 2, tax / 2, 0)
        }
        GstJurisdiction::Inter => (0, 0, tax),
    };
    Ok(GstBillTotals {
        taxable: Money::from_minor_units(taxable),
        cgst: Money::from_minor_units(cgst),
        sgst: Money::from_minor_units(sgst),
        igst: Money::from_minor_units(igst),
        total: Money::from_minor_units(inclusive),
    })
}
