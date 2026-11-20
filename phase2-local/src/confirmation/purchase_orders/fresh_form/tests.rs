use super::*;
use crate::{Money, ProductId, Role};
use std::num::NonZeroU32;

const OWNER: Role = Role::Owner;
const MANAGER: Role = Role::Manager;
const PRODUCT: ProductId = ProductId::new(8);
const BUY_RATE: Money = Money::from_minor_units(350);

fn date() -> BillDate {
    BillDate::new(2025, 3, 14).unwrap()
}

fn fixture() -> crate::confirmation::ConfirmationFixture {
    crate::confirmation::ConfirmationFixture::with_ordered_lots(
        PRODUCT,
        &[(7, BUY_RATE)],
        Money::from_minor_units(500),
    )
    .unwrap()
}

fn register_product(fixture: &mut crate::confirmation::ConfirmationFixture) {
    fixture
        .register_purchase_product(super::super::super::PurchaseProduct {
            id: PRODUCT,
            item: "Synthetic Orange".into(),
            buy_rate: BUY_RATE,
        })
        .unwrap();
}

#[test]
fn l13_01_owner_opens_repeatable_fresh_form_without_fixture_mutation() {
    let mut fixture = fixture();
    register_product(&mut fixture);
    let before = fixture.clone();

    let first = fixture.begin_fresh_purchase_order(OWNER, date()).unwrap();
    let second = fixture.begin_fresh_purchase_order(OWNER, date()).unwrap();

    println!("L13-01 fresh defaults: {first:?}");
    assert_eq!(first, second);
    assert_eq!(first.preview_number, "PO-001");
    assert_eq!(first.date, date());
    assert_eq!(first.supplier, None);
    assert_eq!(first.note, "");
    assert_eq!(first.items.len(), 1);
    assert_eq!(
        first.items[0],
        FreshPurchaseOrderLine {
            item: String::new(),
            product_id: None,
            ordered: String::new(),
            rate: String::new(),
        }
    );
    assert_eq!(fixture, before);
}

#[test]
fn l13_01_fresh_preview_follows_populated_book_without_reserving_next_number() {
    let mut fixture = fixture();
    register_product(&mut fixture);
    let saved = crate::confirmation::PurchaseDraft {
        supplier: Some(super::super::super::PurchaseSupplier {
            id: None,
            name: "Synthetic Grove".into(),
        }),
        date: Some(date()),
        note: "Existing order".into(),
        items: vec![super::super::super::PurchaseLine {
            item: "Synthetic Orange".into(),
            product_id: Some(PRODUCT),
            ordered: NonZeroU32::new(3).unwrap(),
            rate: BUY_RATE,
        }],
    };
    fixture.save_purchase_order(OWNER, &saved).unwrap();
    let before = fixture.clone();

    let fresh = fixture.begin_fresh_purchase_order(OWNER, date()).unwrap();

    println!("L13-01 populated-book control: {fresh:?}");
    assert_eq!(fresh.preview_number, "PO-002");
    assert_eq!(fresh.supplier, None);
    assert_eq!(fresh.note, "");
    assert_eq!(fresh.items[0].item, "");
    assert_eq!(fresh.items[0].rate, "");
    assert_eq!(fresh.items[0].ordered, "");
    assert_eq!(fixture, before);
}

#[test]
fn l13_01_manager_cannot_open_fresh_form_and_fixture_is_unchanged() {
    let mut fixture = fixture();
    register_product(&mut fixture);
    let before = fixture.clone();

    let denied = fixture.begin_fresh_purchase_order(MANAGER, date());

    assert_eq!(
        denied,
        Err(super::super::super::PurchaseError::AccessDenied)
    );
    assert_eq!(fixture, before);
}
