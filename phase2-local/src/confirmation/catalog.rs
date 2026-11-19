use std::collections::BTreeMap;

use super::{ConfirmationError, Money, ProductId, ProvisionalDraft, Review};

#[derive(Clone)]
struct Product {
    stock: u32,
    price: Money,
}

#[derive(Clone)]
pub(super) struct Catalog(BTreeMap<u64, Product>);

impl Catalog {
    pub(super) fn new(id: ProductId, stock: u32, price: Money) -> Self {
        Self(BTreeMap::from([(id.value(), Product { stock, price })]))
    }

    pub(super) fn add(
        &mut self,
        id: ProductId,
        stock: u32,
        price: Money,
    ) -> Result<(), ConfirmationError> {
        if price.minor_units() < 0 {
            return Err(ConfirmationError::InvalidAmount);
        }
        if self.0.contains_key(&id.value()) {
            return Err(ConfirmationError::InvalidDraft);
        }
        self.0.insert(id.value(), Product { stock, price });
        Ok(())
    }

    pub(super) fn set_price(
        &mut self,
        id: ProductId,
        price: Money,
    ) -> Result<(), ConfirmationError> {
        if price.minor_units() < 0 {
            return Err(ConfirmationError::InvalidAmount);
        }
        self.0
            .get_mut(&id.value())
            .ok_or(ConfirmationError::InvalidDraft)?
            .price = price;
        Ok(())
    }

    pub(super) fn stock(&self, id: ProductId) -> Option<u32> {
        self.0.get(&id.value()).map(|product| product.stock)
    }

    pub(super) fn prices(&self, draft: &ProvisionalDraft) -> Result<Vec<Money>, ConfirmationError> {
        if draft.lines().is_empty() {
            return Err(ConfirmationError::InvalidDraft);
        }
        draft
            .lines()
            .iter()
            .map(|line| {
                self.0
                    .get(&line.product_id().value())
                    .map(|p| p.price)
                    .ok_or(ConfirmationError::InvalidDraft)
            })
            .collect()
    }

    pub(super) fn stage(&self, review: &Review) -> Result<(Self, u32, i64), ConfirmationError> {
        let mut staged = self.clone();
        let mut quantity = 0_u32;
        let mut total = 0_i64;
        for (line, price) in review.draft.lines().iter().zip(&review.prices) {
            let product = staged
                .0
                .get_mut(&line.product_id().value())
                .ok_or(ConfirmationError::InvalidDraft)?;
            product.stock = product
                .stock
                .checked_sub(line.quantity().get())
                .ok_or(ConfirmationError::InsufficientStock)?;
            quantity = quantity
                .checked_add(line.quantity().get())
                .ok_or(ConfirmationError::Overflow)?;
            let line_total = price
                .minor_units()
                .checked_mul(i64::from(line.quantity().get()))
                .ok_or(ConfirmationError::Overflow)?;
            total = total
                .checked_add(line_total)
                .ok_or(ConfirmationError::Overflow)?;
        }
        Ok((staged, quantity, total))
    }
}
