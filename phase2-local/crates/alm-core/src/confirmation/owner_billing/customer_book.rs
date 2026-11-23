use crate::confirmation::{ConfirmationError, ConfirmationFixture};
use crate::{DraftCustomerId, Money, Role};

use super::{BillingDirectory, DirectoryCustomer};

/// Which side of zero a balance sits on, with the amount to show as positive.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BalanceSide {
    Due(Money),
    Advance(Money),
    Settled,
}

impl BalanceSide {
    pub fn of(balance: Money) -> Result<Self, ConfirmationError> {
        let paise = balance.minor_units();
        Ok(match paise.cmp(&0) {
            std::cmp::Ordering::Greater => Self::Due(balance),
            std::cmp::Ordering::Equal => Self::Settled,
            std::cmp::Ordering::Less => Self::Advance(Money::from_minor_units(
                paise.checked_neg().ok_or(ConfirmationError::Overflow)?,
            )),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CustomerFilter {
    All,
    /// Only customers who are debtors under the Counter rule.
    Due,
    /// Case-insensitive name match.
    Name(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CustomerBookRow {
    pub customer: DirectoryCustomer,
    pub balance: Money,
    pub side: BalanceSide,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CustomerBook {
    /// Filtered rows sorted by name.
    pub rows: Vec<CustomerBookRow>,
    /// Totals over every directory customer, independent of the filter.
    pub receivable: Money,
    pub advances: Money,
    pub debtors: usize,
    pub customers: usize,
}

/// The Counter's debtor rule: a positive balance after rounding to whole rupees.
pub fn is_debtor(balance: Money) -> bool {
    balance.rounded_rupees() > 0
}

impl BillingDirectory {
    /// Owner-only customer list with opening-aware closing balances (the L14 ledger).
    ///
    /// # Errors
    /// Denies Managers, propagates ledger errors and rejects total overflow.
    pub fn customer_book(
        &self,
        authority: &ConfirmationFixture,
        role: Role,
        filter: &CustomerFilter,
    ) -> Result<CustomerBook, ConfirmationError> {
        if role != Role::Owner {
            return Err(ConfirmationError::AccessDenied);
        }
        let mut receivable = 0_i64;
        let mut advances = 0_i64;
        let mut debtors = 0;
        let mut customers = 0;
        let mut rows = Vec::new();
        let needle = match filter {
            CustomerFilter::Name(text) => text.trim().to_lowercase(),
            CustomerFilter::All | CustomerFilter::Due => String::new(),
        };
        for customer in self.customers(authority) {
            let balance = authority
                .customer_ledger(role, &DraftCustomerId::new(customer.id.into()))?
                .closing;
            let side = BalanceSide::of(balance)?;
            customers += 1;
            match side {
                BalanceSide::Due(amount) => {
                    receivable = receivable
                        .checked_add(amount.minor_units())
                        .ok_or(ConfirmationError::Overflow)?;
                }
                BalanceSide::Advance(amount) => {
                    advances = advances
                        .checked_add(amount.minor_units())
                        .ok_or(ConfirmationError::Overflow)?;
                }
                BalanceSide::Settled => {}
            }
            if is_debtor(balance) {
                debtors += 1;
            }
            let keep = match filter {
                CustomerFilter::All => true,
                CustomerFilter::Due => is_debtor(balance),
                CustomerFilter::Name(_) => customer.name.to_lowercase().contains(&needle),
            };
            if keep {
                rows.push(CustomerBookRow {
                    customer,
                    balance,
                    side,
                });
            }
        }
        rows.sort_by(|a, b| a.customer.name.cmp(b.customer.name));
        Ok(CustomerBook {
            rows,
            receivable: Money::from_minor_units(receivable),
            advances: Money::from_minor_units(advances),
            debtors,
            customers,
        })
    }
}

#[cfg(test)]
#[path = "customer_book_tests.rs"]
mod tests;
