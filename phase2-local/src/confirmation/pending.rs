use std::fmt;
use std::num::NonZeroU32;

use super::{ProposedPayment, RequestId, Review};
use crate::{
    DraftCustomer, DraftCustomerId, DraftId, DraftLine, DraftStorageError, DraftStore, Money,
    ProductId, ProvisionalDraft, Role, SalesSession,
};

/// Local precondition, authorization or persistence failure, not an authority outcome.
#[derive(Debug)]
pub enum PendingError {
    AccessDenied,
    ActiveDraftMismatch,
    DurableStoreRequired,
    NotFound,
    Storage(DraftStorageError),
}

impl fmt::Display for PendingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AccessDenied => f.write_str("Owner access required"),
            Self::ActiveDraftMismatch => f.write_str("approved draft is not the active snapshot"),
            Self::DurableStoreRequired => f.write_str("SQLite store required"),
            Self::NotFound => f.write_str("pending approval not found"),
            Self::Storage(error) => write!(f, "pending approval storage: {error}"),
        }
    }
}

impl std::error::Error for PendingError {}

impl From<DraftStorageError> for PendingError {
    fn from(value: DraftStorageError) -> Self {
        Self::Storage(value)
    }
}

impl SalesSession<'_> {
    /// Commits the immutable approval before invoking the sender. Duplicate IDs fail.
    /// Pending approvals are retained after a response; no receipt is stored.
    ///
    /// # Errors
    /// Denies Manager and fails without calling the sender if persistence fails.
    pub fn confirm_pending<T>(
        &mut self,
        id: RequestId,
        review: &Review,
        send: impl FnOnce(RequestId, &Review) -> T,
    ) -> Result<T, PendingError> {
        if self.role != Role::Owner {
            return Err(PendingError::AccessDenied);
        }
        let DraftStore::Sqlite(store) = &self.sales.drafts else {
            return Err(PendingError::DurableStoreRequired);
        };
        store.save_pending(id, &encode(review))?;
        Ok(send(id, review))
    }

    /// Explicitly sends the original stored approval; opening a store never sends.
    ///
    /// # Errors
    /// Denies Manager, missing approvals, and invalid or unavailable storage.
    pub fn retry_pending<T>(
        &self,
        id: RequestId,
        send: impl FnOnce(RequestId, &Review) -> T,
    ) -> Result<T, PendingError> {
        if self.role != Role::Owner {
            return Err(PendingError::AccessDenied);
        }
        let DraftStore::Sqlite(store) = &self.sales.drafts else {
            return Err(PendingError::DurableStoreRequired);
        };
        let snapshot = store.load_pending(id)?.ok_or(PendingError::NotFound)?;
        let review = decode(&snapshot)?;
        Ok(send(id, &review))
    }
}

mod cash_snapshot;
mod checkout_snapshot;
mod crate_snapshot;

fn hex(text: &str) -> String {
    use std::fmt::Write;
    let mut result = String::new();
    for byte in text.bytes() {
        let _ = write!(result, "{byte:02X}");
    }
    result
}

fn unhex(text: &str) -> Result<String, DraftStorageError> {
    let bytes = text.as_bytes();
    if !bytes.len().is_multiple_of(2) {
        return Err(DraftStorageError::InvalidOutput);
    }
    let mut result = Vec::new();
    for pair in bytes.as_chunks::<2>().0 {
        let digits = std::str::from_utf8(pair).map_err(|_| DraftStorageError::InvalidOutput)?;
        result.push(u8::from_str_radix(digits, 16).map_err(|_| DraftStorageError::InvalidOutput)?);
    }
    String::from_utf8(result).map_err(|_| DraftStorageError::InvalidOutput)
}

fn encode_customer(customer: &DraftCustomer) -> String {
    match customer {
        DraftCustomer::Unassigned => "0,,".to_owned(),
        DraftCustomer::Typed(name) => format!("1,,{}", hex(name)),
        DraftCustomer::Selected { id, name } => format!("2,{},{}", hex(id.value()), hex(name)),
    }
}

fn decode_customer(customer: &str) -> Result<DraftCustomer, DraftStorageError> {
    let parts: Vec<_> = customer.split(',').collect();
    Ok(match parts.as_slice() {
        ["0", "", ""] => DraftCustomer::Unassigned,
        ["1", "", name] => DraftCustomer::Typed(unhex(name)?),
        ["2", id, name] => DraftCustomer::Selected {
            id: DraftCustomerId::new(unhex(id)?),
            name: unhex(name)?,
        },
        _ => return Err(DraftStorageError::InvalidOutput),
    })
}

