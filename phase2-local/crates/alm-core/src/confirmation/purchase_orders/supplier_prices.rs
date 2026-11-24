use std::collections::BTreeMap;

use super::{
    ConfirmationFixture, Money, ProductId, PurchaseBook, PurchaseSupplierId, Role,
    SavedPurchaseSupplier, owner,
};

mod rename;

#[cfg(test)]
mod legacy_delete;
#[cfg(test)]
mod rename_test;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SupplierPrice {
    pub supplier: SavedPurchaseSupplier,
    pub rate: Option<Money>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SupplierPriceComparison {
    pub suppliers: Vec<SavedPurchaseSupplier>,
    pub rows: Vec<SupplierPriceRow>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SupplierPriceRow {
    pub product: ProductId,
    pub item: String,
    pub our_buy: Money,
    /// One rate (or blank) per comparison supplier, in column order.
    pub prices: Vec<Option<Money>>,
    pub best: Option<(PurchaseSupplierId, Money)>,
}

impl SupplierPriceRow {
    pub fn is_highlighted(&self, column: usize) -> bool {
        self.best
            .is_some_and(|(_, lowest)| self.prices.get(column).copied().flatten() == Some(lowest))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SupplierPriceEdit {
    pub product: ProductId,
    pub supplier: PurchaseSupplierId,
    /// Blank and zero clear the quote; a negative quote is stored but cannot be cheapest.
    pub rate: Option<Money>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SupplierPriceError {
    AccessDenied,
    ProductNotFound,
    SupplierNotFound,
}

pub(super) type ProductQuotes = BTreeMap<u64, SupplierQuotes>;

// Insertion order breaks equal-price ties in the same order as the stored quotes.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "demo-fixture", derive(serde::Serialize, serde::Deserialize))]
pub(in crate::confirmation) struct SupplierQuotes(pub(in crate::confirmation) Vec<(String, Money)>);

impl SupplierQuotes {
    pub(in crate::confirmation) fn rate(&self, supplier: &str) -> Option<Money> {
        self.0
            .iter()
            .find(|(name, _)| name == supplier)
            .map(|(_, rate)| *rate)
    }

    pub(in crate::confirmation) fn set(&mut self, supplier: &str, rate: Option<Money>) {
        let existing = self.0.iter().position(|(name, _)| name == supplier);
        match (existing, rate.filter(|rate| rate.minor_units() != 0)) {
            (Some(index), Some(rate)) => self.0[index].1 = rate,
            (None, Some(rate)) => self.0.push((supplier.to_owned(), rate)),
            (Some(index), None) => {
                self.0.remove(index);
            }
            (None, None) => {}
        }
    }

    pub(super) fn record_receipt(&mut self, supplier: &str, rate: Money) {
        if let Some((_, current)) = self
            .0
            .iter_mut()
            .find(|(name, _)| name.eq_ignore_ascii_case(supplier))
        {
            *current = rate;
        } else {
            self.set(supplier, Some(rate));
        }
    }

    pub(super) fn receipt_quote(&self, supplier: &str) -> Option<Money> {
        self.0
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(supplier))
            .map(|(_, rate)| *rate)
    }

    fn best(&self, suppliers: &[SavedPurchaseSupplier]) -> Option<(PurchaseSupplierId, Money)> {
        self.0
            .iter()
            .filter(|(_, rate)| rate.minor_units() > 0)
            .filter_map(|(name, rate)| {
                let supplier = suppliers.iter().find(|supplier| supplier.name == *name)?;
                Some((supplier.id, *rate))
            })
            .min_by_key(|(_, rate)| rate.minor_units())
    }
}

struct PriceSupplierColumns<'a> {
    explicit: &'a [PurchaseSupplierId],
    contacts: &'a [SavedPurchaseSupplier],
}

impl PurchaseBook {
    pub(super) fn supplier_id_or_insert(&mut self, name: &str) -> PurchaseSupplierId {
        if let Some(supplier) = self.suppliers.iter().find(|supplier| supplier.name == name) {
            return supplier.id;
        }
        let id = PurchaseSupplierId(self.suppliers.len());
        self.suppliers.push(SavedPurchaseSupplier {
            id,
            name: name.to_owned(),
        });
        id
    }

    pub(in crate::confirmation) fn add_price_supplier(
        &mut self,
        name: &str,
    ) -> Option<SavedPurchaseSupplier> {
        let name = name.trim();
        if name.is_empty() || self.suppliers.iter().any(|supplier| supplier.name == name) {
            return None;
        }
        let id = self.supplier_id_or_insert(name);
        self.price_suppliers.push(id);
        self.suppliers.get(id.0).cloned()
    }

    pub(in crate::confirmation) fn price_suppliers(&self) -> Vec<SavedPurchaseSupplier> {
        self.project_price_suppliers(self.supplier_columns())
    }

    fn supplier_columns(&self) -> PriceSupplierColumns<'_> {
        PriceSupplierColumns {
            explicit: &self.price_suppliers,
            contacts: &self.suppliers,
        }
    }

    fn project_price_suppliers(
        &self,
        columns: PriceSupplierColumns<'_>,
    ) -> Vec<SavedPurchaseSupplier> {
        let mut names = std::collections::BTreeSet::new();
        columns
            .explicit
            .iter()
            .map(|id| &self.suppliers[id.0])
            .chain(
                columns
                    .contacts
                    .iter()
                    .filter(|supplier| !columns.explicit.contains(&supplier.id)),
            )
            .filter(|supplier| names.insert(supplier.name.clone()))
            .cloned()
            .collect()
    }

    pub(in crate::confirmation) fn product_quotes(&self, id: ProductId) -> SupplierQuotes {
        self.quotes.get(&id.value()).cloned().unwrap_or_default()
    }

    pub(in crate::confirmation) fn set_registered_buy(&mut self, id: ProductId, rate: Money) {
        if let Some(product) = self.products.iter_mut().find(|product| product.id == id) {
            product.buy_rate = rate;
        }
    }

    pub(in crate::confirmation) fn reorder_quote(
        &self,
        id: ProductId,
        columns: &[SavedPurchaseSupplier],
    ) -> Option<(&str, Money)> {
        let quotes = self.quotes.get(&id.value())?;
        let (_, minimum) = quotes.best(columns)?;
        // Legacy Reorder names the first stored quote at the live minimum, even off-column.
        let (name, _) = quotes.0.iter().find(|(_, rate)| *rate == minimum)?;
        Some((name.as_str(), minimum))
    }

    pub(in crate::confirmation) fn save_product_quotes(
        &mut self,
        id: ProductId,
        quotes: SupplierQuotes,
    ) {
        if quotes.0.is_empty() {
            self.quotes.remove(&id.value());
        } else {
            self.quotes.insert(id.value(), quotes);
        }
    }
}

impl ConfirmationFixture {
    /// Adds a supplier record and an initially blank column independently of any quote.
    pub fn add_comparison_supplier(
        &mut self,
        role: Role,
        name: &str,
    ) -> Result<Option<SavedPurchaseSupplier>, SupplierPriceError> {
        owner(role).map_err(|_| SupplierPriceError::AccessDenied)?;
        Ok(self.purchases.add_price_supplier(name))
    }

    /// Compares a selected product, or every product with at least one stored quote.
    pub fn compare_supplier_prices(
        &self,
        role: Role,
        product: Option<ProductId>,
    ) -> Result<SupplierPriceComparison, SupplierPriceError> {
        self.compare_supplier_prices_with_columns(role, product, self.purchases.supplier_columns())
    }

    fn compare_supplier_prices_with_columns(
        &self,
        role: Role,
        product: Option<ProductId>,
        columns: PriceSupplierColumns<'_>,
    ) -> Result<SupplierPriceComparison, SupplierPriceError> {
        owner(role).map_err(|_| SupplierPriceError::AccessDenied)?;
        let suppliers = self.purchases.project_price_suppliers(columns);
        let row = |id| {
            let (item, our_buy) = self.supplier_price_product(id)?;
            let quotes = self.purchases.quotes.get(&id.value());
            Some(SupplierPriceRow {
                product: id,
                item: item.to_owned(),
                our_buy,
                prices: suppliers
                    .iter()
                    .map(|supplier| quotes.and_then(|quotes| quotes.rate(&supplier.name)))
                    .collect(),
                best: quotes.and_then(|quotes| quotes.best(&suppliers)),
            })
        };
        let rows = match product {
            Some(id) => vec![row(id).ok_or(SupplierPriceError::ProductNotFound)?],
            None => self
                .purchases
                .quotes
                .keys()
                .copied()
                .map(ProductId::new)
                .filter_map(row)
                .collect(),
        };
        Ok(SupplierPriceComparison { suppliers, rows })
    }

    /// Stores the comparison cell immediately, without changing product pricing or purchase costs.
    pub fn edit_comparison_price(
        &mut self,
        role: Role,
        edit: SupplierPriceEdit,
    ) -> Result<(), SupplierPriceError> {
        owner(role).map_err(|_| SupplierPriceError::AccessDenied)?;
        self.supplier_price_product(edit.product)
            .ok_or(SupplierPriceError::ProductNotFound)?;
        let supplier = self
            .purchases
            .suppliers
            .get(edit.supplier.0)
            .ok_or(SupplierPriceError::SupplierNotFound)?;
        let mut quotes = self.purchases.product_quotes(edit.product);
        quotes.set(&supplier.name, edit.rate);
        self.purchases.save_product_quotes(edit.product, quotes);
        Ok(())
    }

    pub(in super::super) fn supplier_price_product(&self, id: ProductId) -> Option<(&str, Money)> {
        let catalog = self.catalog.entered_price_product(id);
        let products = &self.purchases.products;
        let Some(product) = products.iter().find(|product| product.id == id) else {
            return catalog;
        };
        let item = catalog.map_or(product.item.as_str(), |(item, _)| item);
        Some((item, product.buy_rate))
    }
}
