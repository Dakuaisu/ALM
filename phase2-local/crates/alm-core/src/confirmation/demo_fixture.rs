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
        mobile: "90000 00001",
        addr1: "12 Synthetic Lane",
        addr2: "Kanpur",
    },
    DirectoryCustomer {
        id: "batch1-market",
        name: "Synthetic Market",
        mobile: "",
        addr1: "",
        addr2: "",
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
            pack_size: "600 ml",
            uom: "Crate",
            packet_quantity: 24,
            mrp: Some(Money::from_minor_units(600)),
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
            pack_size: "1.25 L",
            uom: "Case",
            packet_quantity: 12,
            mrp: None,
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
    Quote(crate::confirmation::SupplierPriceError),
}

pub fn directory() -> BillingDirectory {
    BillingDirectory {
        customers: CUSTOMERS.to_vec(),
        products: PRODUCTS.iter().map(|seed| seed.product).collect(),
    }
}

/// Comparison suppliers and their quotes for the seeded products (per-packet buy rates).
const QUOTES: [(&str, [i64; 2]); 2] = [
    ("Synthetic Wholesale", [6_900, 3_700]),
    ("Synthetic Depot", [7_200, 3_600]),
];

/// Adds the purchasing seed (buy rates, comparison suppliers and quotes) for the app's batch 2
/// screens; never creates stock, lots or orders.
pub fn seed_purchasing(fixture: &mut ConfirmationFixture) -> Result<(), FixtureError> {
    use FixtureError::Confirmation;
    for seed in &PRODUCTS {
        fixture
            .register_purchase_product(crate::confirmation::PurchaseProduct {
                id: seed.product.id,
                item: seed.product.name.into(),
                buy_rate: seed.cost,
            })
            .map_err(Confirmation)?;
    }
    for (name, rates) in QUOTES {
        let supplier = fixture
            .add_comparison_supplier(Role::Owner, name)
            .map_err(FixtureError::Quote)?
            .ok_or(FixtureError::Quote(
                crate::confirmation::SupplierPriceError::SupplierNotFound,
            ))?;
        for (seed, rate) in PRODUCTS.iter().zip(rates) {
            fixture
                .edit_comparison_price(
                    Role::Owner,
                    crate::confirmation::SupplierPriceEdit {
                        product: seed.product.id,
                        supplier: supplier.id,
                        rate: Some(Money::from_minor_units(rate)),
                    },
                )
                .map_err(FixtureError::Quote)?;
        }
    }
    Ok(())
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
