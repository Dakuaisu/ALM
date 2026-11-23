use super::{DraftStorageError, Money, ProposedPayment, Review, decode_undated};
use crate::confirmation::payment;

pub(super) fn decode(snapshot: &str) -> Result<Review, DraftStorageError> {
    let invalid = DraftStorageError::InvalidOutput;
    let (received, dated) = snapshot.split_once("|6|").ok_or(invalid)?;
    let (date, payload) = dated
        .split_once('|')
        .ok_or(DraftStorageError::InvalidOutput)?;
    if !payload.starts_with("2|") {
        return Err(DraftStorageError::InvalidOutput);
    }
    let date = date.parse().map_err(|_| DraftStorageError::InvalidOutput)?;
    let received = Money::from_minor_units(
        received
            .parse()
            .map_err(|_| DraftStorageError::InvalidOutput)?,
    );
    let mut review = decode_undated(payload)?;
    review.draft = review.draft.with_bill_date(Some(date));
    let total = review
        .amounts()
        .map_err(|_| DraftStorageError::InvalidOutput)?
        .total;
    payment::walk_in_stage(&review.draft, received, total)
        .map_err(|_| DraftStorageError::InvalidOutput)?;
    review.payment = ProposedPayment::WalkInCash(received);
    Ok(review)
}
