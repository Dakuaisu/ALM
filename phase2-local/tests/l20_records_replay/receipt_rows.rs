use alm_phase2_local::BillDate;
use serde::{Deserialize, Serialize};
use serde_json::Number;

use super::model::Tick;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(try_from = "Number", into = "Number")]
pub(super) struct Rupees(i64);

impl Rupees {
    pub const fn from_minor(value: i64) -> Self {
        Self(value)
    }

    pub const fn minor(self) -> i64 {
        self.0
    }

    pub const fn whole(value: i64) -> Self {
        Self::from_minor(value * 100)
    }
}

impl TryFrom<Number> for Rupees {
    type Error = &'static str;

    fn try_from(number: Number) -> Result<Self, Self::Error> {
        let text = number.to_string();
        let (units, fraction) = text.split_once('.').unwrap_or((&text, ""));
        let error = "fixture amount must fit integer paise with at most two decimals";
        let whole = units
            .parse::<i64>()
            .map_err(|_| error)?
            .checked_mul(100)
            .ok_or(error)?;
        let cents = match fraction.len() {
            0 => 0,
            1 => fraction.parse::<i64>().map_err(|_| error)? * 10,
            2 => fraction.parse::<i64>().map_err(|_| error)?,
            _ => return Err(error),
        };
        let signed = if text.starts_with('-') { -cents } else { cents };
        whole.checked_add(signed).map(Self).ok_or(error)
    }
}

impl From<Rupees> for Number {
    fn from(value: Rupees) -> Self {
        if value.0 % 100 == 0 {
            return Self::from(value.0 / 100);
        }
        let sign = if value.0 < 0 { "-" } else { "" };
        let magnitude = value.0.unsigned_abs();
        format!("{sign}{}.{:02}", magnitude / 100, magnitude % 100)
            .parse()
            .expect("fixture paise serialize to a finite JSON number")
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(try_from = "String", into = "String")]
pub(super) struct Date(pub BillDate);

impl TryFrom<String> for Date {
    type Error = alm_phase2_local::InvalidBillDate;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse().map(Self)
    }
}

impl From<Date> for String {
    fn from(value: Date) -> Self {
        value.0.to_string()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ProductCosts {
    pub buy_rate: Rupees,
    pub avg_cost: Rupees,
    pub sell_rate: Rupees,
    pub margin: Rupees,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PurchaseOrder {
    pub no: u32,
    pub date: Date,
    pub supplier: String,
    pub sup_id: Option<String>,
    pub note: String,
    pub created: u64,
    pub cancelled: bool,
    pub items: Vec<PurchaseOrderLine>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct PurchaseOrderLine {
    pub item: String,
    pub pid: String,
    pub rate: Rupees,
    pub ordered: u32,
    pub received: u32,
    pub cancelled: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub receipts: Vec<Receipt>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct Receipt {
    pub date: Date,
    pub qty: u32,
    pub rate: Rupees,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct GoodsLog {
    pub ts: u64,
    pub date: Date,
    pub device: String,
    #[serde(rename = "type")]
    pub kind: ActionKind,
    pub msg: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum ActionKind {
    Grn,
}

pub(super) struct ReceiveInput<'a> {
    pub receipt: Receipt,
    pub tick: Tick<'a>,
    pub log_id: &'a str,
    pub today: Date,
}
