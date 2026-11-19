use std::fmt;
use std::num::NonZeroU32;

use super::{RequestId, Review};
use crate::{
    DraftCustomer, DraftCustomerId, DraftId, DraftLine, DraftStorageError, DraftStore, Money,
    ProductId, ProvisionalDraft, Role, SalesSession,
};

/// Local persistence and authorization failures, never an authority receipt.
#[derive(Debug)]
pub enum PendingError {
    AccessDenied,
    DurableStoreRequired,
    NotFound,
    Storage(DraftStorageError),
}

impl fmt::Display for PendingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AccessDenied => f.write_str("Owner access required"),
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

fn encode(review: &Review) -> String {
    let customer = match review.draft.customer() {
        DraftCustomer::Unassigned => "0,,".to_owned(),
        DraftCustomer::Typed(name) => format!("1,,{}", hex(name)),
        DraftCustomer::Selected { id, name } => format!("2,{},{}", hex(id.value()), hex(name)),
    };
    let lines = review
        .draft
        .lines()
        .iter()
        .map(|line| format!("{},{}", line.product_id().value(), line.quantity()))
        .collect::<Vec<_>>()
        .join(";");
    format!(
        "1|{}|{}|{}|{}|{}|{}",
        review.draft.id().value(),
        review.draft_revision,
        review.terms_revision,
        review.unit_price.minor_units(),
        customer,
        lines
    )
}

fn decode(snapshot: &str) -> Result<Review, DraftStorageError> {
    let invalid = || DraftStorageError::InvalidOutput;
    let fields: Vec<_> = snapshot.split('|').collect();
    let [
        "1",
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
    let parts: Vec<_> = customer.split(',').collect();
    let customer = match parts.as_slice() {
        ["0", "", ""] => DraftCustomer::Unassigned,
        ["1", "", name] => DraftCustomer::Typed(unhex(name)?),
        ["2", id, name] => DraftCustomer::Selected {
            id: DraftCustomerId::new(unhex(id)?),
            name: unhex(name)?,
        },
        _ => return Err(invalid()),
    };
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
    Ok(Review {
        draft: ProvisionalDraft::new(DraftId::new(id.parse().map_err(|_| invalid())?), decoded)
            .with_customer(customer),
        draft_revision: draft_revision.parse().map_err(|_| invalid())?,
        terms_revision: terms_revision.parse().map_err(|_| invalid())?,
        unit_price: Money::from_minor_units(price.parse().map_err(|_| invalid())?),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_round_trips_customer_variants_and_ordered_lines() {
        for customer in [
            DraftCustomer::Unassigned,
            DraftCustomer::Typed("नाम | ' \0\n".into()),
            DraftCustomer::Selected {
                id: DraftCustomerId::new("id | \0".into()),
                name: "客户 ; ' \n".into(),
            },
        ] {
            let review = Review {
                draft: ProvisionalDraft::new(
                    DraftId::new(u64::MAX),
                    vec![
                        DraftLine::new(ProductId::new(987), NonZeroU32::new(13).unwrap()),
                        DraftLine::new(ProductId::new(12), NonZeroU32::new(7).unwrap()),
                    ],
                )
                .with_customer(customer),
                draft_revision: u64::MAX,
                terms_revision: 39,
                unit_price: Money::from_minor_units(223),
            };
            assert_eq!(decode(&encode(&review)).unwrap(), review);
        }
    }
}
