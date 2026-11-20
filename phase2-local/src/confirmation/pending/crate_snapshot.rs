use std::collections::BTreeMap;
use std::num::NonZeroU32;

use crate::{SaleUnit, crate_row_codec};

use super::{
    DraftId, DraftStorageError, Money, ProposedPayment, ProvisionalDraft, Review, decode_customer,
    encode_customer,
};
use crate::confirmation::{CheckoutTerms, payment};

pub(super) fn encode(review: &Review) -> Option<String> {
    let row = review.draft.crate_row()?;
    let terms = review.checkout.as_ref()?;
    let [rate] = review.prices.as_slice() else {
        return None;
    };
    let ProposedPayment::Named(received) = review.payment else {
        return None;
    };
    let packs = review
        .crate_packs
        .as_ref()?
        .iter()
        .map(NonZeroU32::to_string)
        .collect::<Vec<_>>()
        .join(",");
    Some(format!(
        "5|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        review.draft.id().value(),
        review.draft_revision,
        review.terms_revision,
        encode_customer(review.draft.customer()),
        crate_row_codec::encode(row),
        rate.minor_units(),
        terms.loading_rate.minor_units(),
        packs,
        received.minor_units(),
    ))
}

pub(super) fn decode(snapshot: &str) -> Result<Review, DraftStorageError> {
    let invalid = || DraftStorageError::InvalidOutput;
    let fields = snapshot.split('|').collect::<Vec<_>>();
    let [
        "5",
        id,
        revision,
        terms_revision,
        customer,
        definition,
        rate,
        loading,
        packs,
        received,
    ] = fields.as_slice()
    else {
        return Err(invalid());
    };
    let amount = |value: &str| {
        value
            .parse::<i64>()
            .ok()
            .filter(|value| *value >= 0)
            .map(Money::from_minor_units)
            .ok_or_else(invalid)
    };
    let row = crate_row_codec::decode(definition)?;
    let rate = amount(rate)?;
    if row.rate().is_some_and(|copied| copied != rate) {
        return Err(invalid());
    }
    let packs = packs
        .split(',')
        .map(|value| value.parse::<NonZeroU32>().map_err(|_| invalid()))
        .collect::<Result<Vec<_>, _>>()?;
    if row
        .components()
        .iter()
        .zip(&packs)
        .any(|(component, &pack)| component.unit == SaleUnit::Case && pack != NonZeroU32::MIN)
    {
        return Err(invalid());
    }
    let mut draft =
        ProvisionalDraft::new(DraftId::new(id.parse().map_err(|_| invalid())?), Vec::new())
            .with_customer(decode_customer(customer)?);
    draft.add_crate_row(row).map_err(|_| invalid())?;
    let review = Review {
        draft,
        draft_revision: revision.parse().map_err(|_| invalid())?,
        terms_revision: terms_revision.parse().map_err(|_| invalid())?,
        prices: vec![rate],
        payment: ProposedPayment::Named(amount(received)?),
        checkout: Some(CheckoutTerms {
            lines: Vec::new(),
            loading_rate: amount(loading)?,
        }),
        crate_packs: Some(packs),
    };
    let amounts = review.amounts().map_err(|_| invalid())?;
    payment::stage(&review, amounts.total.minor_units(), &BTreeMap::new())
        .map_err(|_| invalid())?;
    Ok(review)
}
