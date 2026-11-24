//! Owner billing rules: customer/product selection, priced review, the serialised Owner
//! posting review and standalone receipts. Callers supply the directory and dates.
use super::{ConfirmationError, ConfirmationFixture, Receipt, ReceiptMode, RequestId, Review};
use crate::receive_payment::{Customer, ReceivePayment, SaveError};
use crate::{BillDate, DraftCustomerId, InvalidBillDate, Money, ProvisionalDraft, Role};

mod customer_book;
mod directory;
pub use customer_book::{BalanceSide, CustomerBook, CustomerBookRow, CustomerFilter, is_debtor};
pub use directory::{BillingDirectory, DirectoryCustomer, DirectoryProduct};

/// One Owner-priced line of a saved draft: the catalog sell rate times the draft quantity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PricedLine {
    pub product_id: crate::ProductId,
    pub quantity: std::num::NonZeroU32,
    pub unit_price: Money,
    pub amount: Money,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PricedDraft {
    pub lines: Vec<PricedLine>,
    /// The draft's per-packet loading rate times its packets.
    pub loading: Money,
    /// Line amounts plus loading.
    pub total: Money,
}

/// Owner-only line amounts and total for the review screen, with checked arithmetic.
pub fn priced_draft(
    authority: &ConfirmationFixture,
    role: Role,
    draft: &ProvisionalDraft,
) -> Result<PricedDraft, ConfirmationError> {
    let prices = preview_unit_prices(authority, role, draft)?;
    let mut total = 0_i64;
    let mut lines = Vec::with_capacity(prices.len());
    for (line, unit_price) in draft.lines().iter().zip(prices) {
        let amount = unit_price
            .minor_units()
            .checked_mul(i64::from(line.quantity().get()))
            .ok_or(ConfirmationError::Overflow)?;
        total = total
            .checked_add(amount)
            .ok_or(ConfirmationError::Overflow)?;
        lines.push(PricedLine {
            product_id: line.product_id(),
            quantity: line.quantity(),
            unit_price,
            amount: Money::from_minor_units(amount),
        });
    }
    let loading = crate::provisional_preview::loading_charge(draft)
        .map_err(|_| ConfirmationError::Overflow)?;
    let total = total
        .checked_add(loading.minor_units())
        .ok_or(ConfirmationError::Overflow)?;
    Ok(PricedDraft {
        lines,
        loading,
        total: Money::from_minor_units(total),
    })
}

pub fn preview_unit_prices(
    authority: &ConfirmationFixture,
    role: Role,
    draft: &ProvisionalDraft,
) -> Result<Vec<Money>, ConfirmationError> {
    require_owner(role)?;
    authority.catalog.prices(draft)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OwnerBillingError {
    Confirmation(ConfirmationError),
    Date(InvalidBillDate),
    Customer,
    NoSavedDraft,
    NoReview,
    ReviewLimit,
}

impl From<ConfirmationError> for OwnerBillingError {
    fn from(error: ConfirmationError) -> Self {
        Self::Confirmation(error)
    }
}

pub struct SaleTerms<'a> {
    pub date: &'a str,
    pub paid: &'a str,
}

pub struct ReceiptTerms<'a> {
    pub date: &'a str,
    pub amount: &'a str,
    pub mode: ReceiptMode,
}

#[derive(Default)]
pub struct OwnerPosting {
    last_serial: u64,
    pending: Option<PostingReview>,
}

pub struct PostingReview {
    serial: u64,
    review: Review,
}

impl PostingReview {
    pub const fn serial(&self) -> u64 {
        self.serial
    }

    pub const fn review(&self) -> &Review {
        &self.review
    }
}

impl OwnerPosting {
    pub fn invalidate(&mut self) {
        self.pending = None;
    }

    pub const fn pending(&self, role: Role) -> Option<&PostingReview> {
        match role {
            Role::Owner => self.pending.as_ref(),
            Role::Manager => None,
        }
    }

