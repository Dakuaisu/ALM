//! Owner-only manual Cash Book for synthetic local development. System figures come
//! from `cashbook_day_close`: Online receipts are supported, Online bill payments are not.

use std::collections::BTreeMap;

use super::{CashBookDay, ConfirmationError, ConfirmationFixture};
use crate::{BillDate, Money, Role};

const ZERO: Money = Money::from_minor_units(0);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CashBookAmounts {
    pub cash: Money,
    pub paytm: Money,
    pub phonepe: Money,
    pub goods: Money,
    pub misc: Money,
    pub salary: Money,
    pub debt: Money,
}

impl CashBookAmounts {
    pub const ZERO: Self = Self {
        cash: ZERO,
        paytm: ZERO,
        phonepe: ZERO,
        goods: ZERO,
        misc: ZERO,
        salary: ZERO,
        debt: ZERO,
    };
}

/// Reads one typed Cash Book amount as the HTML does (`+input.value || 0`): blank is zero;
/// anything else must be exact non-negative rupees.
pub fn parse_cash_book_amount(text: &str) -> Result<Money, crate::RupeesError> {
    if text.trim().is_empty() {
        Ok(ZERO)
    } else {
        Money::parse_rupees(text)
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ManualCashBookId(u64);

impl ManualCashBookId {
    /// The fixture-local row number, for carrying a selection across IPC.
    pub const fn value(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ManualCashBookDraft {
    id: Option<ManualCashBookId>,
    pub date: Option<BillDate>,
    pub amounts: CashBookAmounts,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ManualCashBookRow {
    pub id: ManualCashBookId,
    pub date: BillDate,
    pub amounts: CashBookAmounts,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CashBookComparison {
    pub system: CashBookDay,
    pub cash_difference_rupees: i64,
    pub online_difference_rupees: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CashBookMonth {
    pub rows: usize,
    pub goods: Money,
    pub misc: Money,
    pub salary: Money,
    pub debt: Money,
    pub income: Money,
    pub expenses: Money,
    pub net: Money,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CashBookDeletion {
    Decline(ManualCashBookId),
    Confirm(ManualCashBookId),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ManualCashBookError {
    AccessDenied,
    MissingDate,
    RowNotFound,
    Overflow,
    System(ConfirmationError),
}

impl ManualCashBookDraft {
    pub const fn new(date: Option<BillDate>, amounts: CashBookAmounts) -> Self {
        Self {
            id: None,
            date,
            amounts,
        }
    }

    pub fn compare(
        &self,
        fixture: &ConfirmationFixture,
        role: Role,
    ) -> Result<CashBookComparison, ManualCashBookError> {
        let system = self.system_day(fixture, role)?;
        let entered_online = add(self.amounts.paytm, self.amounts.phonepe)?;
        Ok(CashBookComparison {
            cash_difference_rupees: difference(self.amounts.cash, system.cash_rupees)?,
            online_difference_rupees: difference(entered_online, system.online_rupees)?,
            system,
        })
    }

    /// Copies the rounded day figures into the draft only, with all Online assigned to Paytm.
    pub fn use_system_figures(
        &mut self,
        fixture: &ConfirmationFixture,
        role: Role,
    ) -> Result<(), ManualCashBookError> {
        let system = self.system_day(fixture, role)?;
        let cash = from_rupees(system.cash_rupees)?;
        let paytm = from_rupees(system.online_rupees)?;
        self.amounts.cash = cash;
        self.amounts.paytm = paytm;
        self.amounts.phonepe = ZERO;
        Ok(())
    }

    fn system_day(
        &self,
        fixture: &ConfirmationFixture,
        role: Role,
    ) -> Result<CashBookDay, ManualCashBookError> {
        owner(role)?;
        fixture
            .cashbook_day_close(role, self.date.ok_or(ManualCashBookError::MissingDate)?)
            .map_err(ManualCashBookError::System)
    }
}

impl ManualCashBookRow {
    pub const fn draft(self) -> ManualCashBookDraft {
        ManualCashBookDraft {
            id: Some(self.id),
            date: Some(self.date),
            amounts: self.amounts,
        }
    }
}

#[derive(Default)]
pub struct ManualCashBook {
    rows: BTreeMap<ManualCashBookId, ManualCashBookRow>,
    next_id: u64,
}

impl ManualCashBook {
    pub(super) fn saved_rows(&self) -> impl Iterator<Item = &ManualCashBookRow> {
        self.rows.values()
    }

    pub fn row(
        &self,
        role: Role,
        id: ManualCashBookId,
    ) -> Result<Option<ManualCashBookRow>, ManualCashBookError> {
        owner(role)?;
        Ok(self.rows.get(&id).copied())
    }

    /// Saved rows newest date first; equal dates keep the later entry first.
    pub fn rows(&self, role: Role) -> Result<Vec<ManualCashBookRow>, ManualCashBookError> {
        owner(role)?;
        let mut rows: Vec<_> = self.rows.values().copied().collect();
        rows.sort_by(|a, b| b.date.parts().cmp(&a.date.parts()).then(b.id.cmp(&a.id)));
        Ok(rows)
    }

    /// Finds a saved row by its IPC row number.
    pub fn find(&self, role: Role, value: u64) -> Result<ManualCashBookRow, ManualCashBookError> {
        owner(role)?;
        self.rows
            .get(&ManualCashBookId(value))
            .copied()
            .ok_or(ManualCashBookError::RowNotFound)
    }

    pub fn save(
        &mut self,
        role: Role,
        draft: &ManualCashBookDraft,
    ) -> Result<ManualCashBookRow, ManualCashBookError> {
        owner(role)?;
        let date = draft.date.ok_or(ManualCashBookError::MissingDate)?;
        let id = match draft.id {
            Some(id) if self.rows.contains_key(&id) => id,
            Some(_) => return Err(ManualCashBookError::RowNotFound),
            None => {
                let id = ManualCashBookId(self.next_id);
                self.next_id = self
                    .next_id
                    .checked_add(1)
                    .ok_or(ManualCashBookError::Overflow)?;
                id
            }
        };
        let row = ManualCashBookRow {
            id,
            date,
            amounts: draft.amounts,
        };
        self.rows.insert(id, row);
        Ok(row)
    }

    pub fn delete(
        &mut self,
        role: Role,
        deletion: CashBookDeletion,
    ) -> Result<Option<ManualCashBookRow>, ManualCashBookError> {
        owner(role)?;
        match deletion {
            CashBookDeletion::Decline(_) => Ok(None),
            CashBookDeletion::Confirm(id) => Ok(self.rows.remove(&id)),
        }
    }

    pub fn month(&self, role: Role, date: BillDate) -> Result<CashBookMonth, ManualCashBookError> {
        owner(role)?;
        let (year, month, _) = date.parts();
        let mut total = CashBookMonth {
            rows: 0,
            goods: ZERO,
            misc: ZERO,
            salary: ZERO,
            debt: ZERO,
            income: ZERO,
            expenses: ZERO,
            net: ZERO,
        };
        for row in self.rows.values() {
            let (y, m, _) = row.date.parts();
            if (y, m) != (year, month) {
                continue;
            }
            let a = row.amounts;
            total.rows += 1;
            total.goods = add(total.goods, a.goods)?;
            total.misc = add(total.misc, a.misc)?;
            total.salary = add(total.salary, a.salary)?;
            total.debt = add(total.debt, a.debt)?;
            total.income = add(total.income, add(add(a.cash, a.paytm)?, a.phonepe)?)?;
        }
        total.expenses = add(add(total.goods, total.misc)?, total.salary)?;
        total.net = sub(total.income, total.expenses)?;
        Ok(total)
    }
}

const fn owner(role: Role) -> Result<(), ManualCashBookError> {
    match role {
        Role::Owner => Ok(()),
        Role::Manager => Err(ManualCashBookError::AccessDenied),
    }
}

fn add(left: Money, right: Money) -> Result<Money, ManualCashBookError> {
    left.minor_units()
        .checked_add(right.minor_units())
        .map(Money::from_minor_units)
        .ok_or(ManualCashBookError::Overflow)
}

fn sub(left: Money, right: Money) -> Result<Money, ManualCashBookError> {
    left.minor_units()
        .checked_sub(right.minor_units())
        .map(Money::from_minor_units)
        .ok_or(ManualCashBookError::Overflow)
}

// Converts whole-rupee system figures, not typed text; typed amounts use `Money::parse_rupees`.
fn from_rupees(rupees: i64) -> Result<Money, ManualCashBookError> {
    rupees
        .checked_mul(100)
        .map(Money::from_minor_units)
        .ok_or(ManualCashBookError::Overflow)
}

fn difference(entered: Money, system_rupees: i64) -> Result<i64, ManualCashBookError> {
    let units = sub(entered, from_rupees(system_rupees)?)?.minor_units();
    Ok(units.div_euclid(100) + i64::from(units.rem_euclid(100) >= 50))
}

#[cfg(test)]
mod input_tests {
    use super::{CashBookAmounts, parse_cash_book_amount};
    use crate::{Money, RupeesError};

    #[test]
    fn blank_cash_book_inputs_read_as_zero_and_typed_ones_exactly() {
        for (text, paise) in [("", 0), ("   ", 0), ("0", 0), ("20.5", 2_050), (" .01 ", 1)] {
            assert_eq!(
                parse_cash_book_amount(text),
                Ok(Money::from_minor_units(paise))
            );
        }
        for text in ["-1", "abc", "1.234"] {
            assert_eq!(parse_cash_book_amount(text), Err(RupeesError::Invalid));
        }
        assert!(CashBookAmounts::ZERO.cash.is_zero());
        assert!(!Money::from_minor_units(-1).is_zero());
    }
}

#[cfg(test)]
mod listing_tests {
    use super::{CashBookAmounts, ManualCashBook, ManualCashBookDraft, ManualCashBookError, ZERO};
    use crate::{BillDate, Money, Role};

    #[test]
    fn rows_list_newest_date_first_and_find_is_owner_only() {
        let amounts = CashBookAmounts {
            cash: Money::from_minor_units(100),
            paytm: ZERO,
            phonepe: ZERO,
            goods: ZERO,
            misc: ZERO,
            salary: ZERO,
            debt: ZERO,
        };
        let mut book = ManualCashBook::default();
        for day in [1, 3, 3, 2] {
            let date = BillDate::new(2025, 5, day).ok();
            book.save(Role::Owner, &ManualCashBookDraft::new(date, amounts))
                .unwrap();
        }
        let order: Vec<_> = book
            .rows(Role::Owner)
            .unwrap()
            .iter()
            .map(|row| (row.date.parts().2, row.id.value()))
            .collect();
        assert_eq!(order, [(3, 2), (3, 1), (2, 3), (1, 0)]);
        assert_eq!(book.find(Role::Owner, 3).unwrap().date.parts().2, 2);
        assert_eq!(
            book.find(Role::Owner, 9),
            Err(ManualCashBookError::RowNotFound)
        );
        assert_eq!(
            book.rows(Role::Manager),
            Err(ManualCashBookError::AccessDenied)
        );
        assert_eq!(
            book.find(Role::Manager, 0),
            Err(ManualCashBookError::AccessDenied)
        );
    }
}
