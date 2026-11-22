//! Local Receive Payment selection and Save workflow; not a browser event implementation.
use std::time::{SystemTime, UNIX_EPOCH};

use crate::confirmation::{
    ConfirmationError, ConfirmationFixture, CreationKey, CustomerReceiptInput,
    CustomerReceiptMetadata, ReceiptMode,
};
use crate::{BillDate, DraftCustomerId, Money, Role};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Customer {
    pub id: DraftCustomerId,
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SaveError {
    PickCustomer,
    Confirmation(ConfirmationError),
}

impl SaveError {
    pub const fn toast(&self) -> Option<&'static str> {
        match self {
            Self::PickCustomer => Some("Pick a customer"),
            Self::Confirmation(_) => None,
        }
    }
}

/// Focused-input model; sorting is Rust string ordering, not browser collation.
#[derive(Default)]
pub struct ReceivePayment<'a> {
    pub text: &'a str,
    pub suggestions: Vec<&'a Customer>,
    pub highlighted: Option<usize>,
    chosen: Option<&'a Customer>,
    clock: Option<Box<dyn Fn() -> Result<CreationKey, ConfirmationError> + 'a>>,
}

impl<'a> ReceivePayment<'a> {
    /// Uses a per-instance clock sampled only by Save, for deterministic local replay.
    pub fn with_clock(clock: impl Fn() -> Result<CreationKey, ConfirmationError> + 'a) -> Self {
        Self {
            clock: Some(Box::new(clock)),
            ..Self::default()
        }
    }

    pub fn input(&mut self, text: &'a str, customers: &'a [Customer]) {
        self.text = text;
        self.highlighted = None;
        let trimmed = text.trim();
        let query = trimmed.to_lowercase();
        self.suggestions = customers
            .iter()
            .filter(|customer| customer.name.to_lowercase().contains(&query))
            .collect();
        self.suggestions
            .sort_by(|left, right| left.name.cmp(&right.name));
        self.suggestions.truncate(40);
        let mut exact = customers.iter().filter(|customer| customer.name == trimmed);
        self.chosen = match (exact.next(), exact.next()) {
            (Some(customer), None) => Some(customer),
            (None, _) | (Some(_), Some(_)) => None,
        };
    }

    pub fn arrow_down(&mut self) {
        if let Some(last) = self.suggestions.len().checked_sub(1) {
            self.highlighted = Some(self.highlighted.map_or(0, |index| (index + 1).min(last)));
        }
    }

    pub fn enter(&mut self) {
        if let Some(index) = self.highlighted {
            self.pick(index);
        }
    }

    pub fn pick(&mut self, index: usize) {
        if let Some(&customer) = self.suggestions.get(index) {
            self.text = &customer.name;
            self.suggestions.clear();
            self.chosen = Some(customer);
        }
    }

    pub fn selected(&self) -> Option<&Customer> {
        self.chosen
    }

    pub fn save(
        &self,
        role: Role,
        authority: &mut ConfirmationFixture,
        date: BillDate,
        amount: Money,
        mode: ReceiptMode,
    ) -> Result<(), SaveError> {
        if role != Role::Owner {
            return Err(SaveError::Confirmation(ConfirmationError::AccessDenied));
        }
        let customer = self.chosen.ok_or(SaveError::PickCustomer)?;
        if amount.minor_units() == 0 {
            return Err(SaveError::Confirmation(ConfirmationError::InvalidAmount));
        }
        let input = CustomerReceiptInput {
            customer_id: customer.id.clone(),
            date,
            amount,
            mode,
        };
        let metadata = CustomerReceiptMetadata {
            selected_name: customer.name.clone(),
            creation_key: match &self.clock {
                Some(clock) => clock(),
                None => current_creation_key(),
            }
            .map_err(SaveError::Confirmation)?,
        };
        authority
            .record_customer_receipt_with_metadata(role, input, metadata)
            .map_err(SaveError::Confirmation)
    }
}

fn current_creation_key() -> Result<CreationKey, ConfirmationError> {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ConfirmationError::Overflow)?
        .as_millis();
    Ok(CreationKey::new(
        u64::try_from(millis).map_err(|_| ConfirmationError::Overflow)?,
    ))
}
