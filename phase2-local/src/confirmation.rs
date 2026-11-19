//! Synthetic in-memory confirmation fixture, not a production posting authority.
use std::collections::BTreeMap;

use crate::{DraftId, Money, ProductId, ProvisionalDraft, Role};

mod pending;
pub use pending::PendingError;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RequestId(pub(crate) u64);

impl RequestId {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfirmationError {
    AccessDenied,
    InvalidAmount,
    InvalidDraft,
    DraftNotFound,
    StaleReview,
    RequestConflict,
    DraftAlreadyPosted,
    InsufficientStock,
    Overflow,
}

/// A review can only be produced by the fixture and cannot be altered by its caller.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Review {
    draft: ProvisionalDraft,
    draft_revision: u64,
    terms_revision: u64,
    unit_price: Money,
}

impl Review {
    pub const fn draft(&self) -> &ProvisionalDraft {
        &self.draft
    }

    pub const fn unit_price(&self) -> Money {
        self.unit_price
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Receipt {
    pub draft_id: DraftId,
    pub invoice: u64,
    pub quantity: u32,
    pub total: Money,
}

/// Trusted synthetic fixture. Role selection is not authentication.
pub struct ConfirmationFixture {
    product: ProductId,
    unit_price: Money,
    terms_revision: u64,
    draft_revision: u64,
    drafts: BTreeMap<DraftId, (u64, ProvisionalDraft)>,
    stock: u32,
    ledger: i64,
    next_invoice: u64,
    requests: BTreeMap<RequestId, (Review, Receipt)>,
    posted: BTreeMap<DraftId, RequestId>,
}

impl ConfirmationFixture {
    pub fn new(
        product: ProductId,
        stock: u32,
        unit_price: Money,
    ) -> Result<Self, ConfirmationError> {
        if unit_price.minor_units() < 0 {
            return Err(ConfirmationError::InvalidAmount);
        }
        Ok(Self {
            product,
            unit_price,
            terms_revision: 0,
            draft_revision: 0,
            drafts: BTreeMap::new(),
            stock,
            ledger: 0,
            next_invoice: 1,
            requests: BTreeMap::new(),
            posted: BTreeMap::new(),
        })
    }

    /// Registers the current draft from the trusted fixture authority.
    pub fn register_draft(&mut self, draft: ProvisionalDraft) -> Result<(), ConfirmationError> {
        let revision = self
            .draft_revision
            .checked_add(1)
            .ok_or(ConfirmationError::Overflow)?;
        self.drafts.insert(draft.id(), (revision, draft));
        self.draft_revision = revision;
        Ok(())
    }

    /// Gives Owner a current review of a reopened draft without posting it.
    /// An identical draft keeps its revision; a changed draft replaces it.
    ///
    /// # Errors
    /// Denies non-Owners before registration. Rejects drafts that do not contain
    /// exactly one line for the fixture product, or if a new revision would overflow.
    pub fn reconnect_review(
        &mut self,
        role: Role,
        draft: ProvisionalDraft,
    ) -> Result<Review, ConfirmationError> {
        match role {
            Role::Owner => {}
            Role::Manager => return Err(ConfirmationError::AccessDenied),
        }
        let id = draft.id();
        if self
            .drafts
            .get(&id)
            .is_none_or(|(_, current)| current != &draft)
        {
            self.draft_quantity(&draft)?;
            self.register_draft(draft)?;
        }
        self.review(role, id)
    }

    /// Replaces fixture terms and invalidates all prior reviews, even if the price is unchanged.
    pub fn set_unit_price(&mut self, unit_price: Money) -> Result<(), ConfirmationError> {
        if unit_price.minor_units() < 0 {
            return Err(ConfirmationError::InvalidAmount);
        }
        let revision = self
            .terms_revision
            .checked_add(1)
            .ok_or(ConfirmationError::Overflow)?;
        self.unit_price = unit_price;
        self.terms_revision = revision;
        Ok(())
    }

    pub fn review(&self, role: Role, draft_id: DraftId) -> Result<Review, ConfirmationError> {
        if role != Role::Owner {
            return Err(ConfirmationError::AccessDenied);
        }
        let (revision, draft) = self
            .drafts
            .get(&draft_id)
            .ok_or(ConfirmationError::DraftNotFound)?;
        self.draft_quantity(draft)?;
        Ok(Review {
            draft: draft.clone(),
            draft_revision: *revision,
            terms_revision: self.terms_revision,
            unit_price: self.unit_price,
        })
    }

    fn draft_quantity(&self, draft: &ProvisionalDraft) -> Result<u32, ConfirmationError> {
        match draft.lines() {
            [line] if line.product_id() == self.product => Ok(line.quantity().get()),
            _ => Err(ConfirmationError::InvalidDraft),
        }
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
            || self.unit_price != review.unit_price
        {
            return Err(ConfirmationError::StaleReview);
        }
        let quantity = self.draft_quantity(&review.draft)?;
        let stock = self
            .stock
            .checked_sub(quantity)
            .ok_or(ConfirmationError::InsufficientStock)?;
        let total = review
            .unit_price
            .minor_units()
            .checked_mul(i64::from(quantity))
            .ok_or(ConfirmationError::Overflow)?;
        let ledger = self
            .ledger
            .checked_add(total)
            .ok_or(ConfirmationError::Overflow)?;
        let next_invoice = self
            .next_invoice
            .checked_add(1)
            .ok_or(ConfirmationError::Overflow)?;
        let receipt = Receipt {
            draft_id: id,
            invoice: self.next_invoice,
            quantity,
            total: Money::from_minor_units(total),
        };
        self.stock = stock;
        self.ledger = ledger;
        self.next_invoice = next_invoice;
        self.requests
            .insert(request, (review.clone(), receipt.clone()));
        self.posted.insert(id, request);
        Ok(receipt)
    }

    pub const fn stock(&self) -> u32 {
        self.stock
    }
    pub const fn ledger(&self) -> Money {
        Money::from_minor_units(self.ledger)
    }

    /// Returns the number of receipts posted by the fixture, not counting replays.
    pub fn posted_count(&self) -> usize {
        self.requests.len()
    }
}

#[cfg(test)]
mod tests;
