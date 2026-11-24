//! Products "Delete" (`removeRec("products", id)`), journaled through the authority.
use super::product_entry::ProductEntryError;
use crate::confirmation::ConfirmationFixture;
use crate::{ProductId, Role};

impl ConfirmationFixture {
    /// Removes a product with its lots, purchase metadata and supplier quotes. Saved purchase
    /// orders and GST bills keep their item text. Unlike the HTML, a product on a posted bill
    /// is refused, because cancel, restore and edit return stock to it.
    ///
    /// # Errors
    /// Denies Managers; `ProductNotFound`; `ProductInUse` for a product on a posted bill.
    pub fn delete_product(&mut self, role: Role, id: ProductId) -> Result<(), ProductEntryError> {
        super::product_entry::owner(role)?;
        if !self.catalog.products.contains_key(&id.value()) {
            return Err(ProductEntryError::ProductNotFound);
        }
        let on_bill =
            self.posted_sale_items
                .values()
                .flatten()
                .any(|line| line.product_id == id)
                || self.current_sales.values().any(|(review, _, _)| {
                    review.draft.lines().iter().any(|l| l.product_id() == id)
                });
        if on_bill {
            return Err(ProductEntryError::ProductInUse);
        }
        let revision = self
            .terms_revision
            .checked_add(1)
            .ok_or(ProductEntryError::Catalog(
                crate::confirmation::ConfirmationError::Overflow,
            ))?;
        self.catalog.products.remove(&id.value());
        self.catalog.product_order.retain(|other| *other != id);
        self.checkout_products.remove(&id.value());
        self.purchases.forget_product(id);
        self.terms_revision = revision;
        Ok(())
    }
}
