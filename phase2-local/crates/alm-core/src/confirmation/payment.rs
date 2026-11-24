use std::collections::BTreeMap;

use super::{ConfirmationError, ConfirmationFixture, DraftId, Money, ReceiptMode, Review, Role};
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

    pub const fn payment_mode(&self) -> ReceiptMode {
        self.payment_mode
    }

    pub(super) fn with_named_payment(mut self, received: Money) -> Result<Self, ConfirmationError> {
        self.payment = ProposedPayment::Named(received);
        let total = self.amounts()?.total.minor_units();
        stage(&self, total, &BTreeMap::new())?;
        Ok(self)
    }
}

/// Legacy approvals carry no proposed payment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "demo-fixture", derive(serde::Serialize, serde::Deserialize))]
pub enum ProposedPayment {
    Legacy,
    Named(Money),
    WalkInCash(Money),
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
        self.bill_for_payment(role, id)?;
        self.review(role, id)?.with_named_payment(received)
    }

    /// Selects a payment mode for a regular named bill after Owner validation.
    pub fn review_named_with_mode(
        &self,
        role: Role,
        id: DraftId,
        received: Money,
        mode: ReceiptMode,
    ) -> Result<Review, ConfirmationError> {
        let mut review = self.review_named(role, id, received)?;
        if review.draft.crate_row().is_some() {
            return Err(ConfirmationError::InvalidDraft);
        }
        review.payment_mode = mode;
        Ok(review)
    }

    /// Explicitly approves the checked full total for the unlinked walk-in label.
    pub fn review_walk_in_cash(
        &self,
        role: Role,
        id: DraftId,
    ) -> Result<Review, ConfirmationError> {
        walk_in_identity(self.bill_for_payment(role, id)?)?;
        let mut review = self.review(role, id)?;
        review.payment = ProposedPayment::WalkInCash(review.amounts()?.total);
        Ok(review)
    }

    fn bill_for_payment(
        &self,
        role: Role,
        id: DraftId,
    ) -> Result<&ProvisionalDraft, ConfirmationError> {
        if role != Role::Owner {
            return Err(ConfirmationError::AccessDenied);
        }
        let (_, draft) = self
            .drafts
            .get(&id)
            .ok_or(ConfirmationError::DraftNotFound)?;
        super::posting::validate_bill(draft)?;
        Ok(draft)
    }

    pub fn customer_due(&self, id: &str) -> Money {
        Money::from_minor_units(self.customer_balances.get(id).copied().unwrap_or_default())
    }
}

fn walk_in_identity(draft: &ProvisionalDraft) -> Result<(), ConfirmationError> {
    match draft.customer() {
        DraftCustomer::Typed(name) if name == "Local - Neighbor" => Ok(()),
        DraftCustomer::Typed(_) | DraftCustomer::Selected { .. } | DraftCustomer::Unassigned => {
            Err(ConfirmationError::InvalidDraft)
        }
    }
}

pub(super) fn walk_in_stage(
    draft: &ProvisionalDraft,
    received: Money,
    total: Money,
) -> Result<FinancialStage, ConfirmationError> {
    walk_in_identity(draft)?;
    if received != total || received.minor_units() < 0 {
        return Err(ConfirmationError::InvalidAmount);
    }
    Ok(FinancialStage {
        received: Some(received),
        due: Some(Money::from_minor_units(0)),
        balance: None,
    })
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
        ProposedPayment::WalkInCash(received) => {
            return walk_in_stage(&review.draft, received, Money::from_minor_units(total));
        }
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
