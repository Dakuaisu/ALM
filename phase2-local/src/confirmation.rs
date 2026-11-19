//! Synthetic in-memory confirmation fixture, not a production posting authority.
use std::collections::BTreeMap;

use crate::{DraftId, Money, ProductId, ProvisionalDraft, Role};

mod catalog;
mod payment;
mod pending;
use catalog::Catalog;
pub use payment::ProposedPayment;
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
    prices: Vec<Money>,
    payment: ProposedPayment,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Receipt {
    pub draft_id: DraftId,
    pub invoice: u64,
    pub quantity: u32,
    pub total: Money,
    pub received: Option<Money>,
    pub due: Option<Money>,
    pub customer_balance: Option<Money>,
}

/// Trusted synthetic fixture. Role selection is not authentication.
pub struct ConfirmationFixture {
    product: ProductId,
    catalog: Catalog,
    terms_revision: u64,
    draft_revision: u64,
    drafts: BTreeMap<DraftId, (u64, ProvisionalDraft)>,
    ledger: i64,
    received_ledger: i64,
    customer_balances: BTreeMap<String, i64>,
    next_invoice: u64,
    requests: BTreeMap<RequestId, (Review, Receipt)>,
    posted: BTreeMap<DraftId, RequestId>,
}

impl ConfirmationFixture {
    /// Creates an authority with synthetic lots in trusted oldest-first order.
    ///
    /// # Errors
    /// Rejects negative amounts or a total stock exceeding the quantity range.
    pub fn with_ordered_lots(
        product: ProductId,
        lots: &[(u32, Money)],
        unit_price: Money,
    ) -> Result<Self, ConfirmationError> {
        let mut fixture = Self::new(product, 0, unit_price)?;
        fixture.catalog.seed_lots(product, lots)?;
        Ok(fixture)
    }

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
            catalog: Catalog::new(product, stock, unit_price),
            terms_revision: 0,
            draft_revision: 0,
            drafts: BTreeMap::new(),
            ledger: 0,
            received_ledger: 0,
            customer_balances: BTreeMap::new(),
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
    /// known, nonempty product lines, or if a new revision would overflow.
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
            self.catalog.prices(&draft)?;
            self.register_draft(draft)?;
        }
        self.review(role, id)
    }

    /// Replaces fixture terms and invalidates all prior reviews, even if the price is unchanged.
    pub fn set_unit_price(&mut self, unit_price: Money) -> Result<(), ConfirmationError> {
        self.set_product_price(self.product, unit_price)
    }

    /// Adds a distinct synthetic product to the fixture catalog.
    pub fn add_product(
        &mut self,
        product: ProductId,
        stock: u32,
        price: Money,
    ) -> Result<(), ConfirmationError> {
        let revision = self
            .terms_revision
            .checked_add(1)
            .ok_or(ConfirmationError::Overflow)?;
        self.catalog.add(product, stock, price)?;
        self.terms_revision = revision;
        Ok(())
    }

    /// Changes the price for a synthetic product and invalidates prior reviews.
    pub fn set_product_price(
        &mut self,
        product: ProductId,
        price: Money,
    ) -> Result<(), ConfirmationError> {
        let revision = self
            .terms_revision
            .checked_add(1)
            .ok_or(ConfirmationError::Overflow)?;
        self.catalog.set_price(product, price)?;
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
        let prices = self.catalog.prices(draft)?;
        Ok(Review {
            draft: draft.clone(),
            draft_revision: *revision,
            terms_revision: self.terms_revision,
            prices,
            payment: ProposedPayment::Legacy,
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
            || self.catalog.prices(&review.draft).ok().as_ref() != Some(&review.prices)
        {
            return Err(ConfirmationError::StaleReview);
        }
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
        self.requests
            .insert(request, (review.clone(), receipt.clone()));
        self.posted.insert(id, request);
        Ok(receipt)
    }

    pub fn stock(&self) -> u32 {
        self.catalog.stock(self.product).unwrap_or_default()
    }
    pub fn product_stock(&self, product: ProductId) -> Option<u32> {
        self.catalog.stock(product)
    }
    pub const fn ledger(&self) -> Money {
        Money::from_minor_units(self.ledger)
    }
    pub const fn received_ledger(&self) -> Money {
        Money::from_minor_units(self.received_ledger)
    }

    /// Returns the number of receipts posted by the fixture, not counting replays.
    pub fn posted_count(&self) -> usize {
        self.requests.len()
    }
}

#[cfg(test)]
mod tests;
