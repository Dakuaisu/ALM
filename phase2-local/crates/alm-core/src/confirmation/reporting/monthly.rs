//! Owner-only reports over current local synthetic sales and saved manual entries.
use std::collections::BTreeMap;

use super::super::manual_cashbook::{ManualCashBook, ManualCashBookId};
use super::super::{ConfirmationError, ConfirmationFixture, DraftId, Money, Role};
use crate::{BillDate, DraftCustomer};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MonthlyProfit {
    pub year: u16,
    pub month: u8,
    pub bills: Vec<DraftId>,
    pub cash_rows: Vec<ManualCashBookId>,
    pub revenue: Money,
    pub cost: Money,
    pub gross: Money,
    pub goods: Money,
    pub misc: Money,
    pub salary: Money,
    pub net: Money,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CustomerRank {
    pub name: String,
    pub total: Money,
    pub bills: Vec<(DraftId, Option<String>)>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReportsError {
    Confirmation(ConfirmationError),
    UnknownFrozenCost(DraftId),
}

impl From<ConfirmationError> for ReportsError {
    fn from(error: ConfirmationError) -> Self {
        Self::Confirmation(error)
    }
}

fn add(a: Money, b: Money) -> Result<Money, ConfirmationError> {
    a.minor_units()
        .checked_add(b.minor_units())
        .map(Money::from_minor_units)
        .ok_or(ConfirmationError::Overflow)
}

fn sub(a: Money, b: Money) -> Result<Money, ConfirmationError> {
    a.minor_units()
        .checked_sub(b.minor_units())
        .map(Money::from_minor_units)
        .ok_or(ConfirmationError::Overflow)
}

fn owner(role: Role) -> Result<(), ConfirmationError> {
    match role {
        Role::Owner => Ok(()),
        Role::Manager => Err(ConfirmationError::AccessDenied),
    }
}

impl ConfirmationFixture {
    /// Projects six complete calendar months, oldest first, without changing either source.
    /// Only frozen posting costs are used; missing cost returns `UnknownFrozenCost` and is
    /// never inferred from current catalog prices. Customer ranking remains independent.
    pub fn monthly_profit_report(
        &self,
        role: Role,
        book: &ManualCashBook,
        today: BillDate,
    ) -> Result<Vec<MonthlyProfit>, ReportsError> {
        owner(role)?;
        let (year, month, _) = today.parts();
        let serial = i32::from(year) * 12 + i32::from(month) - 1;
        let zero = Money::from_minor_units(0);
        let mut months = Vec::with_capacity(6);
        for offset in (0..6).rev() {
            let value = serial - offset;
            let y =
                u16::try_from(value.div_euclid(12)).map_err(|_| ConfirmationError::InvalidDraft)?;
            if y == 0 {
                return Err(ConfirmationError::InvalidDraft.into());
            }
            let m = u8::try_from(value.rem_euclid(12) + 1)
                .map_err(|_| ConfirmationError::InvalidDraft)?;
            months.push(MonthlyProfit {
                year: y,
                month: m,
                bills: Vec::new(),
                cash_rows: Vec::new(),
                revenue: zero,
                cost: zero,
                gross: zero,
                goods: zero,
                misc: zero,
                salary: zero,
                net: zero,
            });
        }
        for (id, (review, receipt, _)) in &self.current_sales {
            if self
                .lifecycle
                .get(id)
                .ok_or(ConfirmationError::InvalidDraft)?
                .0
            {
                continue;
            }
            let (y, m, _) = review
                .draft
                .bill_date()
                .ok_or(ConfirmationError::InvalidDraft)?
                .parts();
            if let Some(row) = months
                .iter_mut()
                .find(|row| (row.year, row.month) == (y, m))
            {
                let cost = Money::from_minor_units(
                    self.catalog
                        .frozen_cost(*id)?
                        .ok_or(ReportsError::UnknownFrozenCost(*id))?,
                );
                row.revenue = add(row.revenue, receipt.total)?;
                row.cost = add(row.cost, cost)?;
                row.gross = add(row.gross, sub(review.amounts()?.subtotal, cost)?)?;
                row.bills.push(*id);
            }
        }
        for entry in book.saved_rows() {
            let (y, m, _) = entry.date.parts();
            if let Some(row) = months
                .iter_mut()
                .find(|row| (row.year, row.month) == (y, m))
            {
                row.goods = add(row.goods, entry.amounts.goods)?;
                row.misc = add(row.misc, entry.amounts.misc)?;
                row.salary = add(row.salary, entry.amounts.salary)?;
                row.cash_rows.push(entry.id);
            }
        }
        for row in &mut months {
            row.net = sub(sub(row.gross, row.misc)?, row.salary)?;
        }
        Ok(months)
    }

    /// Ranks exact stored names over all active sales, using legacy key order for ties.
    pub fn customer_ranking(&self, role: Role) -> Result<Vec<CustomerRank>, ConfirmationError> {
        owner(role)?;
        let mut groups = BTreeMap::<String, CustomerRank>::new();
        for (id, (review, receipt, _)) in &self.current_sales {
            if self
                .lifecycle
                .get(id)
                .ok_or(ConfirmationError::InvalidDraft)?
                .0
            {
                continue;
            }
            let (name, customer) = match review.draft.customer() {
                DraftCustomer::Selected { name, id } => {
                    (name.as_str(), Some(id.value().to_owned()))
                }
                DraftCustomer::Typed(name) => (name.as_str(), None),
                DraftCustomer::Unassigned => ("", None),
            };
            let group = groups
                .entry(name.to_owned())
                .or_insert_with(|| CustomerRank {
                    name: name.to_owned(),
                    total: Money::from_minor_units(0),
                    bills: Vec::new(),
                });
            group.total = add(group.total, receipt.total)?;
            group.bills.push((*id, customer));
        }
        let mut ranks = Vec::with_capacity(groups.len());
        for (id, _) in self.first_posts.iter().rev() {
            let (review, _, _) = self
                .current_sales
                .get(id)
                .ok_or(ConfirmationError::InvalidDraft)?;
            if self
                .lifecycle
                .get(id)
                .ok_or(ConfirmationError::InvalidDraft)?
                .0
            {
                continue;
            }
            let name = match review.draft.customer() {
                DraftCustomer::Selected { name, .. } | DraftCustomer::Typed(name) => name.as_str(),
                DraftCustomer::Unassigned => "",
            };
            if let Some(group) = groups.remove(name) {
                ranks.push(group);
            }
        }
        if !groups.is_empty() {
            return Err(ConfirmationError::InvalidDraft);
        }
        ranks.sort_by_key(|row| match row.name.as_bytes() {
            [b'0'] | [b'1'..=b'9', ..] => row.name.parse::<u32>().unwrap_or(u32::MAX),
            _ => u32::MAX,
        });
        ranks.sort_by_key(|row| std::cmp::Reverse(row.total.minor_units()));
        ranks.truncate(8);
        Ok(ranks)
    }
}
