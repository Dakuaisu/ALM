//! Seeded synthetic shop for development screens and tests only; never real data.
use crate::confirmation::owner_billing::{BillingDirectory, DirectoryCustomer, DirectoryProduct};
use crate::confirmation::{ConfirmationError, ConfirmationFixture, ProductEntryError};
use crate::provisional_preview::{LIME_GST_BASIS_POINTS, LIME_ID, LIME_INCLUSIVE_PRICE, LIME_NAME};
use crate::{DraftCustomerId, Money, ProductId, Role};

pub const CUSTOMER: &str = "Synthetic Customer";

pub fn customer_id() -> DraftCustomerId {
    DraftCustomerId::new(CUSTOMERS[0].id.into())
}

const CUSTOMERS: [DirectoryCustomer; 2] = [
    DirectoryCustomer {
        id: "batch1-customer",
        name: CUSTOMER,
    },
    DirectoryCustomer {
        id: "batch1-market",
        name: "Synthetic Market",
    },
];

struct Seed {
    product: DirectoryProduct,
    sale_rate: Money,
    cost: Money,
    stock: u32,
}

const PRODUCTS: [Seed; 2] = [
    Seed {
        product: DirectoryProduct {
            id: LIME_ID,
            name: LIME_NAME,
            provisional_rate: LIME_INCLUSIVE_PRICE,
            gst_basis_points: LIME_GST_BASIS_POINTS,
        },
        sale_rate: Money::from_minor_units(12_050),
        cost: Money::from_minor_units(7_000),
        stock: 50,
    },
    Seed {
        product: DirectoryProduct {
            id: ProductId::new(2),
            name: "Synthetic Soda",
            provisional_rate: Money::from_minor_units(5_900),
            gst_basis_points: 1_800,
        },
        sale_rate: Money::from_minor_units(6_250),
        cost: Money::from_minor_units(3_800),
        stock: 40,
    },
];

#[derive(Debug)]
pub enum FixtureError {
    Confirmation(ConfirmationError),
    Product(ProductEntryError),
}

pub fn directory() -> BillingDirectory {
    BillingDirectory {
        customers: CUSTOMERS.to_vec(),
        products: PRODUCTS.iter().map(|seed| seed.product).collect(),
    }
}

pub fn fixture() -> Result<ConfirmationFixture, FixtureError> {
    use FixtureError::{Confirmation, Product};

    let [first, second] = &PRODUCTS;
    let mut fixture = ConfirmationFixture::with_ordered_lots(
        first.product.id,
        &[(first.stock, first.cost)],
        first.sale_rate,
    )
    .map_err(Confirmation)?;
    fixture
        .add_product(second.product.id, 0, second.sale_rate)
        .map_err(Confirmation)?;
    fixture
        .catalog
        .seed_lots(second.product.id, &[(second.stock, second.cost)])
        .map_err(Confirmation)?;
    for seed in &PRODUCTS {
        let mut product = fixture
            .open_product_entry(Role::Owner, seed.product.id)
            .map_err(Product)?;
        product.item = seed.product.name.into();
        fixture
            .save_product_entry(Role::Owner, &product)
            .map_err(Product)?;
    }
    for customer in CUSTOMERS {
        fixture
            .register_customer(Role::Owner, &DraftCustomerId::new(customer.id.into()))
            .map_err(Confirmation)?;
    }
    Ok(fixture)
}
