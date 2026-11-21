//! Synthetic, Owner-scoped read model; it cannot post sales or authorize transactions.
mod date;
mod export;
mod storage;

use serde::{Deserialize, Serialize};

pub use storage::{Error, Store};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Records {
    pub reorder_default: u32,
    pub products: Vec<Product>,
    pub customers: Vec<Customer>,
    pub sales: Vec<Sale>,
    pub receipts: Vec<Receipt>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Product {
    pub item: String,
    pub company: String,
    pub pack: String,
    pub uom: String,
    pub packet_quantity: u32,
    pub mrp_minor: i64,
    pub buy_minor: i64,
    pub sell_minor: i64,
    pub purchased: u32,
    pub sold: u32,
    pub on_hand: i64,
    pub reorder: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Customer {
    pub name: String,
    pub mobile: String,
    pub address: String,
    pub balance_minor: i64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Sale {
    pub id: String,
    #[serde(with = "date")]
    pub date: crate::BillDate,
    pub number: u64,
    pub customer: String,
    pub mode: String,
    pub lines: Vec<Line>,
    pub total_minor: i64,
    pub paid_minor: i64,
    pub cancelled: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Line {
    pub item: String,
    pub pack: String,
    pub uom: String,
    pub packet_quantity: u32,
    pub kind: LineKind,
    pub rate_minor: i64,
    pub quantity: u32,
    pub amount_minor: i64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum LineKind {
    Item,
    Crate { components: Vec<Component> },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Component {
    pub item: String,
    pub quantity: u32,
}

impl Line {
    fn display_item(&self) -> String {
        match self.kind {
            LineKind::Item => self.item.clone(),
            LineKind::Crate { .. } => format!("Crate: {}", self.item),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    pub id: String,
    #[serde(with = "date")]
    pub date: crate::BillDate,
    pub mode: String,
    pub customer: String,
    pub amount_minor: i64,
}
