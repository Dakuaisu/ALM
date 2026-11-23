//! Legacy case L16-01.
use super::super::{ConfirmationFixture, CustomerRank, MonthlyProfit};
use crate::confirmation::manual_cashbook::ManualCashBookId;

type RankedBill = (u64, Option<String>);
type RankedCustomer = (String, i64, Vec<RankedBill>);

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Month {
    pub key: (u16, u8),
    pub bills: Vec<u64>,
    pub cash_rows: Vec<ManualCashBookId>,
    pub receipts_excluded: Vec<usize>,
    pub paise: [i64; 6],
    pub misc_salary: [i64; 2],
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Report {
    pub months: Vec<Month>,
    pub ranking: Vec<RankedCustomer>,
}

pub(super) const fn round(p: i64) -> i64 {
    p.div_euclid(100) + if p.rem_euclid(100) >= 50 { 1 } else { 0 }
}

impl Month {
    pub fn display(&self) -> [String; 6] {
        let money = |p| format!("₹{}", round(p));
        [
            self.bills.len().to_string(),
            money(self.paise[0]),
            money(self.paise[2]),
            if self.paise[3] == 0 {
                "—".into()
            } else {
                money(self.paise[3])
            },
            money(self.paise[4]),
            money(self.paise[5]),
        ]
    }
}

pub(super) fn format_report(
    fixture: &ConfirmationFixture,
    rows: Vec<MonthlyProfit>,
    ranks: Vec<CustomerRank>,
) -> Report {
    let months = rows
        .into_iter()
        .map(|row| Month {
            key: (row.year, row.month),
            bills: row.bills.iter().map(|id| id.value()).collect(),
            cash_rows: row.cash_rows,
            receipts_excluded: fixture
                .preview_payments
                .iter()
                .enumerate()
                .filter_map(|(index, receipt)| {
                    let (year, month, _) = receipt.date.parts();
                    ((year, month) == (row.year, row.month)).then_some(index + 1)
                })
                .collect(),
            paise: [
                row.revenue.minor_units(),
                row.cost.minor_units(),
                row.gross.minor_units(),
                row.goods.minor_units(),
                row.misc
                    .minor_units()
                    .checked_add(row.salary.minor_units())
                    .unwrap(),
                row.net.minor_units(),
            ],
            misc_salary: [row.misc.minor_units(), row.salary.minor_units()],
        })
        .collect();
    let ranking = ranks
        .into_iter()
        .map(|row| {
            (
                row.name,
                row.total.minor_units(),
                row.bills
                    .into_iter()
                    .map(|(id, customer)| (id.value(), customer))
                    .collect(),
            )
        })
        .collect();
    Report { months, ranking }
}
