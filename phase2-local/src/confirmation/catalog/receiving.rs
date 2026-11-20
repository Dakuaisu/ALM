use super::{Catalog, ConfirmationError, Lot, Money, ProductId};
use crate::BillDate;

impl Catalog {
    pub(in crate::confirmation) fn receive_lot(
        &mut self,
        id: ProductId,
        date: BillDate,
        quantity: u32,
        rate: Money,
    ) -> Result<(), ConfirmationError> {
        let product = self
            .products
            .get_mut(&id.value())
            .ok_or(ConfirmationError::InvalidDraft)?;
        product.stock = product
            .stock
            .checked_add(quantity)
            .ok_or(ConfirmationError::Overflow)?;
        product.lots.push(Lot {
            remaining: quantity,
            unit_cost: Some(rate.minor_units()),
            date: Some(date),
        });
        Ok(())
    }

    #[cfg(test)]
    pub(in crate::confirmation) fn receipt_lots(&self, id: ProductId) -> Vec<(BillDate, u32, i64)> {
        let mut lots: Vec<_> = self.products[&id.value()]
            .lots
            .iter()
            .filter_map(|lot| Some((lot.date?, lot.remaining, lot.unit_cost?)))
            .collect();
        lots.sort_by_key(|lot| lot.0.parts());
        lots
    }
}
