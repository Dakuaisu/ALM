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
                record.name,
                record.provisional_rate,
                record.gst_basis_points,
            ))
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
