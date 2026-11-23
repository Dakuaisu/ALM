use super::{
    ConfirmationError, ConfirmationFixture, ProductEntryError, Receipt, ReceiptMode, RequestId,
    Review,
};
use crate::receive_payment::{Customer, ReceivePayment, SaveError};
use crate::{BillDate, DraftCustomerId, InvalidBillDate, Money, ProvisionalDraft, Role};

mod entry;
pub use entry::{customers, fixture, preview, products, select_customer, select_line};

pub const CUSTOMER: &str = "Synthetic Customer";
pub const DAY: &str = "2025-04-30";

pub fn customer_id() -> DraftCustomerId {
    DraftCustomerId::new("batch1-customer".into())
}

#[derive(Debug)]
pub enum FixtureError {
    Confirmation(ConfirmationError),
    Product(ProductEntryError),
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
pub enum TrialBillingError {
    Confirmation(ConfirmationError),
    Date(InvalidBillDate),
    Customer,
    NoSavedDraft,
    NoReview,
    ReviewLimit,
}

impl From<ConfirmationError> for TrialBillingError {
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
        role: Role,
        saved: Option<&ProvisionalDraft>,
        terms: SaleTerms<'_>,
    ) -> Result<(), TrialBillingError> {
        require_owner(role)?;
        self.invalidate();
        let draft = saved.ok_or(TrialBillingError::NoSavedDraft)?;
        let serial = self
            .last_serial
            .checked_add(1)
            .ok_or(TrialBillingError::ReviewLimit)?;
        entry::check_customer(authority, draft.customer())?;
        let date = terms.date.parse().map_err(TrialBillingError::Date)?;
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
    ) -> Result<Receipt, TrialBillingError> {
        require_owner(role)?;
        let pending = self.pending.as_ref().ok_or(TrialBillingError::NoReview)?;
        if serial != pending.serial {
            return Err(ConfirmationError::StaleReview.into());
        }
        let review = pending.review();
        Ok(authority.confirm(role, RequestId::new(review.draft().id().value()), review)?)
    }
}

pub fn save_receipt(
    authority: &mut ConfirmationFixture,
    role: Role,
    terms: ReceiptTerms<'_>,
) -> Result<(), TrialBillingError> {
    require_owner(role)?;
    let date: BillDate = terms.date.parse().map_err(TrialBillingError::Date)?;
    let amount = parse_rupees(terms.amount)?;
    let customers = [Customer {
        id: customer_id(),
        name: CUSTOMER.into(),
    }];
    let mut payment = ReceivePayment::default();
    payment.input(CUSTOMER, &customers);
    payment
        .save(role, authority, date, amount, terms.mode)
        .map_err(|error| match error {
            SaveError::PickCustomer => TrialBillingError::Customer,
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
    let text = text.trim();
    let (whole, fraction) = text.split_once('.').unwrap_or((text, ""));
    if (whole.is_empty() && fraction.is_empty())
        || fraction.len() > 2
        || !whole
            .bytes()
            .chain(fraction.bytes())
            .all(|b| b.is_ascii_digit())
    {
        return Err(ConfirmationError::InvalidAmount);
    }
    let whole = if whole.is_empty() {
        0
    } else {
        whole
            .parse::<i64>()
            .map_err(|_| ConfirmationError::Overflow)?
    };
    let mut digits = fraction.bytes().map(|digit| i64::from(digit - b'0'));
    let paise = 10 * digits.next().unwrap_or(0) + digits.next().unwrap_or(0);
    whole
        .checked_mul(100)
        .and_then(|value| value.checked_add(paise))
        .map(Money::from_minor_units)
        .ok_or(ConfirmationError::Overflow)
}

#[cfg(test)]
#[path = "trial_billing_tests.rs"]
mod tests;
