use super::{
    ConfirmationFixture, Money, ProductEntry, ProductId, ProductPricing, ProductTaxOverrides, Role,
    Target, owner,
};
use crate::confirmation::catalog::Catalog;
use crate::confirmation::purchase_orders::supplier_prices::SupplierQuotes;
use crate::confirmation::{
    PurchaseSupplierId, SavedPurchaseSupplier, SupplierPrice, SupplierPriceError,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct SupplierPriceDraft {
    columns: Vec<SavedPurchaseSupplier>,
    pub(super) quotes: SupplierQuotes,
}

impl ProductEntry {
    pub(super) fn blank(
        target: Target,
        pricing: ProductPricing,
        supplier_prices: SupplierPriceDraft,
    ) -> Self {
        Self {
            target,
            item: String::new(),
            brand: String::new(),
            pricing,
            buy_edited: false,
            reorder_level: None,
            tax: ProductTaxOverrides::default(),
            supplier_prices,
        }
    }

    pub fn supplier_prices(&self) -> Vec<SupplierPrice> {
        self.supplier_prices
            .columns
            .iter()
            .map(|supplier| SupplierPrice {
                rate: self.supplier_prices.quotes.rate(&supplier.name),
                supplier: supplier.clone(),
            })
            .collect()
    }

    /// Edits only the detached form; a successful Product Save stores the quote.
    pub fn set_supplier_price(
        &mut self,
        id: PurchaseSupplierId,
        rate: Option<Money>,
    ) -> Result<(), SupplierPriceError> {
        let supplier = self
            .supplier_prices
            .columns
            .iter()
            .find(|supplier| supplier.id == id)
            .ok_or(SupplierPriceError::SupplierNotFound)?;
        self.supplier_prices.quotes.set(&supplier.name, rate);
        Ok(())
    }
}

impl ConfirmationFixture {
    pub(super) fn save_product_entry_prices(&mut self, id: ProductId, form: &ProductEntry) {
        if form.buy_edited {
            self.purchases.set_registered_buy(id, form.pricing.buy());
        }
        self.purchases
            .save_product_quotes(id, form.supplier_prices.quotes.clone());
    }

    pub(super) fn product_quote_draft(&self, id: ProductId) -> SupplierPriceDraft {
        SupplierPriceDraft {
            columns: self.purchases.price_suppliers(),
            quotes: self.purchases.product_quotes(id),
        }
    }

    /// Adds the supplier immediately, without saving any of the open form's changes.
    pub fn add_product_entry_supplier(
        &mut self,
        role: Role,
        form: &mut ProductEntry,
        name: &str,
    ) -> Result<Option<SavedPurchaseSupplier>, SupplierPriceError> {
        owner(role).map_err(|_| SupplierPriceError::AccessDenied)?;
        let added = self.purchases.add_price_supplier(name);
        if let Some(supplier) = &added {
            form.supplier_prices.columns.push(supplier.clone());
        }
        Ok(added)
    }
}

impl Catalog {
    pub(in crate::confirmation) fn entered_price_product(
        &self,
        id: ProductId,
    ) -> Option<(&str, Money)> {
        let product = self.products.get(&id.value())?;
        Some((&product.details.as_ref()?.item, product.pricing.buy()))
    }
}
