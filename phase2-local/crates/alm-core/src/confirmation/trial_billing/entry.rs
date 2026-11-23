use std::num::NonZeroU32;

use crate::provisional_preview::{
    self, LIME_GST_BASIS_POINTS, LIME_ID, LIME_INCLUSIVE_PRICE, LIME_NAME, ProvisionalBillPreview,
    ProvisionalPreviewError,
};
use crate::{DraftCustomer, DraftCustomerId, DraftLine, Money, ProductId, ProvisionalDraft, Role};

use super::{ConfirmationFixture, FixtureError, TrialBillingError};

const CUSTOMERS: [(&str, &str); 2] = [
    ("batch1-customer", super::CUSTOMER),
    ("batch1-market", "Synthetic Market"),
];

struct Product {
    id: ProductId,
    name: &'static str,
    provisional_rate: Money,
    gst: u32,
    sale_rate: Money,
    cost: Money,
    stock: u32,
}

const PRODUCTS: [Product; 2] = [
    Product {
        id: LIME_ID,
        name: LIME_NAME,
        provisional_rate: LIME_INCLUSIVE_PRICE,
        gst: LIME_GST_BASIS_POINTS,
        sale_rate: Money::from_minor_units(12_050),
        cost: Money::from_minor_units(7_000),
        stock: 50,
    },
    Product {
        id: ProductId::new(2),
        name: "Synthetic Soda",
        provisional_rate: Money::from_minor_units(5_900),
        gst: 1_800,
        sale_rate: Money::from_minor_units(6_250),
        cost: Money::from_minor_units(3_800),
        stock: 40,
    },
];

pub fn fixture() -> Result<ConfirmationFixture, FixtureError> {
    use FixtureError::{Confirmation, Product};

    let [first, second] = &PRODUCTS;
    let mut fixture = ConfirmationFixture::with_ordered_lots(
        first.id,
        &[(first.stock, first.cost)],
        first.sale_rate,
    )
    .map_err(Confirmation)?;
    fixture
        .add_product(second.id, 0, second.sale_rate)
        .map_err(Confirmation)?;
    fixture
        .catalog
        .seed_lots(second.id, &[(second.stock, second.cost)])
        .map_err(Confirmation)?;
    for record in &PRODUCTS {
        let mut product = fixture
            .open_product_entry(Role::Owner, record.id)
            .map_err(Product)?;
        product.item = record.name.into();
        fixture
            .save_product_entry(Role::Owner, &product)
            .map_err(Product)?;
    }
    for (id, _) in CUSTOMERS {
        fixture
            .register_customer(Role::Owner, &DraftCustomerId::new(id.into()))
            .map_err(Confirmation)?;
    }
    Ok(fixture)
}

pub fn customers(
    authority: &ConfirmationFixture,
) -> impl Iterator<Item = (&'static str, &'static str)> + '_ {
    CUSTOMERS
        .into_iter()
        .filter(|(id, _)| authority.registered_customers.contains(*id))
}

pub fn products(authority: &ConfirmationFixture) -> impl Iterator<Item = (ProductId, &str)> {
    PRODUCTS.iter().filter_map(|record| {
        authority
            .catalog
            .item_name(record.id)
            .map(|name| (record.id, name))
    })
}

pub fn select_customer(
    authority: &ConfirmationFixture,
    choice: &str,
) -> Result<DraftCustomer, TrialBillingError> {
    let (id, name) = customers(authority)
        .find(|(id, _)| *id == choice)
        .ok_or(TrialBillingError::Customer)?;
    Ok(DraftCustomer::Selected {
        id: DraftCustomerId::new(id.into()),
        name: name.into(),
    })
}

pub(super) fn check_customer(
    authority: &ConfirmationFixture,
    choice: &DraftCustomer,
) -> Result<(), TrialBillingError> {
    match choice {
        DraftCustomer::Selected { id, .. }
            if &select_customer(authority, id.value())? == choice =>
        {
            Ok(())
        }
        DraftCustomer::Selected { .. } | DraftCustomer::Typed(_) | DraftCustomer::Unassigned => {
            Err(TrialBillingError::Customer)
        }
    }
}

pub fn select_line(
    authority: &ConfirmationFixture,
    id: ProductId,
    qty: NonZeroU32,
) -> Result<DraftLine, ProvisionalPreviewError> {
    product(authority, id).ok_or(ProvisionalPreviewError::UnknownProduct(id))?;
    Ok(DraftLine::new(id, qty))
}

fn product(authority: &ConfirmationFixture, id: ProductId) -> Option<&'static Product> {
    authority.catalog.item_name(id)?;
    PRODUCTS.iter().find(|record| record.id == id)
}

pub fn preview(
    authority: &ConfirmationFixture,
    draft: &ProvisionalDraft,
) -> Result<ProvisionalBillPreview, ProvisionalPreviewError> {
    provisional_preview::with_catalog(draft, |id| {
        let record = product(authority, id)?;
        Some((record.name, record.provisional_rate, record.gst))
    })
}
