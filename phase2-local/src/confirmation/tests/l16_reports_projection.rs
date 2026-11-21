use super::super::*;
use crate::confirmation::manual_cashbook::{ManualCashBookId, ManualCashBookRow};
use std::collections::BTreeMap;

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

pub(super) fn report(
    fixture: &ConfirmationFixture,
    input: (&[ManualCashBookRow], BillDate),
    role: Role,
) -> Result<Report, ConfirmationError> {
    if role != Role::Owner {
        return Err(ConfirmationError::AccessDenied);
    }
    let (rows, today) = input;
    let (year, month, _) = today.parts();
    let current = i32::from(year) * 12 + i32::from(month) - 1;
    let mut months: Vec<_> = (0..6)
        .rev()
        .map(|offset| {
            let serial = current - offset;
            Month {
                key: (
                    u16::try_from(serial.div_euclid(12)).unwrap(),
                    u8::try_from(serial.rem_euclid(12) + 1).unwrap(),
                ),
                bills: Vec::new(),
                cash_rows: Vec::new(),
                receipts_excluded: Vec::new(),
                paise: [0; 6],
                misc_salary: [0; 2],
            }
        })
        .collect();
    let mut ranking = BTreeMap::<String, (i64, Vec<RankedBill>)>::new();
    for (id, (review, bill, _)) in &fixture.current_sales {
        if fixture.lifecycle[id].0 {
            continue;
        }
        let (name, customer_id) = match review.draft.customer() {
            DraftCustomer::Selected { name, id } => (name.as_str(), Some(id.value().to_owned())),
            DraftCustomer::Typed(name) => (name.as_str(), None),
            DraftCustomer::Unassigned => ("", None),
        };
        let group = ranking.entry(name.to_owned()).or_default();
        group.0 += bill.total.minor_units();
        group.1.push((id.value(), customer_id));
        let (y, m, _) = review.draft.bill_date().unwrap().parts();
        if let Some(row) = months.iter_mut().find(|row| row.key == (y, m)) {
            row.bills.push(id.value());
            row.paise[0] += bill.total.minor_units();
            row.paise[1] += fixture
                .catalog
                .frozen_cost(*id)?
                .ok_or(ConfirmationError::InvalidDraft)?;
        }
    }
    for row in &mut months {
        for entry in rows {
            let (year, month, _) = entry.date.parts();
            if row.key == (year, month) {
                row.cash_rows.push(entry.id);
                row.paise[3] += entry.amounts.goods.minor_units();
                row.misc_salary[0] += entry.amounts.misc.minor_units();
                row.misc_salary[1] += entry.amounts.salary.minor_units();
            }
        }
        for (index, receipt) in fixture.preview_payments.iter().enumerate() {
            let (year, month, _) = receipt.date.parts();
            if row.key == (year, month) {
                row.receipts_excluded.push(index + 1);
            }
        }
        row.paise[4] = row.misc_salary[0] + row.misc_salary[1];
        row.paise[2] = row.paise[0] - row.paise[1];
        row.paise[5] = row.paise[2] - row.paise[4];
    }
    let mut ranking: Vec<_> = ranking
        .into_iter()
        .map(|(name, (amount, ids))| (name, amount, ids))
        .collect();
    ranking.sort_by_key(|(_, amount, _)| std::cmp::Reverse(*amount));
    ranking.truncate(8);
    Ok(Report { months, ranking })
}
