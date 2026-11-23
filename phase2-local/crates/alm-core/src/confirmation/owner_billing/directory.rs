use std::num::NonZeroU32;

use crate::provisional_preview::{self, ProvisionalBillPreview, ProvisionalPreviewError};
use crate::{DraftCustomer, DraftCustomerId, DraftLine, Money, ProductId, ProvisionalDraft};

use super::{ConfirmationFixture, OwnerBillingError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DirectoryCustomer {
    pub id: &'static str,
    pub name: &'static str,
}

/// A product offered on New Bill with its provisional (Manager-visible) GST-inclusive rate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DirectoryProduct {
    pub id: ProductId,
    pub name: &'static str,
    pub provisional_rate: Money,
    pub gst_basis_points: u32,
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
