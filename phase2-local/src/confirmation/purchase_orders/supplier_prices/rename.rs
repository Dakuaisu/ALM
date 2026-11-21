use super::{ConfirmationFixture, PurchaseSupplierId, Role, SupplierPriceError, owner};

impl ConfirmationFixture {
    /// Renames a supplier and its exact-name purchase and quote references.
    pub fn rename_purchase_supplier(
        &mut self,
        role: Role,
        id: PurchaseSupplierId,
        name: &str,
    ) -> Result<bool, SupplierPriceError> {
        owner(role).map_err(|_| SupplierPriceError::AccessDenied)?;
        let supplier = self
            .purchases
            .suppliers
            .get(id.0)
            .ok_or(SupplierPriceError::SupplierNotFound)?;
        let name = name.trim();
        if name.is_empty() || name == supplier.name {
            return Ok(false);
        }
        let old = supplier.name.clone();
        self.purchases.suppliers[id.0].name = name.to_owned();
        for order in &mut self.purchases.orders {
            if order.supplier_name == old {
                order.supplier_name = name.to_owned();
            }
        }
        for quotes in self.purchases.quotes.values_mut() {
            if let Some(index) = quotes.0.iter().position(|(key, _)| key == &old) {
                let target_exists = quotes.0.iter().any(|(key, _)| key == name);
                let (_, rate) = quotes.0.remove(index);
                if !target_exists {
                    quotes.0.push((name.to_owned(), rate));
                }
            }
        }
        Ok(true)
    }
}