    pub fn prepare(
        &mut self,
        authority: &mut ConfirmationFixture,
        directory: &BillingDirectory,
        role: Role,
        saved: Option<&ProvisionalDraft>,
        terms: SaleTerms<'_>,
    ) -> Result<(), OwnerBillingError> {
        require_owner(role)?;
        self.invalidate();
        let draft = saved.ok_or(OwnerBillingError::NoSavedDraft)?;
        let serial = self
            .last_serial
            .checked_add(1)
            .ok_or(OwnerBillingError::ReviewLimit)?;
        directory.check_customer(authority, draft.customer())?;
        let date = terms.date.parse().map_err(OwnerBillingError::Date)?;
        let paid = parse_rupees(terms.paid)?;
        let selected = draft.clone().with_bill_date(Some(date));
        super::posting::validate_bill(&selected)?;
        let existing = authority
            .drafts
            .get(&selected.id())
            .filter(|(_, current)| current == &selected);
        let revision = match existing {
            Some((revision, _)) => *revision,
            None => authority
                .draft_revision
                .checked_add(1)
                .ok_or(ConfirmationError::Overflow)?,
        };
        let review = authority
            .review_draft(&selected, revision)?
            .with_named_payment(paid)?;
        if existing.is_none() {
            authority.register_draft(selected)?;
        }
        self.pending = Some(PostingReview { serial, review });
        self.last_serial = serial;
        Ok(())
    }

    pub fn confirm(
        &self,
        authority: &mut ConfirmationFixture,
        role: Role,
        serial: u64,
    ) -> Result<Receipt, OwnerBillingError> {
        require_owner(role)?;
        let pending = self.pending.as_ref().ok_or(OwnerBillingError::NoReview)?;
        if serial != pending.serial {
            return Err(ConfirmationError::StaleReview.into());
        }
        let review = pending.review();
        Ok(authority.confirm(role, RequestId::new(review.draft().id().value()), review)?)
    }
}

/// Records a standalone receipt for a directory customer, dated independently of any bill.
pub fn save_receipt(
    authority: &mut ConfirmationFixture,
    directory: &BillingDirectory,
    role: Role,
    customer: &str,
    terms: ReceiptTerms<'_>,
) -> Result<(), OwnerBillingError> {
    save_receipt_with(authority, directory, role, customer, terms, None)
}

/// [`save_receipt`] stamped with the authority's `creation_key` (milliseconds) instead of the
/// wall clock, so a journaled authority replays the same receipt order.
///
/// # Errors
/// As [`save_receipt`].
pub fn save_receipt_at(
    authority: &mut ConfirmationFixture,
    directory: &BillingDirectory,
    role: Role,
    customer: &str,
    terms: ReceiptTerms<'_>,
    creation_key: u64,
) -> Result<(), OwnerBillingError> {
    save_receipt_with(
        authority,
        directory,
        role,
        customer,
        terms,
        Some(creation_key),
    )
}

fn save_receipt_with(
    authority: &mut ConfirmationFixture,
    directory: &BillingDirectory,
    role: Role,
    customer: &str,
    terms: ReceiptTerms<'_>,
    creation_key: Option<u64>,
) -> Result<(), OwnerBillingError> {
    require_owner(role)?;
    let date: BillDate = terms.date.parse().map_err(OwnerBillingError::Date)?;
    let amount = parse_rupees(terms.amount)?;
    let chosen = directory.customer(authority, customer)?;
    let customers = [Customer {
        id: DraftCustomerId::new(chosen.id.into()),
        name: chosen.name.into(),
    }];
    let mut payment = match creation_key {
        Some(key) => ReceivePayment::with_clock(move || Ok(super::CreationKey::new(key))),
        None => ReceivePayment::default(),
    };
    payment.input(chosen.name, &customers);
    payment
        .save(role, authority, date, amount, terms.mode)
        .map_err(|error| match error {
            SaveError::PickCustomer => OwnerBillingError::Customer,
            SaveError::Confirmation(error) => error.into(),
        })
}

const fn require_owner(role: Role) -> Result<(), ConfirmationError> {
    match role {
        Role::Owner => Ok(()),
        Role::Manager => Err(ConfirmationError::AccessDenied),
    }
}

fn parse_rupees(text: &str) -> Result<Money, ConfirmationError> {
    Money::parse_rupees(text).map_err(|error| match error {
        crate::RupeesError::Invalid => ConfirmationError::InvalidAmount,
        crate::RupeesError::Overflow => ConfirmationError::Overflow,
    })
}

#[cfg(test)]
#[path = "owner_billing_tests.rs"]
mod tests;
