use std::collections::BTreeMap;
use std::num::NonZeroU32;

use crate::{SaleUnit, crate_row_codec};

use super::{
    DraftId, InvalidStoredText, Money, ProposedPayment, ProvisionalDraft, Review, decode_customer,
    encode_customer,
};
use crate::confirmation::{CheckoutTerms, payment};

/// Crate rows joined by `;;` (a row's own text never holds an empty component).
fn encode_rows(review: &Review) -> String {
    review
        .draft
        .crate_rows()
        .iter()
        .map(crate_row_codec::encode)
        .collect::<Vec<_>>()
        .join(";;")
}

fn decode_rows(text: &str) -> Result<Vec<crate::CrateRow>, InvalidStoredText> {
    text.split(";;").map(crate_row_codec::decode).collect()
}

fn encode_amounts(values: &[Money]) -> String {
    values
        .iter()
        .map(|value| value.minor_units().to_string())
        .collect::<Vec<_>>()
        .join(",")
}

fn encode_packs(review: &Review) -> Option<String> {
    Some(
        review
            .crate_packs
            .as_ref()?
            .iter()
            .map(NonZeroU32::to_string)
            .collect::<Vec<_>>()
            .join(","),
    )
}

fn decode_packs(text: &str) -> Result<Vec<NonZeroU32>, InvalidStoredText> {
    text.split(',')
        .map(|value| value.parse::<NonZeroU32>().map_err(|_| InvalidStoredText))
        .collect()
}

/// A bill of crate rows beside ordinary lines: `10|rows|rates|packs|` before the snapshot of
/// its ordinary lines.
pub(super) fn encode_mixed(review: &Review, inner: impl Fn(&Review) -> String) -> Option<String> {
    let ordinary = review.draft.lines().len();
    if ordinary == 0 || review.draft.crate_rows().is_empty() {
        return None;
    }
    let mut lines_only = review.clone();
    let mut draft = ProvisionalDraft::new(review.draft.id(), review.draft.lines().to_vec())
        .with_customer(review.draft.customer().clone())
        .with_bill_date(review.draft.bill_date())
        .with_loading_rate(review.draft.loading_rate());
    std::mem::swap(&mut lines_only.draft, &mut draft);
    lines_only.prices.truncate(ordinary);
    lines_only.crate_packs = None;
    Some(format!(
        "10|{}|{}|{}|{}",
        encode_rows(review),
        encode_amounts(review.prices.get(ordinary..)?),
        encode_packs(review)?,
        inner(&lines_only)
    ))
}

pub(super) fn decode_mixed(
    snapshot: &str,
    inner: impl Fn(&str) -> Result<Review, InvalidStoredText>,
) -> Result<Review, InvalidStoredText> {
    let invalid = || InvalidStoredText;
    let mut parts = snapshot.splitn(4, '|');
    let (Some(rows), Some(rates), Some(packs), Some(rest)) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(invalid());
    };
    let mut review = inner(rest)?;
    if review.draft.lines().is_empty() || review.crate_packs.is_some() {
        return Err(invalid());
    }
    for row in decode_rows(rows)? {
        review.draft.add_crate_row(row);
    }
    for rate in rates.split(',') {
        let rate = rate.parse::<i64>().ok().filter(|rate| *rate >= 0);
        review
            .prices
            .push(Money::from_minor_units(rate.ok_or_else(invalid)?));
    }
    review.crate_packs = Some(decode_packs(packs)?);
    review.amounts().map_err(|_| invalid())?;
    Ok(review)
}

pub(super) fn encode(review: &Review) -> Option<String> {
    if !review.draft.lines().is_empty() || review.draft.crate_rows().is_empty() {
        return None;
    }
    let terms = review.checkout.as_ref()?;
    let ProposedPayment::Named(received) = review.payment else {
        return None;
    };
    let packs = encode_packs(review)?;
    Some(format!(
        "5|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        review.draft.id().value(),
        review.draft_revision,
        review.terms_revision,
        encode_customer(review.draft.customer()),
        encode_rows(review),
        encode_amounts(&review.prices),
        terms.loading_rate.minor_units(),
        packs,
        received.minor_units(),
    ))
}

pub(super) fn decode(snapshot: &str) -> Result<Review, InvalidStoredText> {
    let invalid = || InvalidStoredText;
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
    let rows = decode_rows(definition)?;
    let rates = rate.split(',').map(amount).collect::<Result<Vec<_>, _>>()?;
    if rates.len() != rows.len()
        || rows
            .iter()
            .zip(&rates)
            .any(|(row, rate)| row.rate().is_some_and(|copied| copied != *rate))
    {
        return Err(invalid());
    }
    let packs = decode_packs(packs)?;
    if rows
        .iter()
        .flat_map(|row| row.components())
        .zip(&packs)
        .any(|(component, &pack)| component.unit == SaleUnit::Case && pack != NonZeroU32::MIN)
    {
        return Err(invalid());
    }
    let mut draft =
        ProvisionalDraft::new(DraftId::new(id.parse().map_err(|_| invalid())?), Vec::new())
            .with_customer(decode_customer(customer)?);
    for row in rows {
        draft.add_crate_row(row);
    }
    let review = Review {
        draft,
        draft_revision: revision.parse().map_err(|_| invalid())?,
        terms_revision: terms_revision.parse().map_err(|_| invalid())?,
        prices: rates,
        payment: ProposedPayment::Named(amount(received)?),
        payment_mode: crate::confirmation::ReceiptMode::Cash,
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
