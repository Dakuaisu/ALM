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

/// One crate row of a saved draft: its name, crates, rate and amount (`lineAmt`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PricedCrate {
    pub name: String,
    pub quantity: std::num::NonZeroU32,
    pub rate: Money,
    pub amount: Money,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PricedDraft {
    pub lines: Vec<PricedLine>,
    /// Crate rows beside the ordinary lines, in order.
    pub crates: Vec<PricedCrate>,
    /// The draft's per-packet loading rate times its packets (crate components included).
    pub loading: Money,
    /// Line and crate amounts plus loading.
    pub total: Money,
}

/// Owner-only line and crate amounts and total for the review screen, with checked arithmetic.
pub fn priced_draft(
    authority: &ConfirmationFixture,
    role: Role,
    draft: &ProvisionalDraft,
) -> Result<PricedDraft, ConfirmationError> {
    require_owner(role)?;
    let (prices, crate_packs) = authority.sale_terms(draft)?;
    let mut lines = Vec::with_capacity(draft.lines().len());
    for (line, &unit_price) in draft.lines().iter().zip(&prices) {
        let amount = unit_price
            .minor_units()
            .checked_mul(i64::from(line.quantity().get()))
            .ok_or(ConfirmationError::Overflow)?;
        lines.push(PricedLine {
            product_id: line.product_id(),
            quantity: line.quantity(),
            unit_price,
            amount: Money::from_minor_units(amount),
        });
    }
    let mut crates = Vec::with_capacity(draft.crate_rows().len());
    for (row, &rate) in draft
        .crate_rows()
        .iter()
        .zip(&prices[draft.lines().len()..])
    {
        let amount = rate
            .minor_units()
            .checked_mul(i64::from(row.quantity().get()))
            .ok_or(ConfirmationError::Overflow)?;
        crates.push(PricedCrate {
            name: row.name().unwrap_or_default().to_owned(),
            quantity: row.quantity(),
            rate,
            amount: Money::from_minor_units(amount),
        });
    }
    let review = Review {
        draft: draft.clone(),
        draft_revision: 0,
        terms_revision: 0,
        prices,
        payment: super::ProposedPayment::Legacy,
        payment_mode: ReceiptMode::Cash,
        checkout: None,
        crate_packs,
    };
    let amounts = review.amounts()?;
    Ok(PricedDraft {
        lines,
        crates,
        loading: amounts.loading,
        total: amounts.total,
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

/// Prepares and confirms a saved sale in one step, as the authority's `post_sale` does:
/// nothing changes when it is refused (a draft registered by the review is taken back).
///
/// # Errors
/// As [`OwnerPosting::prepare`] and [`OwnerPosting::confirm`].
pub fn post_saved_sale(
    authority: &mut ConfirmationFixture,
    directory: &BillingDirectory,
    role: Role,
    draft: &ProvisionalDraft,
    terms: SaleTerms<'_>,
) -> Result<Receipt, OwnerBillingError> {
    let id = draft.id();
    let before = (authority.drafts.get(&id).cloned(), authority.draft_revision);
    let mut posting = OwnerPosting::default();
    let result = posting
        .prepare(authority, directory, role, Some(draft), terms)
        .and_then(|()| {
            let serial = posting
                .pending(role)
                .map(PostingReview::serial)
                .ok_or(OwnerBillingError::NoReview)?;
            posting.confirm(authority, role, serial)
        });
    if result.is_err() {
        match before.0 {
            Some(previous) => {
                authority.drafts.insert(id, previous);
            }
            None => {
                authority.drafts.remove(&id);
            }
        }
        authority.draft_revision = before.1;
    }
    result
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

/// A crate sold from a saved template: the draft identity, the template and its count, the
/// directory customer, the bill date and the amount paid.
#[derive(Clone, Debug)]
pub struct CrateSaleTerms<'a> {
    pub draft: crate::DraftId,
    pub template: super::CrateTemplateId,
    pub quantity: std::num::NonZeroU32,
    pub customer: &'a crate::DraftCustomer,
    pub date: BillDate,
    pub paid: Money,
}

/// Posts a crate sale (`dispatchCrate`): the template is copied as a bill of one crate row, each
/// component's packets are taken from its FIFO lots as a separate demand (a repeated product
/// takes from its lots once per component), and the bill is reviewed at the template's rate
/// with the authority's loading charge. Nothing changes on any rejection.
///
/// # Errors
/// Denies Managers; rejects unknown customers or templates, piece components without
/// checkout terms, paid amounts above the total, stock shortages and already posted drafts.
pub fn sell_crate(
    authority: &mut ConfirmationFixture,
    directory: &BillingDirectory,
    role: Role,
    terms: &CrateSaleTerms<'_>,
) -> Result<Receipt, OwnerBillingError> {
    require_owner(role)?;
    if authority.current_sale(role, terms.draft).is_ok() {
        return Err(ConfirmationError::DraftAlreadyPosted.into());
    }
    directory.check_customer(authority, terms.customer)?;
    let mut draft = ProvisionalDraft::new(terms.draft, Vec::new())
        .with_customer(terms.customer.clone())
        .with_bill_date(Some(terms.date));
    authority
        .load_crate_template(role, terms.template, terms.quantity, &mut draft)
        .map_err(|_| ConfirmationError::InvalidDraft)?;
    authority.register_draft(draft)?;
    let review = authority.review_checkout(role, terms.draft, &[], terms.paid)?;
    Ok(authority.confirm(role, RequestId::new(terms.draft.value()), &review)?)
}
