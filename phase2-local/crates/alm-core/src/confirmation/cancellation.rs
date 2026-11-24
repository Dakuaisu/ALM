use super::{
    ConfirmationError, ConfirmationFixture, DraftId, ProposedPayment, Receipt, Review, Role,
};
use crate::DraftCustomer;

pub(super) fn require_received_regular(
    review: &Review,
    receipt: &Receipt,
) -> Result<(), ConfirmationError> {
    let received = receipt
        .received
        .ok_or(ConfirmationError::InvalidDraft)?
        .minor_units();
    let due = receipt
        .due
        .ok_or(ConfirmationError::InvalidDraft)?
        .minor_units();
    if !matches!(review.draft.customer(), DraftCustomer::Selected { .. })
        || !matches!(review.payment, ProposedPayment::Named(_))
        || ((review.checkout.is_some() || !review.draft.crate_rows().is_empty())
            && !review.is_named_crate_sale())
        || received <= 0
        || due < 0
    {
        return Err(ConfirmationError::InvalidDraft);
    }
    Ok(())
}

impl ConfirmationFixture {
    pub fn cancel_sale(&mut self, role: Role, id: DraftId) -> Result<(), ConfirmationError> {
        self.set_cancelled(role, id, true)
    }

    pub fn undo_cancel_sale(&mut self, role: Role, id: DraftId) -> Result<(), ConfirmationError> {
        self.set_cancelled(role, id, false)
    }

    fn set_cancelled(
        &mut self,
        role: Role,
        id: DraftId,
        cancel: bool,
    ) -> Result<(), ConfirmationError> {
        if role != Role::Owner {
            return Err(ConfirmationError::AccessDenied);
        }
        let &(cancelled, epoch) = self
            .lifecycle
            .get(&id)
            .ok_or(ConfirmationError::DraftNotFound)?;
        let (review, receipt, revision) = self
            .current_sales
            .get(&id)
            .ok_or(ConfirmationError::DraftNotFound)?;
        require_received_regular(review, receipt)?;
        let DraftCustomer::Selected { id: customer, .. } = review.draft.customer() else {
            return Err(ConfirmationError::InvalidDraft);
        };
        let received = receipt
            .received
            .ok_or(ConfirmationError::InvalidDraft)?
            .minor_units();
        let due = receipt
            .due
            .ok_or(ConfirmationError::InvalidDraft)?
            .minor_units();
        if cancelled == cancel {
            return Ok(());
        }
        // The global epoch also rejects unrelated intervening stock movements conservatively.
        if epoch != self.movement_epoch {
            return Err(ConfirmationError::StaleReview);
        }
        let adjust = |value: i64, amount: i64| {
            if cancel {
                value.checked_sub(amount)
            } else {
                value.checked_add(amount)
            }
            .ok_or(ConfirmationError::Overflow)
        };
        let ledger = adjust(self.ledger, receipt.total.minor_units())?;
        let received_ledger = adjust(self.received_ledger, received)?;
        let balance = adjust(
            *self
                .customer_balances
                .get(customer.value())
                .ok_or(ConfirmationError::InvalidDraft)?,
            due,
        )?;
        let customer = customer.value().to_owned();
        let revision = revision.checked_add(1).ok_or(ConfirmationError::Overflow)?;
        let movement = self
            .movement_epoch
            .checked_add(1)
            .ok_or(ConfirmationError::Overflow)?;
        let catalog = self.catalog.stage_cancellation(id, cancel)?;
        let mut sale = self
            .current_sales
            .get(&id)
            .cloned()
            .ok_or(ConfirmationError::DraftNotFound)?;
        sale.2 = revision;
        self.catalog = catalog;
        self.ledger = ledger;
        self.received_ledger = received_ledger;
        self.customer_balances.insert(customer, balance);
        self.current_sales.insert(id, sale);
        self.movement_epoch = movement;
        self.lifecycle.insert(id, (cancel, movement));
        Ok(())
    }
}
