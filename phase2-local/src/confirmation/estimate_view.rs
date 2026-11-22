use super::{BalanceMode, PreviousBalance};
use crate::{BillDate, Money};

pub struct EstimateRow<'a> {
    pub label: &'a str,
    pub quantity: u32,
    pub price: Money,
    pub amount: Money,
}

pub struct EstimateDocument<'a> {
    pub invoice: u64,
    pub date: BillDate,
    pub customer: &'a str,
    pub lines: Vec<EstimateRow<'a>>,
    pub items_quantity: u32,
    pub items_total: Money,
    pub loading: Money,
    pub total: Money,
    pub paid: Money,
    pub cancelled: bool,
    pub balance_mode: BalanceMode,
    pub balance: Money,
    pub previous_balance: Option<PreviousBalance>,
    pub today: BillDate,
}
