use super::{Product, ProductPriceInput, ProductPricing};
use crate::confirmation::{ConfirmationError, ConfirmationFixture, gst_defaults};
use crate::{Money, ProductId, Role};

/// Overrides in basis points; `None` means automatic, not zero.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProductTaxOverrides {
    pub gst_rate: Option<u32>,
    pub cess: Option<u32>,
    pub hsn: Option<String>,
}

impl ProductTaxOverrides {
    fn hsn(&self) -> Option<&str> {
        self.hsn.as_deref().map(str::trim).filter(|s| !s.is_empty())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResolvedProductTax<'a> {
    pub gst_rate: u32,
    pub cess: u32,
    pub hsn: &'a str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProductEntryError {
    AccessDenied,
    ProductNotFound,
    ProductIdInUse,
    ItemNameRequired,
    DuplicateName { existing: String },
    Catalog(ConfirmationError),
}

impl ProductEntryError {
    pub fn duplicate_warning_text(&self) -> Option<String> {
        match self {
            Self::DuplicateName { existing } => Some(format!(
                "A product called “{existing}” already exists — use a different name (e.g. add the pack size)"
            )),
            Self::AccessDenied
            | Self::ProductNotFound
            | Self::ProductIdInUse
            | Self::ItemNameRequired
            | Self::Catalog(_) => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Target {
    New(ProductId),
    Existing(ProductId),
}

/// Detached Owner form. Pricing edits do not touch the catalog until save.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductEntry {
    target: Target,
    pub item: String,
    pub brand: String,
    pricing: ProductPricing,
    pub reorder_level: Option<u32>,
    pub tax: ProductTaxOverrides,
}

impl ProductEntry {
    pub const fn pricing(&self) -> ProductPricing {
        self.pricing
    }

    pub fn apply_price(&mut self, input: ProductPriceInput) -> Result<(), ConfirmationError> {
        self.pricing.apply(input)
    }

    pub fn effective_tax(&self) -> ResolvedProductTax<'_> {
        let (rate, hsn) = gst_defaults::defaults(self.item.trim());
        ResolvedProductTax {
            gst_rate: self.tax.gst_rate.unwrap_or(rate),
            cess: self.tax.cess.unwrap_or(0),
            hsn: self.tax.hsn().unwrap_or(hsn),
        }
    }

    fn blank(target: Target, pricing: ProductPricing) -> Self {
        Self {
            target,
            item: String::new(),
            brand: String::new(),
            pricing,
            reorder_level: None,
            tax: ProductTaxOverrides::default(),
        }
    }
}

#[derive(Clone)]
#[cfg_attr(test, derive(Debug, Eq, PartialEq))]
pub(super) struct ProductDetails {
    item: String,
    brand: String,
    reorder_level: u32,
    tax: ProductTaxOverrides,
}

const fn owner(role: Role) -> Result<(), ProductEntryError> {
    match role {
        Role::Owner => Ok(()),
        Role::Manager => Err(ProductEntryError::AccessDenied),
    }
}

impl ConfirmationFixture {
    /// Sets the trusted synthetic fallback used when an Owner saves a blank reorder field.
    pub const fn with_product_reorder_default(mut self, default: u32) -> Self {
        self.product_reorder_default = default;
        self
    }

    pub fn begin_product_entry(
        &self,
        role: Role,
        id: ProductId,
    ) -> Result<ProductEntry, ProductEntryError> {
        owner(role)?;
        if self.catalog.products.contains_key(&id.value()) {
            return Err(ProductEntryError::ProductIdInUse);
        }
        Ok(ProductEntry::blank(
            Target::New(id),
            ProductPricing::at_sell(Money::from_minor_units(0)),
        ))
    }

