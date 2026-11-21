use alm_phase2_local::confirmation::{ConfirmationFixture, ProductEntryError};
use alm_phase2_local::{Money, ProductId, Role};

#[test]
fn owner_setting_changes_blank_product_reorder_default() {
    let mut fixture =
        ConfirmationFixture::new(ProductId::new(1), 0, Money::from_minor_units(100)).unwrap();

    fixture
        .save_product_reorder_default(Role::Owner, 13)
        .unwrap();
    let mut form = fixture
        .begin_product_entry(Role::Owner, ProductId::new(2))
        .unwrap();
    form.item = "Synthetic Setting Control".into();
    fixture.save_product_entry(Role::Owner, &form).unwrap();

    assert_eq!(
        fixture
            .open_product_entry(Role::Owner, ProductId::new(2))
            .unwrap()
            .reorder_level,
        Some(13)
    );
}

#[test]
fn manager_cannot_change_product_reorder_default() {
    let mut fixture =
        ConfirmationFixture::new(ProductId::new(1), 0, Money::from_minor_units(100)).unwrap();
    assert_eq!(
        fixture.save_product_reorder_default(Role::Manager, 13),
        Err(ProductEntryError::AccessDenied)
    );
    let mut form = fixture
        .begin_product_entry(Role::Owner, ProductId::new(2))
        .unwrap();
    form.item = "Synthetic Denial Control".into();
    fixture.save_product_entry(Role::Owner, &form).unwrap();
    assert_eq!(
        fixture
            .open_product_entry(Role::Owner, ProductId::new(2))
            .unwrap()
            .reorder_level,
        Some(5)
    );
}
