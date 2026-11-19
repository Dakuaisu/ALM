use crate::{DraftCustomer, Money, ProvisionalDraft, Role};

use super::{
    ConfirmationAck, ConfirmationError, ConfirmationFixture, Receipt, RequestId, Review, payment,
};

pub(super) fn validate_bill(draft: &ProvisionalDraft) -> Result<(), ConfirmationError> {
    if draft.lines().is_empty()
        && draft
            .crate_row()
            .is_none_or(|row| row.components().is_empty())
    {
        return Err(ConfirmationError::MissingBillableItems);
    }
    let name = match draft.customer() {
        DraftCustomer::Unassigned => "",
        DraftCustomer::Typed(name) | DraftCustomer::Selected { name, .. } => name,
    };
    if name.is_empty() {
        return Err(ConfirmationError::MissingCustomerName);
    }
    if draft.bill_date().is_none() {
        return Err(ConfirmationError::MissingBillDate);
    }
    Ok(())
}

impl ConfirmationAck {
    pub const fn receipt(&self) -> &Receipt {
        &self.receipt
    }

    pub(crate) fn into_receipt_for(self, request: RequestId, review: &Review) -> Option<Receipt> {
        (self.request == request && &self.review == review).then_some(self.receipt)
    }
}

impl ConfirmationFixture {
    pub fn confirm_acknowledged(
        &mut self,
        role: Role,
        request: RequestId,
        review: &Review,
    ) -> Result<ConfirmationAck, ConfirmationError> {
        let receipt = self.confirm(role, request, review)?;
        Ok(ConfirmationAck {
            request,
            review: review.clone(),
            receipt,
        })
    }

    /// Replays return the original receipt; all fallible calculations precede mutation.
    pub fn confirm(
        &mut self,
        role: Role,
        request: RequestId,
        review: &Review,
    ) -> Result<Receipt, ConfirmationError> {
        if role != Role::Owner {
            return Err(ConfirmationError::AccessDenied);
        }
        if let Some((original, receipt)) = self.requests.get(&request) {
            return if original == review {
                Ok(receipt.clone())
            } else {
                Err(ConfirmationError::RequestConflict)
            };
        }
        let id = review.draft.id();
        if self.posted.contains_key(&id) {
            return Err(ConfirmationError::DraftAlreadyPosted);
        }
        let current = self.drafts.get(&id).ok_or(ConfirmationError::StaleReview)?;
        if current.0 != review.draft_revision
            || current.1 != review.draft
            || self.terms_revision != review.terms_revision
            || !self.checkout_current(review)
            || !self.checkout_approved(review)
        {
            return Err(ConfirmationError::StaleReview);
        }
        validate_bill(&review.draft)?;
        let (catalog, quantity, total) = self.catalog.stage(review)?;
        let financial = payment::stage(review, total, &self.customer_balances)?;
        let received_ledger = self
            .received_ledger
            .checked_add(financial.received.map_or(0, Money::minor_units))
            .ok_or(ConfirmationError::Overflow)?;
        let ledger = self
            .ledger
            .checked_add(total)
            .ok_or(ConfirmationError::Overflow)?;
        let next_invoice = self
            .next_invoice
            .checked_add(1)
            .ok_or(ConfirmationError::Overflow)?;
        let movement = self
            .movement_epoch
            .checked_add(1)
            .ok_or(ConfirmationError::Overflow)?;
        let receipt = Receipt {
            draft_id: id,
            invoice: self.next_invoice,
            quantity,
            total: Money::from_minor_units(total),
            received: financial.received,
            due: financial.due,
            customer_balance: financial
                .balance
                .as_ref()
                .map(|(_, value)| Money::from_minor_units(*value)),
        };
        self.catalog = catalog;
        self.ledger = ledger;
        self.received_ledger = received_ledger;
        if let Some((customer, balance)) = financial.balance {
            self.customer_balances.insert(customer, balance);
        }
        self.next_invoice = next_invoice;
        self.movement_epoch = movement;
        self.lifecycle.insert(id, (false, movement));
        self.requests
            .insert(request, (review.clone(), receipt.clone()));
        self.current_sales
            .insert(id, (review.clone(), receipt.clone(), 0));
        self.posted.insert(id, request);
        Ok(receipt)
    }
}
