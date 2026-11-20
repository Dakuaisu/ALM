use std::collections::BTreeSet;
use std::num::NonZeroU32;

use crate::{DraftLine, ProductId, SaleUnit};

use super::{
    CheckoutAmounts, CheckoutTerms, ConfirmationError, ConfirmationFixture, DraftId, Money,
    ProposedPayment, Review,
};

impl Review {
    /// Returns the packet demand of each component in its original order.
    pub fn crate_packet_demands(&self) -> Result<Option<Vec<DraftLine>>, ConfirmationError> {
        let Some(row) = self.draft.crate_row() else {
            return Ok(None);
        };
        let packs = self
            .crate_packs
            .as_deref()
            .ok_or(ConfirmationError::InvalidDraft)?;
        if !self.draft.lines().is_empty()
            || row.components().is_empty()
            || row.components().len() != packs.len()
        {
            return Err(ConfirmationError::InvalidDraft);
        }
        let mut seen = BTreeSet::new();
        row.components()
            .iter()
            .zip(packs)
            .map(|(component, pack)| {
                if !seen.insert(component.product.value()) {
                    return Err(ConfirmationError::InvalidDraft);
                }
                let quantity =
                    u64::from(component.quantity.get()) * u64::from(row.quantity().get());
                let divisor = match component.unit {
                    SaleUnit::Piece => u64::from(pack.get()),
                    SaleUnit::Case => 1,
                };
                if !quantity.is_multiple_of(divisor) {
                    return Err(ConfirmationError::InvalidAmount);
                }
                let packets = u32::try_from(quantity / divisor)
                    .ok()
                    .and_then(NonZeroU32::new)
                    .ok_or(ConfirmationError::Overflow)?;
                Ok(DraftLine::new(component.product, packets))
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Some)
    }

    pub(super) fn crate_amounts(&self) -> Result<CheckoutAmounts, ConfirmationError> {
        let demands = self
            .crate_packet_demands()?
            .ok_or(ConfirmationError::InvalidDraft)?;
        let row = self
            .draft
            .crate_row()
            .ok_or(ConfirmationError::InvalidDraft)?;
        let [rate] = self.prices.as_slice() else {
            return Err(ConfirmationError::InvalidDraft);
        };
        let packets = demands.iter().try_fold(0_u32, |sum, demand| {
            sum.checked_add(demand.quantity().get())
                .ok_or(ConfirmationError::Overflow)
        })?;
        let subtotal = rate
            .minor_units()
            .checked_mul(i64::from(row.quantity().get()))
            .ok_or(ConfirmationError::Overflow)?;
        self.calculate_amounts(packets, subtotal)
    }
}

impl ConfirmationFixture {
    /// Sets the authority's rate per crate and invalidates prior reviews.
    pub fn set_crate_rate(&mut self, rate: Money) -> Result<(), ConfirmationError> {
        if rate.minor_units() < 0 {
            return Err(ConfirmationError::InvalidAmount);
        }
        self.terms_revision = self
            .terms_revision
            .checked_add(1)
            .ok_or(ConfirmationError::Overflow)?;
        self.crate_rate = Some(rate);
        Ok(())
    }

    pub(super) fn build_crate_review(
        &self,
        id: DraftId,
        received: Money,
    ) -> Result<Review, ConfirmationError> {
        let (revision, draft) = self
            .drafts
            .get(&id)
            .ok_or(ConfirmationError::DraftNotFound)?;
        let row = draft.crate_row().ok_or(ConfirmationError::InvalidDraft)?;
        let rate = row
            .rate()
            .or(self.crate_rate)
            .ok_or(ConfirmationError::InvalidDraft)?;
        let packs = row
            .components()
            .iter()
            .map(|component| self.component_pack(component.product, component.unit))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Review {
            draft: draft.clone(),
            draft_revision: *revision,
            terms_revision: self.terms_revision,
            prices: vec![rate],
            payment: ProposedPayment::Named(received),
            checkout: Some(CheckoutTerms {
                lines: Vec::new(),
                loading_rate: self.loading_rate,
            }),
            crate_packs: Some(packs),
        })
    }

    fn component_pack(
        &self,
        product: ProductId,
        unit: SaleUnit,
    ) -> Result<NonZeroU32, ConfirmationError> {
        match unit {
            SaleUnit::Case if self.catalog.stock(product).is_some() => Ok(NonZeroU32::MIN),
            SaleUnit::Case => Err(ConfirmationError::InvalidDraft),
            SaleUnit::Piece => self
                .checkout_products
                .get(&product.value())
                .map(|terms| terms.pack)
                .ok_or(ConfirmationError::InvalidDraft),
        }
    }
}
