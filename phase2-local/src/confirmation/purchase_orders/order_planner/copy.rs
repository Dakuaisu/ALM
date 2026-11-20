use std::fmt::Write;

use super::{OrderPlanner, OrderPlannerError, owner};
use crate::confirmation::ConfirmationFixture;
use crate::confirmation::estimate::display_date;
use crate::confirmation::image_handoff::group_indian_digits;
use crate::{BillDate, Money, Role};

#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CopyPlanRejected;

type CopyPlanWriter<'a> = dyn FnMut(&str) -> Result<(), CopyPlanRejected> + 'a;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CopyPlanFeedback {
    Copied,
    Rejected,
    Unavailable,
}

impl CopyPlanFeedback {
    pub const fn message(self) -> &'static str {
        match self {
            Self::Copied => "Order plan copied ✓",
            Self::Rejected => "Copy failed",
            Self::Unavailable => "Copy not available in this browser",
        }
    }
}

impl ConfirmationFixture {
    /// Sends the frozen Copy Plan text once to the supplied writer, without changing the plan.
    ///
    /// # Errors
    /// Denies Managers before reading prices or invoking the writer. Reports amount overflow
    /// without invoking the writer if the exact total or a formatted magnitude is out of range.
    pub fn copy_order_plan(
        &self,
        role: Role,
        plan: &OrderPlanner,
        today: BillDate,
        writer: Option<&mut CopyPlanWriter<'_>>,
    ) -> Result<CopyPlanFeedback, OrderPlannerError> {
        owner(role).map_err(|_| OrderPlannerError::AccessDenied)?;
        let Some(writer) = writer else {
            return Ok(CopyPlanFeedback::Unavailable);
        };

        let mut text = format!("*ALM Order Plan* ({})\n\n", display_date(today));
        let mut append_line = |item: &str,
                               quantity: f64,
                               quote: Option<(&str, f64)>|
         -> Result<(), OrderPlannerError> {
            let _ = write!(text, "{item} — {} pkt", fmt_number(quantity)?);
            if let Some((supplier, rate)) = quote {
                let _ = write!(text, " ({supplier} ₹{})", fmt_number(rate)?);
            }
            text.push('\n');
            Ok(())
        };

        if plan.cart().is_empty() {
            let fresh = self.open_order_planner(role)?;
            for row in fresh.rows(true) {
                let quote = match &row.best_supplier {
                    Some(supplier) => Some((supplier.name.as_str(), rupee_number(row.unit_rate)?)),
                    None => None,
                };
                append_line(&row.item, packet_number(row.suggested_quantity), quote)?;
            }
        } else {
            plan.cart_total()?;
            let mut lines: Vec<_> = plan.cart().iter().collect();
            // Object.keys puts array indices first; u32::MAX shares the stable ordinary-key rank.
            lines.sort_by_key(|line| match line.item.as_bytes() {
                [b'0'] | [b'1'..=b'9', ..] => line.item.parse::<u32>().unwrap_or(u32::MAX),
                _ => u32::MAX,
            });
            let mut total = 0.0;
            for line in lines {
                let quantity = packet_number(line.quantity);
                let rate = rupee_number(line.unit_rate)?;
                let quote = line
                    .best_supplier
                    .as_ref()
                    .map(|supplier| (supplier.name.as_str(), rate));
                append_line(&line.item, quantity, quote)?;
                total += quantity * rate;
            }
            let _ = write!(text, "\nTotal ₹{}", fmt_number(js_round(total))?);
        }

        Ok(match writer(&text) {
            Ok(()) => CopyPlanFeedback::Copied,
            Err(CopyPlanRejected) => CopyPlanFeedback::Rejected,
        })
    }
}

const fn packet_number(quantity: i64) -> f64 {
    // Legacy Number rounds large whole-packet counts to binary64 for display.
    quantity as f64
}

fn rupee_number(rate: Money) -> Result<f64, OrderPlannerError> {
    // Parse decimal rupees directly to avoid rounding large paise before dividing by 100.
    format!("{}e-2", rate.minor_units())
        .parse()
        .map_err(|_| OrderPlannerError::AmountOverflow)
}

fn js_round(value: f64) -> f64 {
    // Math.round sends half ties toward +infinity; adding 0.5 before flooring can lose a bit.
    let whole = value.trunc();
    let fraction = value.fract();
    if fraction >= 0.5 {
        whole + 1.0
    } else if fraction < -0.5 {
        whole - 1.0
    } else {
        whole
    }
}

fn fmt_number(value: f64) -> Result<String, OrderPlannerError> {
    // The frozen fmtN normalizes signed zero before rounding to cents. Rescaling can
    // introduce a fraction even for a whole input near the binary64 precision limit.
    let displayed = js_round((value + 0.0) * 100.0) / 100.0;
    let decimal = displayed.to_string();
    let decimal = match decimal.split_once('.') {
        Some((_, fraction)) => format!("{displayed:.precision$}", precision = fraction.len()),
        None => decimal,
    };
    let (sign, magnitude) = match decimal.strip_prefix('-') {
        Some(magnitude) => ("-", magnitude),
        None => ("", decimal.as_str()),
    };
    let (whole, fraction) = magnitude.split_once('.').unwrap_or((magnitude, ""));
    let whole = whole
        .parse()
        .map_err(|_| OrderPlannerError::AmountOverflow)?;
    let mut text = format!("{sign}{}", group_indian_digits(whole));
    if !fraction.is_empty() {
        text.push('.');
        text.push_str(fraction);
    }
    Ok(text)
}
