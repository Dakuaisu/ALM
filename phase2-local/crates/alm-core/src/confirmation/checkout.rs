use std::collections::BTreeMap;
use std::num::NonZeroU32;

use super::{
    ConfirmationError, ConfirmationFixture, DraftId, Money, ProposedPayment, Review, Role, payment,
};
use crate::ProductId;

use crate::SaleUnit;

mod crate_checkout;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "demo-fixture", derive(serde::Serialize, serde::Deserialize))]
pub struct ProductTerms {
    pub pack: NonZeroU32,
    pub piece_price: Money,
    pub case_price: Money,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "demo-fixture", derive(serde::Serialize, serde::Deserialize))]
pub struct LineTerms {
    pub unit: SaleUnit,
    pub product: ProductTerms,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "demo-fixture", derive(serde::Serialize, serde::Deserialize))]
pub struct CheckoutTerms {
    pub lines: Vec<LineTerms>,
    pub loading_rate: Money,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CheckoutAmounts {
    pub packets: u32,
    pub subtotal: Money,
    pub loading: Money,
    pub total: Money,
}

impl Review {
    pub fn checkout_terms(&self) -> Option<&CheckoutTerms> {
        self.checkout.as_ref()
    }

    pub fn amounts(&self) -> Result<CheckoutAmounts, ConfirmationError> {
        if self.draft.crate_row().is_some() {
            return self.crate_amounts();
        }
        let mut packets = 0_u32;
        let mut subtotal = 0_i64;
        for (index, line) in self.draft.lines().iter().enumerate() {
            let (quantity, price) = self.line_values(index)?;
            packets = packets
                .checked_add(quantity)
                .ok_or(ConfirmationError::Overflow)?;
            subtotal = subtotal
                .checked_add(
                    price
                        .minor_units()
                        .checked_mul(i64::from(line.quantity().get()))
                        .ok_or(ConfirmationError::Overflow)?,
                )
                .ok_or(ConfirmationError::Overflow)?;
        }
        self.calculate_amounts(packets, subtotal)
    }

    fn calculate_amounts(
        &self,
        packets: u32,
        subtotal: i64,
    ) -> Result<CheckoutAmounts, ConfirmationError> {
        let rate = self
            .checkout
            .as_ref()
            .map_or(0, |terms| terms.loading_rate.minor_units());
        let loading = rate
            .checked_mul(i64::from(packets))
            .ok_or(ConfirmationError::Overflow)?;
        let total = subtotal
            .checked_add(loading)
            .ok_or(ConfirmationError::Overflow)?;
        Ok(CheckoutAmounts {
            packets,
            subtotal: Money::from_minor_units(subtotal),
            loading: Money::from_minor_units(loading),
            total: Money::from_minor_units(total),
        })
    }

    pub(super) fn line_values(&self, index: usize) -> Result<(u32, Money), ConfirmationError> {
        let line = self
            .draft
            .lines()
            .get(index)
            .ok_or(ConfirmationError::InvalidDraft)?;
        let qty = line.quantity().get();
        match &self.checkout {
            None => Ok((
                qty,
                *self
                    .prices
                    .get(index)
                    .ok_or(ConfirmationError::InvalidDraft)?,
            )),
            Some(checkout) => {
                let terms = checkout
                    .lines
                    .get(index)
                    .ok_or(ConfirmationError::InvalidDraft)?;
                match terms.unit {
                    SaleUnit::Case => Ok((qty, terms.product.case_price)),
                    SaleUnit::Piece => {
                        let pack = terms.product.pack.get();
                        if !qty.is_multiple_of(pack) {
                            return Err(ConfirmationError::InvalidAmount);
                        }
                        Ok((qty / pack, terms.product.piece_price))
                    }
                }
            }
        }
    }
}

impl ConfirmationFixture {
    pub fn stock(&self) -> u32 {
        self.catalog.stock(self.product).unwrap_or_default()
    }

    pub fn product_stock(&self, product: ProductId) -> Option<u32> {
        self.catalog.stock(product)
    }

    /// Replaces fixture terms and invalidates all prior reviews, even if the price is unchanged.
    pub fn set_unit_price(&mut self, unit_price: Money) -> Result<(), ConfirmationError> {
        self.set_product_price(self.product, unit_price)
    }

