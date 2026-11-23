use super::{InvalidStoredText, Money, ProposedPayment, Review, decode_undated};
use crate::confirmation::payment;

pub(super) fn decode(snapshot: &str) -> Result<Review, InvalidStoredText> {
    let invalid = InvalidStoredText;
    let (received, dated) = snapshot.split_once("|6|").ok_or(invalid)?;
    let (date, payload) = dated.split_once('|').ok_or(InvalidStoredText)?;
    if !payload.starts_with("2|") {
        return Err(InvalidStoredText);
    }
    let date = date.parse().map_err(|_| InvalidStoredText)?;
    let received = Money::from_minor_units(received.parse().map_err(|_| InvalidStoredText)?);
    let mut review = decode_undated(payload)?;
    review.draft = review.draft.with_bill_date(Some(date));
    let total = review.amounts().map_err(|_| InvalidStoredText)?.total;
    payment::walk_in_stage(&review.draft, received, total).map_err(|_| InvalidStoredText)?;
    review.payment = ProposedPayment::WalkInCash(received);
    Ok(review)
}
