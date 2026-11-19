//! Synthetic in-memory confirmation fixture, not a production posting authority.
use std::collections::BTreeMap;
use std::num::NonZeroU32;

use crate::{DraftId, Money, ProductId, ProvisionalDraft, Role};

mod catalog;
mod checkout;
mod edit;
mod payment;
mod pending;
mod posting;
pub use crate::SaleUnit;
use catalog::Catalog;
pub use checkout::{CheckoutAmounts, CheckoutTerms, LineTerms, ProductTerms};
pub use edit::{EditChanges, SaleEdit};
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
    MissingBillableItems,
    MissingCustomerName,
    MissingBillDate,
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
    checkout: Option<CheckoutTerms>,
    crate_packs: Option<Vec<NonZeroU32>>,
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

/// An authority-issued receipt bound to the request and the exact approved review.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfirmationAck {
    request: RequestId,
    review: Review,
    receipt: Receipt,
}

/// Trusted synthetic fixture. Role selection is not authentication.
#[cfg_attr(test, derive(Clone, Debug, Eq, PartialEq))]
pub struct ConfirmationFixture {
    product: ProductId,
    catalog: Catalog,
    checkout_products: BTreeMap<u64, ProductTerms>,
    checkout_approvals: BTreeMap<DraftId, Review>,
    loading_rate: Money,
    crate_rate: Option<Money>,
    terms_revision: u64,
    draft_revision: u64,
    drafts: BTreeMap<DraftId, (u64, ProvisionalDraft)>,
    ledger: i64,
    received_ledger: i64,
    customer_balances: BTreeMap<String, i64>,
    next_invoice: u64,
    requests: BTreeMap<RequestId, (Review, Receipt)>,
    posted: BTreeMap<DraftId, RequestId>,
    current_sales: BTreeMap<DraftId, (Review, Receipt, u64)>,
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
            checkout_products: BTreeMap::new(),
            checkout_approvals: BTreeMap::new(),
            loading_rate: Money::from_minor_units(0),
            crate_rate: None,
            terms_revision: 0,
            draft_revision: 0,
            drafts: BTreeMap::new(),
            ledger: 0,
            received_ledger: 0,
            customer_balances: BTreeMap::new(),
            next_invoice: 1,
            requests: BTreeMap::new(),
            posted: BTreeMap::new(),
            current_sales: BTreeMap::new(),
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
            checkout: None,
            crate_packs: None,
        })
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
