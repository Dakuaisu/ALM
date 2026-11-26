use std::num::NonZeroU32;

use crate::provisional_preview::{self, ProvisionalBillPreview, ProvisionalPreviewError};
use crate::{DraftCustomer, DraftCustomerId, DraftLine, Money, ProductId, ProvisionalDraft};

use super::{ConfirmationFixture, OwnerBillingError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DirectoryCustomer {
    pub id: &'static str,
    pub name: &'static str,
    pub mobile: &'static str,
    pub addr1: &'static str,
    pub addr2: &'static str,
}

impl DirectoryCustomer {
    /// The bill's customer address: `addr1` then `", " + addr2`, blank without `addr1`.
    pub fn address(&self) -> String {
        if self.addr1.is_empty() {
            String::new()
        } else if self.addr2.is_empty() {
            self.addr1.to_owned()
        } else {
            format!("{}, {}", self.addr1, self.addr2)
        }
    }
}

/// A product offered on New Bill with its provisional (Manager-visible) GST-inclusive rate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DirectoryProduct {
    pub id: ProductId,
    pub name: &'static str,
    pub provisional_rate: Money,
    pub gst_basis_points: u32,
    pub pack_size: &'static str,
    pub uom: &'static str,
    /// Pieces per packet (`packetQty`), at least 1.
    pub packet_quantity: u32,
    /// Per-piece MRP; `None` when the product has none.
    pub mrp: Option<Money>,
}

impl DirectoryProduct {
    /// The PACK column: `(packSize + " " + uom).trim()`.
    pub fn pack(&self) -> String {
        format!("{} {}", self.pack_size, self.uom).trim().to_owned()
    }
}

/// Customers and products a bill may select; only entries the authority knows are offered.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BillingDirectory {
    pub customers: Vec<DirectoryCustomer>,
    pub products: Vec<DirectoryProduct>,
}

