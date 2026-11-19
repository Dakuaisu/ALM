use std::collections::BTreeMap;

use super::{ConfirmationError, ConfirmationFixture, DraftId, Money, Review, Role};
use crate::DraftCustomer;
use crate::ProvisionalDraft;

impl Review {
    pub const fn draft(&self) -> &ProvisionalDraft {
        &self.draft
    }

    pub fn unit_price(&self) -> Money {
        self.prices[0]
    }

    pub fn unit_prices(&self) -> &[Money] {
        &self.prices
    }

    pub const fn payment(&self) -> ProposedPayment {
        self.payment
    }
}

/// Legacy approvals carry no proposed payment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProposedPayment {
    Legacy,
    Named(Money),
}

pub(super) struct FinancialStage {
    pub(super) received: Option<Money>,
    pub(super) due: Option<Money>,
    pub(super) balance: Option<(String, i64)>,
}

impl ConfirmationFixture {
    /// Creates an Owner review with a proposed amount received for a selected customer.
    pub fn review_named(
        &self,
        role: Role,
        id: DraftId,
        received: Money,
    ) -> Result<Review, ConfirmationError> {
        let mut review = self.review(role, id)?;
        review.payment = ProposedPayment::Named(received);
        let total = review.draft.lines().iter().zip(&review.prices).try_fold(
            0_i64,
            |sum, (line, price)| {
                price
                    .minor_units()
                    .checked_mul(i64::from(line.quantity().get()))
                    .and_then(|value| sum.checked_add(value))
                    .ok_or(ConfirmationError::Overflow)
            },
        )?;
        stage(&review, total, &BTreeMap::new())?;
        Ok(review)
    }

    pub fn customer_due(&self, id: &str) -> Money {
        Money::from_minor_units(self.customer_balances.get(id).copied().unwrap_or_default())
    }
}

pub(super) fn stage(
    review: &Review,
    total: i64,
    balances: &BTreeMap<String, i64>,
) -> Result<FinancialStage, ConfirmationError> {
    let received = match review.payment {
        ProposedPayment::Legacy => {
            return Ok(FinancialStage {
                received: None,
                due: None,
                balance: None,
            });
        }
        ProposedPayment::Named(received) => received.minor_units(),
    };
    let DraftCustomer::Selected { id, name } = review.draft.customer() else {
        return Err(ConfirmationError::InvalidDraft);
    };
    if id.value().trim().is_empty() || name.trim().is_empty() {
        return Err(ConfirmationError::InvalidDraft);
    }
    if received < 0 || received > total {
        return Err(ConfirmationError::InvalidAmount);
    }
    let due = total - received;
    let balance = balances
        .get(id.value())
        .copied()
        .unwrap_or_default()
        .checked_add(due)
        .ok_or(ConfirmationError::Overflow)?;
    Ok(FinancialStage {
        received: Some(Money::from_minor_units(received)),
        due: Some(Money::from_minor_units(due)),
        balance: Some((id.value().to_owned(), balance)),
    })
}
