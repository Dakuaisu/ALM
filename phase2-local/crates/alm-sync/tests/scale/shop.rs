//! A shop-sized synthetic authority for the scale checks: 500 products, 1,000 customers, and
//! the intents of 20,000 bills (one in ten with a crate row beside its item line).
#![allow(dead_code)]

use std::num::NonZeroU32;

use alm_core::confirmation::ConfirmationFixture;
use alm_core::confirmation::demo_fixture::{directory, fixture, seed_purchasing};
use alm_core::confirmation::owner_billing::{
    BillingDirectory, DirectoryCustomer, DirectoryProduct,
};
use alm_core::projection::ProjectionSnapshot;
use alm_core::{
    BillDate, CrateComponent, CrateRow, DraftCustomer, DraftCustomerId, DraftId, DraftLine, Money,
    ProductId, ProvisionalDraft, Role, SaleUnit,
};
use alm_sync::{Intent, SaleIntent};

pub const PRODUCTS: u64 = 500;
pub const CUSTOMERS: usize = 1_000;
pub const BILLS: u64 = 20_000;

fn leak(text: String) -> &'static str {
    Box::leak(text.into_boxed_str())
}

fn n(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).unwrap()
}

/// The demo shop grown to 500 products (ample stock) and 1,000 directory customers.
pub fn shop() -> (ConfirmationFixture, BillingDirectory) {
    let mut shop = fixture().unwrap();
    seed_purchasing(&mut shop).unwrap();
    let mut book = directory();
    for id in 3..=PRODUCTS {
        let product = ProductId::new(id);
        let price = Money::from_minor_units(1_000 + i64::try_from(id).unwrap());
        shop.add_product(product, 1_000_000, price).unwrap();
        let mut entry = shop.open_product_entry(Role::Owner, product).unwrap();
        entry.item = format!("Synthetic Product {id}");
        shop.save_product_entry(Role::Owner, &entry).unwrap();
        book.products.push(DirectoryProduct {
            id: product,
            name: leak(format!("Synthetic Product {id}")),
            provisional_rate: price,
            gst_basis_points: 1_800,
            pack_size: "250 ml",
            uom: "Case",
            packet_quantity: 24,
            mrp: None,
        });
    }
    while book.customers.len() < CUSTOMERS {
        let c = book.customers.len();
        let id = leak(format!("scale-customer-{c}"));
        shop.register_customer(Role::Owner, &DraftCustomerId::new(id.into()))
            .unwrap();
        book.customers.push(DirectoryCustomer {
            id: id.into(),
            name: format!("Synthetic Customer {c}"),
            mobile: String::new(),
            addr1: String::new(),
            addr2: String::new(),
        });
    }
    (shop, book)
}

/// Bill `bill`'s sale intent, priced from `snapshot`.
pub fn bill(bill: u64, book: &BillingDirectory, snapshot: &ProjectionSnapshot) -> Intent {
    let p = 3 + bill % (PRODUCTS - 2);
    let customer = &book.customers[usize::try_from(bill).unwrap() % book.customers.len()];
    let month = u8::try_from(1 + bill % 12).unwrap();
    let day = u8::try_from(1 + bill % 28).unwrap();
    let date = BillDate::new(2025, month, day).unwrap();
    let mut draft = ProvisionalDraft::new(
        DraftId::new(bill),
        vec![DraftLine::new(
            ProductId::new(p),
            n(1 + u32::try_from(bill % 3).unwrap()),
        )],
    )
    .with_customer(DraftCustomer::Selected {
        id: DraftCustomerId::new(customer.id.clone()),
        name: customer.name.clone(),
    })
    .with_bill_date(Some(date));
    if bill.is_multiple_of(10) {
        draft.add_crate_row(
            CrateRow::new(
                n(1),
                vec![
                    CrateComponent::new(ProductId::new(3 + (bill + 1) % 400), n(2), SaleUnit::Case),
                    CrateComponent::new(ProductId::new(3 + (bill + 2) % 400), n(1), SaleUnit::Case),
                ],
            )
            .with_price("Synthetic Crate".into(), Money::from_minor_units(30_000)),
        );
    }
    let price = snapshot.products.iter().find(|x| x.id == p).unwrap().sell;
    Intent::PostSale(SaleIntent {
        draft,
        date,
        paid: Money::from_minor_units(if bill.is_multiple_of(4) { 0 } else { 500 }),
        prices: vec![price],
    })
}