    /// Configure synthetic authority-owned terms; changing any terms invalidates reviews.
    pub fn set_checkout_product(
        &mut self,
        product: ProductId,
        terms: ProductTerms,
    ) -> Result<(), ConfirmationError> {
        if self.catalog.stock(product).is_none() {
            return Err(ConfirmationError::InvalidDraft);
        }
        if terms.piece_price.minor_units() < 0 || terms.case_price.minor_units() < 0 {
            return Err(ConfirmationError::InvalidAmount);
        }
        let revision = self
            .terms_revision
            .checked_add(1)
            .ok_or(ConfirmationError::Overflow)?;
        self.checkout_products.insert(product.value(), terms);
        self.terms_revision = revision;
        Ok(())
    }

    pub fn set_loading_rate(&mut self, rate: Money) -> Result<(), ConfirmationError> {
        if rate.minor_units() < 0 {
            return Err(ConfirmationError::InvalidAmount);
        }
        let revision = self
            .terms_revision
            .checked_add(1)
            .ok_or(ConfirmationError::Overflow)?;
        self.loading_rate = rate;
        self.terms_revision = revision;
        Ok(())
    }

    /// Reviews regular lines with their requested units, or a sole crate with an empty `units` slice.
    pub fn review_checkout(
        &mut self,
        role: Role,
        id: DraftId,
        units: &[SaleUnit],
        received: Money,
    ) -> Result<Review, ConfirmationError> {
        if role != Role::Owner {
            return Err(ConfirmationError::AccessDenied);
        }
        let (_, draft) = self
            .drafts
            .get(&id)
            .ok_or(ConfirmationError::DraftNotFound)?;
        super::posting::validate_bill(draft)?;
        if draft.crate_row().is_some() {
            if !units.is_empty() {
                return Err(ConfirmationError::InvalidDraft);
            }
            let review = self.build_crate_review(id, received)?;
            return self.approve_checkout(review);
        }
        let mut review = self.review(role, id)?;
        if units.len() != review.draft.lines().len() {
            return Err(ConfirmationError::InvalidDraft);
        }
        let lines = review
            .draft
            .lines()
            .iter()
            .zip(units)
            .map(|(line, &unit)| {
                let product = *self
                    .checkout_products
                    .get(&line.product_id().value())
                    .ok_or(ConfirmationError::InvalidDraft)?;
                Ok(LineTerms { unit, product })
            })
            .collect::<Result<Vec<_>, ConfirmationError>>()?;
        review.checkout = Some(CheckoutTerms {
            lines,
            loading_rate: self.loading_rate,
        });
        review.payment = ProposedPayment::Named(received);
        self.approve_checkout(review)
    }

    fn approve_checkout(&mut self, review: Review) -> Result<Review, ConfirmationError> {
        payment::stage(
            &review,
            review.amounts()?.total.minor_units(),
            &BTreeMap::new(),
        )?;
        self.checkout_approvals
            .insert(review.draft.id(), review.clone());
        Ok(review)
    }

    pub(super) fn checkout_approved(&self, review: &Review) -> bool {
        match (
            self.checkout_approvals.get(&review.draft.id()),
            &review.checkout,
        ) {
            (None, None) => true,
            (Some(approved), Some(_)) => approved == review,
            (None, Some(_)) | (Some(_), None) => false,
        }
    }

    pub(super) fn checkout_current(&self, review: &Review) -> bool {
        if review.draft.crate_row().is_some() {
            let ProposedPayment::Named(received) = review.payment else {
                return false;
            };
            return self
                .build_crate_review(review.draft.id(), received)
                .as_ref()
                == Ok(review);
        }
        if self.catalog.prices(&review.draft).as_ref() != Ok(&review.prices) {
            return false;
        }
        review.checkout.as_ref().is_none_or(|checkout| {
            checkout.loading_rate == self.loading_rate
                && checkout.lines.len() == review.draft.lines().len()
                && checkout
                    .lines
                    .iter()
                    .zip(review.draft.lines())
                    .all(|(terms, line)| {
                        self.checkout_products.get(&line.product_id().value())
                            == Some(&terms.product)
                    })
        })
    }
}