pub(super) fn encode(review: &Review) -> String {
    let payload = encode_undated(review);
    let dated = match review.draft.bill_date() {
        Some(date) => format!("6|{date}|{payload}"),
        None => payload,
    };
    match review.payment {
        ProposedPayment::WalkInCash(received) => format!("7|{}|{dated}", received.minor_units()),
        ProposedPayment::Legacy | ProposedPayment::Named(_) => dated,
    }
}

fn encode_undated(review: &Review) -> String {
    if let Some(snapshot) = crate_snapshot::encode(review) {
        return snapshot;
    }
    let customer = encode_customer(review.draft.customer());
    let lines = review
        .draft
        .lines()
        .iter()
        .map(|line| format!("{},{}", line.product_id().value(), line.quantity()))
        .collect::<Vec<_>>()
        .join(";");
    let prices = review
        .prices
        .iter()
        .map(|price| price.minor_units().to_string())
        .collect::<Vec<_>>()
        .join(",");
    let base = format!(
        "2|{}|{}|{}|{}|{}|{}",
        review.draft.id().value(),
        review.draft_revision,
        review.terms_revision,
        prices,
        customer,
        lines
    );
    let base = match review.payment {
        ProposedPayment::Legacy | ProposedPayment::WalkInCash(_) => base,
        ProposedPayment::Named(amount) => format!("3{}|{}", &base[1..], amount.minor_units()),
    };
    match &review.checkout {
        None => base,
        Some(terms) => checkout_snapshot::encode(&base, terms),
    }
}

pub(super) fn decode(snapshot: &str) -> Result<Review, DraftStorageError> {
    if let Some(cash) = snapshot.strip_prefix("7|") {
        return cash_snapshot::decode(cash);
    }
    let Some(dated) = snapshot.strip_prefix("6|") else {
        return decode_undated(snapshot);
    };
    let (date, payload) = dated
        .split_once('|')
        .ok_or(DraftStorageError::InvalidOutput)?;
    let date = date.parse().map_err(|_| DraftStorageError::InvalidOutput)?;
    let mut review = decode_undated(payload)?;
    review.draft = review.draft.with_bill_date(Some(date));
    Ok(review)
}

fn decode_undated(snapshot: &str) -> Result<Review, DraftStorageError> {
    let invalid = || DraftStorageError::InvalidOutput;
    if snapshot.starts_with("4|") {
        return checkout_snapshot::decode(snapshot);
    }
    if snapshot.starts_with("5|") {
        return crate_snapshot::decode(snapshot);
    }
    let mut fields: Vec<_> = snapshot.split('|').collect();
    let payment = if fields.first() == Some(&"3") {
        let value = fields
            .pop()
            .ok_or_else(invalid)?
            .parse()
            .map_err(|_| invalid())?;
        ProposedPayment::Named(Money::from_minor_units(value))
    } else {
        ProposedPayment::Legacy
    };
    let [
        version,
        id,
        draft_revision,
        terms_revision,
        price,
        customer,
        lines,
    ] = fields.as_slice()
    else {
        return Err(invalid());
    };
    if *version != "1" && *version != "2" && *version != "3" {
        return Err(invalid());
    }
    let customer = decode_customer(customer)?;
    let mut decoded = Vec::new();
    if !lines.is_empty() {
        for line in lines.split(';') {
            let (product, quantity) = line.split_once(',').ok_or_else(invalid)?;
            decoded.push(DraftLine::new(
                ProductId::new(product.parse().map_err(|_| invalid())?),
                quantity.parse::<NonZeroU32>().map_err(|_| invalid())?,
            ));
        }
    }
    let prices = if *version == "1" {
        let value = Money::from_minor_units(price.parse().map_err(|_| invalid())?);
        vec![value; decoded.len()]
    } else {
        price
            .split(',')
            .map(|value| {
                value
                    .parse()
                    .map(Money::from_minor_units)
                    .map_err(|_| invalid())
            })
            .collect::<Result<Vec<_>, _>>()?
    };
    if decoded.is_empty()
        || prices.len() != decoded.len()
        || prices.iter().any(|price| price.minor_units() < 0)
    {
        return Err(invalid());
    }
    Ok(Review {
        draft: ProvisionalDraft::new(DraftId::new(id.parse().map_err(|_| invalid())?), decoded)
            .with_customer(customer),
        draft_revision: draft_revision.parse().map_err(|_| invalid())?,
        terms_revision: terms_revision.parse().map_err(|_| invalid())?,
        prices,
        payment,
        checkout: None,
        crate_packs: None,
    })
}

#[cfg(test)]
mod tests;
