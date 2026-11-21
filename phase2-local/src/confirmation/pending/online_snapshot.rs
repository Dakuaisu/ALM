use std::collections::BTreeMap;

use super::{DraftStorageError, Review, decode_undated};
use crate::confirmation::{ReceiptMode, payment, posting};

pub(super) fn decode(snapshot: &str) -> Result<Review, DraftStorageError> {
    let invalid = || DraftStorageError::InvalidOutput;
    let dated = snapshot.strip_prefix("6|").ok_or_else(invalid)?;
    let (_, payload) = dated.split_once('|').ok_or_else(invalid)?;
    if !payload.starts_with("3|") {
        return Err(invalid());
    }
    let mut review = decode_dated(dated)?;
    posting::validate_bill(&review.draft).map_err(|_| invalid())?;
    let total = review.amounts().map_err(|_| invalid())?.total.minor_units();
    payment::stage(&review, total, &BTreeMap::new()).map_err(|_| invalid())?;
    review.payment_mode = ReceiptMode::Online;
    Ok(review)
}

pub(super) fn decode_dated(dated: &str) -> Result<Review, DraftStorageError> {
    let (date, payload) = dated
        .split_once('|')
        .ok_or(DraftStorageError::InvalidOutput)?;
    let date = date.parse().map_err(|_| DraftStorageError::InvalidOutput)?;
    let mut review = decode_undated(payload)?;
    review.draft = review.draft.with_bill_date(Some(date));
    Ok(review)
}