impl BillingDirectory {
    pub fn customers<'a>(
        &'a self,
        authority: &'a ConfirmationFixture,
    ) -> impl Iterator<Item = DirectoryCustomer> + 'a {
        self.customers
            .iter()
            .copied()
            .filter(|customer| authority.registered_customers.contains(customer.id))
    }

    pub fn products<'a>(
        &'a self,
        authority: &'a ConfirmationFixture,
    ) -> impl Iterator<Item = (ProductId, &'a str)> + 'a {
        self.products.iter().filter_map(|record| {
            authority
                .catalog
                .item_name(record.id)
                .map(|name| (record.id, name))
        })
    }

    /// The autocomplete list: names containing `query` (case-insensitive), by name, at most 40.
    pub fn search_customers<'a>(
        &'a self,
        authority: &'a ConfirmationFixture,
        query: &str,
    ) -> Vec<DirectoryCustomer> {
        let query = query.trim().to_lowercase();
        let mut found: Vec<_> = self
            .customers(authority)
            .filter(|customer| customer.name.to_lowercase().contains(&query))
            .collect();
        found.sort_by(|a, b| a.name.cmp(b.name));
        found.truncate(40);
        found
    }

    /// The item autocomplete: names containing `query` (case-insensitive), catalog order, ≤ 40.
    pub fn search_products<'a>(
        &'a self,
        authority: &'a ConfirmationFixture,
        query: &str,
    ) -> Vec<(ProductId, &'a str)> {
        let query = query.trim().to_lowercase();
        self.products(authority)
            .filter(|(_, name)| name.to_lowercase().contains(&query))
            .take(40)
            .collect()
    }

    /// GST Sales "Pick customer": fills name, GSTIN (blank: the directory holds none), address
    /// and phone from the saved customer, leaving the state fields as they are.
    ///
    /// # Errors
    /// Rejects a customer the authority does not know, without changing the form.
    pub fn fill_gst_customer(
        &self,
        authority: &ConfirmationFixture,
        fields: &mut super::super::GstBillFields,
        choice: &str,
    ) -> Result<(), OwnerBillingError> {
        let customer = self.customer(authority, choice)?;
        fields.customer = customer.name.to_owned();
        fields.gstin = String::new();
        fields.cust_addr = customer.address();
        fields.cust_phone = customer.mobile.to_owned();
        Ok(())
    }

    pub fn customer(
        &self,
        authority: &ConfirmationFixture,
        choice: &str,
    ) -> Result<DirectoryCustomer, OwnerBillingError> {
        self.customers(authority)
            .find(|customer| customer.id == choice)
            .ok_or(OwnerBillingError::Customer)
    }

    pub fn select_customer(
        &self,
        authority: &ConfirmationFixture,
        choice: &str,
    ) -> Result<DraftCustomer, OwnerBillingError> {
        let customer = self.customer(authority, choice)?;
        Ok(DraftCustomer::Selected {
            id: DraftCustomerId::new(customer.id.into()),
            name: customer.name.into(),
        })
    }

    /// A selected identity must still match its directory label; typed names never bind.
    pub(super) fn check_customer(
        &self,
        authority: &ConfirmationFixture,
        choice: &DraftCustomer,
    ) -> Result<(), OwnerBillingError> {
        match choice {
            DraftCustomer::Selected { id, .. }
                if &self.select_customer(authority, id.value())? == choice =>
            {
                Ok(())
            }
            DraftCustomer::Selected { .. }
            | DraftCustomer::Typed(_)
            | DraftCustomer::Unassigned => Err(OwnerBillingError::Customer),
        }
    }

    pub fn select_line(
        &self,
        authority: &ConfirmationFixture,
        id: ProductId,
        qty: NonZeroU32,
    ) -> Result<DraftLine, ProvisionalPreviewError> {
        self.product(authority, id)
            .ok_or(ProvisionalPreviewError::UnknownProduct(id))?;
        Ok(DraftLine::new(id, qty))
    }

    fn product(&self, authority: &ConfirmationFixture, id: ProductId) -> Option<&DirectoryProduct> {
        authority.catalog.item_name(id)?;
        self.products.iter().find(|record| record.id == id)
    }

    pub fn preview(
        &self,
        authority: &ConfirmationFixture,
        draft: &ProvisionalDraft,
    ) -> Result<ProvisionalBillPreview, ProvisionalPreviewError> {
        provisional_preview::with_catalog(draft, |id| {
            let record = self.product(authority, id)?;
            Some((
                record.name.to_owned(),
                record.provisional_rate,
                record.gst_basis_points,
            ))
        })
    }

    /// A Manager's item autocomplete over the catalog its device pulled (names and sale
    /// prices only): names containing `query` (case-insensitive), catalog order, at most 40.
    pub fn search_catalog<'a>(
        catalog: &'a [crate::projection::ProjectedProduct],
        query: &str,
    ) -> Vec<(ProductId, &'a str)> {
        let query = query.trim().to_lowercase();
        catalog
            .iter()
            .filter(|row| row.name.to_lowercase().contains(&query))
            .take(40)
            .map(|row| (ProductId::new(row.id), row.name.as_str()))
            .collect()
    }

    /// The Manager's customer autocomplete from its pulled names: names containing `query`
    /// (case-insensitive), by name, at most 40.
    pub fn search_customer_names<'a>(
        names: &'a [crate::projection::ProjectedCustomerName],
        query: &str,
    ) -> Vec<&'a crate::projection::ProjectedCustomerName> {
        let query = query.trim().to_lowercase();
        let mut found: Vec<_> = names
            .iter()
            .filter(|row| row.name.to_lowercase().contains(&query))
            .collect();
        found.sort_by(|a, b| a.name.cmp(&b.name));
        found.truncate(40);
        found
    }

    /// A customer the Manager picked from its pulled names.
    ///
    /// # Errors
    /// `Customer` for an ID not among the names.
    pub fn select_listed_customer(
        names: &[crate::projection::ProjectedCustomerName],
        choice: &str,
    ) -> Result<DraftCustomer, OwnerBillingError> {
        let customer = names
            .iter()
            .find(|row| row.id == choice)
            .ok_or(OwnerBillingError::Customer)?;
        Ok(DraftCustomer::Selected {
            id: DraftCustomerId::new(customer.id.clone()),
            name: customer.name.clone(),
        })
    }

    /// A line for a product in the pulled catalog.
    ///
    /// # Errors
    /// Rejects a product the catalog does not hold.
    pub fn select_catalog_line(
        catalog: &[crate::projection::ProjectedProduct],
        id: ProductId,
        qty: NonZeroU32,
    ) -> Result<DraftLine, ProvisionalPreviewError> {
        catalog
            .iter()
            .find(|row| row.id == id.value())
            .ok_or(ProvisionalPreviewError::UnknownProduct(id))?;
        Ok(DraftLine::new(id, qty))
    }

    /// New Bill's provisional preview at the pulled catalog's sale prices. The GST rate is the
    /// directory's for a directory product, otherwise `gstInfo`'s keyword default for the name
    /// (a Manager holds no product tax overrides; OPEN-012 keeps the HTML's rule).
    ///
    /// # Errors
    /// Rejects products the catalog does not hold, crate rows and overflow.
    pub fn catalog_preview(
        &self,
        catalog: &[crate::projection::ProjectedProduct],
        draft: &ProvisionalDraft,
    ) -> Result<ProvisionalBillPreview, ProvisionalPreviewError> {
        provisional_preview::with_catalog(draft, |id| {
            let row = catalog.iter().find(|row| row.id == id.value())?;
            let gst = self
                .products
                .iter()
                .find(|record| record.id == id)
                .map_or_else(
                    || super::super::gst_defaults::defaults(&row.name).0,
                    |record| record.gst_basis_points,
                );
            Some((row.name.clone(), row.rate, gst))
        })
    }
}

#[cfg(test)]
mod search_tests {
    use crate::ProductId;
    use crate::confirmation::demo_fixture::{directory, fixture};

    #[test]
    fn searches_match_case_insensitively_in_core_order() {
        let authority = fixture().unwrap();
        let directory = directory();
        let names: Vec<_> = directory
            .search_customers(&authority, " SYNTH")
            .iter()
            .map(|c| c.name)
            .collect();
        assert_eq!(names, ["Synthetic Customer", "Synthetic Market"]);
        assert_eq!(directory.search_customers(&authority, "market").len(), 1);
        assert_eq!(
            directory.search_products(&authority, "soda"),
            [(ProductId::new(2), "Synthetic Soda")]
        );
        assert_eq!(directory.search_products(&authority, "").len(), 2);
    }
}
