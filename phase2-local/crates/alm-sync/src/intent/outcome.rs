//! The canonical text of outcomes.
use alm_core::Money;

use super::{Outcome, SaleReceipt};

impl Outcome {
    pub fn canonical(&self) -> String {
        match self {
            Self::Sale(r) => format!(
                "sale draft={} invoice={} quantity={} total={}",
                r.draft,
                r.invoice,
                r.quantity,
                r.total.minor_units()
            ),
            Self::Done => "done".into(),
            Self::GstNumber(n) => format!("gst number={n}"),
            Self::Created(id) => format!("created id={id}"),
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        let mut words = text.split(' ');
        let kind = words.next()?;
        let fields: Vec<(&str, &str)> = words.map(|w| w.split_once('=')).collect::<Option<_>>()?;
        let num = |key: &str| {
            fields
                .iter()
                .find(|(k, _)| *k == key)
                .and_then(|(_, v)| v.parse::<i64>().ok())
        };
        let outcome = match kind {
            "sale" => Self::Sale(SaleReceipt {
                draft: u64::try_from(num("draft")?).ok()?,
                invoice: u64::try_from(num("invoice")?).ok()?,
                quantity: u32::try_from(num("quantity")?).ok()?,
                total: Money::from_minor_units(num("total")?),
            }),
            "done" => Self::Done,
            "gst" => Self::GstNumber(u64::try_from(num("number")?).ok()?),
            "created" => Self::Created(u64::try_from(num("id")?).ok()?),
            _ => return None,
        };
        (outcome.canonical() == text).then_some(outcome)
    }
}