    /// Returns IDs of entered products without modifying the catalog.
    pub fn filter_products_by_brand(
        &self,
        role: Role,
        brand: Option<&str>,
    ) -> Result<Vec<ProductId>, ProductEntryError> {
        owner(role)?;
        Ok(self
            .catalog
            .products
            .iter()
            .filter_map(|(&id, product)| {
                let details = product.details.as_ref()?;
                brand
                    .is_none_or(|name| name == details.brand)
                    .then_some(ProductId::new(id))
            })
            .collect())
    }

    pub fn open_product_entry(
        &self,
        role: Role,
        id: ProductId,
    ) -> Result<ProductEntry, ProductEntryError> {
        owner(role)?;
        let product = self
            .catalog
            .products
            .get(&id.value())
            .ok_or(ProductEntryError::ProductNotFound)?;
        let target = Target::Existing(id);
        Ok(match &product.details {
            Some(details) => ProductEntry {
                target,
                item: details.item.clone(),
                brand: details.brand.clone(),
                pricing: product.pricing,
                reorder_level: Some(details.reorder_level),
                tax: details.tax.clone(),
            },
            None => ProductEntry::blank(target, product.pricing),
        })
    }

    pub fn save_product_entry(
        &mut self,
        role: Role,
        form: &ProductEntry,
    ) -> Result<(), ProductEntryError> {
        owner(role)?;
        let id = match form.target {
            Target::New(id) | Target::Existing(id) => id,
        };
        let item = form.item.trim();
        if item.is_empty() {
            return Err(ProductEntryError::ItemNameRequired);
        }
        let name = item.to_lowercase();
        for (&other, product) in &self.catalog.products {
            if other != id.value()
                && let Some(details) = &product.details
                && details.item.to_lowercase() == name
            {
                return Err(ProductEntryError::DuplicateName {
                    existing: details.item.clone(),
                });
            }
        }
        let details = ProductDetails {
            item: item.to_owned(),
            brand: form.brand.trim().to_owned(),
            reorder_level: form.reorder_level.unwrap_or(self.product_reorder_default),
            tax: ProductTaxOverrides {
                gst_rate: form.tax.gst_rate,
                cess: form.tax.cess,
                hsn: form.tax.hsn().map(str::to_owned),
            },
        };
        match form.target {
            Target::New(id) => {
                if self.catalog.products.contains_key(&id.value()) {
                    return Err(ProductEntryError::ProductIdInUse);
                }
                self.add_catalog_product(
                    id,
                    Product {
                        details: Some(details),
                        ..Product::new(0, form.pricing)
                    },
                )
                .map_err(ProductEntryError::Catalog)
            }
            Target::Existing(id) => {
                if form.pricing.sell().minor_units() < 0 {
                    return Err(ProductEntryError::Catalog(ConfirmationError::InvalidAmount));
                }
                let revision = self
                    .terms_revision
                    .checked_add(1)
                    .ok_or(ProductEntryError::Catalog(ConfirmationError::Overflow))?;
                let product = self
                    .catalog
                    .products
                    .get_mut(&id.value())
                    .ok_or(ProductEntryError::ProductNotFound)?;
                product.details = Some(details);
                product.pricing = form.pricing;
                self.terms_revision = revision;
                Ok(())
            }
        }
    }

    /// Adds a distinct synthetic product to the fixture catalog.
    pub fn add_product(
        &mut self,
        product: ProductId,
        stock: u32,
        price: Money,
    ) -> Result<(), ConfirmationError> {
        self.add_catalog_product(product, Product::new(stock, ProductPricing::at_sell(price)))
    }

    fn add_catalog_product(
        &mut self,
        id: ProductId,
        product: Product,
    ) -> Result<(), ConfirmationError> {
        let revision = self
            .terms_revision
            .checked_add(1)
            .ok_or(ConfirmationError::Overflow)?;
        let movement = self
            .movement_epoch
            .checked_add(1)
            .ok_or(ConfirmationError::Overflow)?;
        self.catalog.add(id, product)?;
        self.movement_epoch = movement;
        self.terms_revision = revision;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
