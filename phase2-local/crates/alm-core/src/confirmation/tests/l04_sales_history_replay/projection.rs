//! Display adapter for the frozen L04 replay.
use crate::confirmation::{
    ConfirmationError, ConfirmationFixture, Receipt, SalesPeriod, SalesSource,
};
use crate::{BillDate, Role};

type Bounds = (Option<BillDate>, Option<BillDate>);
#[derive(Clone, Copy)]
pub(super) enum Period {
    Today,
    ThisMonth,
    All,
    Custom(Bounds),
}
impl Period {
    pub(super) fn bounds(self, today: BillDate) -> Bounds {
        match self {
            Self::Today => (Some(today), Some(today)),
            Self::ThisMonth => {
                let (y, m, _) = today.parts();
                (Some(BillDate::new(y, m, 1).unwrap()), None)
            }
            Self::All => (None, None),
            Self::Custom(bounds) => bounds,
        }
    }
}

pub(super) struct SaleRow {
    pub(super) bill: Receipt,
    pub(super) cancelled: bool,
    pub(super) balance_paise: i64,
    remainder: i64,
}
impl SaleRow {
    pub(super) const fn remainder_paise(&self) -> i64 {
        self.remainder
    }
}
pub(super) struct History {
    pub(super) rows: Vec<SaleRow>,
    packets: u32,
    tiles: [i64; 4],
    profit: Profit,
    collected: Collected,
}
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Profit {
    pub(super) bill_ids: Vec<u64>,
    pub(super) revenue_paise: i64,
    pub(super) cost_paise: i64,
    profit: i64,
}
impl Profit {
    pub(super) const fn profit_paise(&self) -> i64 {
        self.profit
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Source {
    Bill(u64),
    Receipt(usize),
}
#[derive(Debug)]
pub(super) struct Collected {
    pub(super) bill_ids: Vec<u64>,
    pub(super) receipt_ids: Vec<usize>,
    pub(super) details: Vec<(Source, i64)>,
    pub(super) cash_paise: i64,
    pub(super) online_paise: i64,
}

pub(super) fn sales_history(
    fixture: &ConfirmationFixture,
    bounds: Bounds,
    search: &str,
) -> History {
    let projected = fixture
        .sales_history(
            Role::Owner,
            BillDate::new(2025, 3, 15).unwrap(),
            SalesPeriod::Custom(bounds),
            search,
        )
        .unwrap();
    let profit = projected.profit.unwrap();
    History {
        rows: projected
            .rows
            .into_iter()
            .map(|row| SaleRow {
                bill: row.bill,
                cancelled: row.cancelled,
                balance_paise: row.balance.minor_units(),
                remainder: row.remainder.minor_units(),
            })
            .collect(),
        packets: projected.active_packets,
        tiles: [
            projected.sales,
            projected.bill_paid,
            projected.receipt_paid,
            projected.total_collected,
        ]
        .map(|amount| amount.minor_units()),
        profit: Profit {
            bill_ids: profit.bill_ids.into_iter().map(|id| id.value()).collect(),
            revenue_paise: profit.revenue.minor_units(),
            cost_paise: profit.cost.minor_units(),
            profit: profit.profit.minor_units(),
        },
        collected: Collected {
            bill_ids: projected
                .collected
                .bill_ids
                .into_iter()
                .map(|id| id.value())
                .collect(),
            receipt_ids: projected
                .collected
                .receipt_ids
                .into_iter()
                .map(|id| id.0)
                .collect(),
            details: projected
                .collected
                .details
                .into_iter()
                .map(|row| {
                    (
                        match row.source {
                            SalesSource::Bill(id) => Source::Bill(id.value()),
                            SalesSource::Receipt(id) => Source::Receipt(id.0),
                        },
                        row.amount.minor_units(),
                    )
                })
                .collect(),
            cash_paise: projected.collected.cash.minor_units(),
            online_paise: projected.collected.online.minor_units(),
        },
    }
}
impl History {
    pub(super) fn row_ids(&self) -> Vec<u64> {
        self.rows
            .iter()
            .map(|row| row.bill.draft_id.value())
            .collect()
    }
    pub(super) const fn active_packet_count(&self) -> u32 {
        self.packets
    }
    pub(super) const fn tile_paise(&self) -> [i64; 4] {
        self.tiles
    }
    pub(super) fn profit(&self, role: Role) -> Result<&Profit, ConfirmationError> {
        if role == Role::Owner {
            Ok(&self.profit)
        } else {
            Err(ConfirmationError::AccessDenied)
        }
    }
    pub(super) const fn collected(&self) -> &Collected {
        &self.collected
    }
}
pub(super) const fn round_rupees(paise: i64) -> i64 {
    paise.div_euclid(100) + if paise.rem_euclid(100) >= 50 { 1 } else { 0 }
}
pub(super) fn money_r(paise: i64) -> String {
    format!("₹{}", round_rupees(paise))
}
