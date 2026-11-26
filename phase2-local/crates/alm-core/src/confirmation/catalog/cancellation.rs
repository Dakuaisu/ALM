use super::{Catalog, ConfirmationError};
use crate::DraftId;

impl Catalog {
    pub(in crate::confirmation) fn stage_cancellation(
        &self,
        id: DraftId,
        cancel: bool,
    ) -> Result<Self, ConfirmationError> {
        let mut staged = self.clone();
        let mut found = false;
        for (&(sale, _, product, lot), &(quantity, _)) in &self.allocations.records {
            if sale != id {
                continue;
            }
            found = true;
            let item = staged
                .products
                .get_mut(&product)
                .ok_or(ConfirmationError::InvalidDraft)?;
            let layer = item
                .lots
                .get_mut(lot)
                .ok_or(ConfirmationError::InvalidDraft)?;
            if cancel {
                item.stock = item
                    .stock
                    .checked_add(quantity)
                    .ok_or(ConfirmationError::Overflow)?;
                layer.remaining = layer
                    .remaining
                    .checked_add(quantity)
                    .ok_or(ConfirmationError::Overflow)?;
            } else {
                item.stock = item
                    .stock
                    .checked_sub(quantity)
                    .ok_or(ConfirmationError::InsufficientStock)?;
                layer.remaining = layer
                    .remaining
                    .checked_sub(quantity)
                    .ok_or(ConfirmationError::InsufficientStock)?;
            }
        }
        if !found {
            return Err(ConfirmationError::InvalidDraft);
        }
        Ok(staged)
    }

    pub(in crate::confirmation) fn frozen_cost(
        &self,
        id: DraftId,
    ) -> Result<Option<i64>, ConfirmationError> {
        self.allocations
            .sale_costs
            .get(&id)
            .copied()
            .ok_or(ConfirmationError::InvalidDraft)
    }
}
