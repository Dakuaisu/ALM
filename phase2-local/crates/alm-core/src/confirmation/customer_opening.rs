use super::{ConfirmationError, ConfirmationFixture, CustomerLedgerKind, Money, Role};
use crate::{BillDate, DraftCustomer, DraftCustomerId};

/// A blank amount is saved as zero; the date is already normalized by the caller.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OpeningBalance {
    pub amount: Option<Money>,
    pub date: Option<BillDate>,
}

impl OpeningBalance {
    /// The typed opening amount: blank is `None` (saved as zero); otherwise signed rupees, a
    /// negative amount being an advance (`c.openingBalance = +ob.value`).
    ///
    /// # Errors
    /// Rejects text that is not exact signed rupees with at most two decimals.
    pub fn parse_amount(text: &str) -> Result<Option<Money>, crate::RupeesError> {
        if text.trim().is_empty() {
            Ok(None)
        } else {
            Money::parse_signed_rupees(text).map(Some)
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "demo-fixture", derive(serde::Serialize, serde::Deserialize))]
pub(super) struct StoredOpening {
    pub amount: Money,
    pub date: Option<BillDate>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OpeningLedgerKind {
    Opening,
    Transaction(CustomerLedgerKind),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpeningLedgerEntry {
    pub date: Option<BillDate>,
    pub kind: OpeningLedgerKind,
    pub debit: Money,
    pub credit: Money,
    pub balance: Money,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpeningLedger {
    pub entries: Vec<OpeningLedgerEntry>,
    pub closing: Money,
}

impl ConfirmationFixture {
    /// Registers a stable synthetic customer identity in this in-memory fixture.
    ///
    /// # Errors
    /// Denies Managers and rejects blank identities without mutation.
    pub fn register_customer(
        &mut self,
        role: Role,
        customer_id: &DraftCustomerId,
    ) -> Result<(), ConfirmationError> {
        if role != Role::Owner {
            return Err(ConfirmationError::AccessDenied);
        }
        if customer_id.value().trim().is_empty() {
            return Err(ConfirmationError::InvalidDraft);
        }
        self.registered_customers
            .insert(customer_id.value().to_owned());
        Ok(())
    }

    /// Replaces a known customer's opening without changing posted transaction facts.
    ///
    /// # Errors
    /// Denies Managers, unknown customers, and arithmetic overflow before mutation.
    pub fn save_customer_opening(
        &mut self,
        role: Role,
        customer_id: &DraftCustomerId,
        opening: OpeningBalance,
    ) -> Result<(), ConfirmationError> {
        if role != Role::Owner {
            return Err(ConfirmationError::AccessDenied);
        }
        if !self.registered_customers.contains(customer_id.value()) && !self.current_sales.values().any(|(review, _, _)| {
            matches!(review.draft.customer(), DraftCustomer::Selected { id, .. } if id == customer_id)
        }) {
            return Err(ConfirmationError::InvalidDraft);
        }
        let key = customer_id.value();
        let old = self
            .customer_openings
            .get(key)
            .map(|value| value.amount)
            .unwrap_or(Money::from_minor_units(0))
            .minor_units();
        let amount = opening.amount.unwrap_or(Money::from_minor_units(0));
        let current = self.customer_balances.get(key).copied().unwrap_or_default();
        let balance =
            i64::try_from(i128::from(current) - i128::from(old) + i128::from(amount.minor_units()))
                .map_err(|_| ConfirmationError::Overflow)?;
        self.customer_balances.insert(key.to_owned(), balance);
        self.customer_openings.insert(
            key.to_owned(),
            StoredOpening {
                amount,
                date: opening.date,
            },
        );
        Ok(())
    }
}
