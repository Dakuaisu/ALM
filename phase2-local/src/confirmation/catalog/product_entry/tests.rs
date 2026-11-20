use super::*;

const ID: ProductId = ProductId::new(42);
const OTHER: ProductId = ProductId::new(43);

fn fixture() -> ConfirmationFixture {
    ConfirmationFixture::with_ordered_lots(
        ID,
        &[
            (4, Money::from_minor_units(80)),
            (7, Money::from_minor_units(90)),
        ],
        Money::from_minor_units(200),
    )
    .unwrap()
    .with_product_reorder_default(13)
}

#[test]
fn l09_01_fresh_fixture_saves_blank_reorder_as_five_and_keeps_blank_tax() {
    let mut fixture = ConfirmationFixture::new(ID, 0, Money::from_minor_units(200)).unwrap();
    let mut form = fixture.begin_product_entry(Role::Owner, OTHER).unwrap();
    form.item = "Synthetic Soda".into();
    assert_eq!(form.reorder_level, None);

    let result = fixture.save_product_entry(Role::Owner, &form);

    assert_eq!(result, Ok(()));
    let saved = fixture.open_product_entry(Role::Owner, OTHER).unwrap();
    assert_eq!(saved.reorder_level, Some(5));
    assert_eq!(saved.tax, ProductTaxOverrides::default());
    assert_eq!(
        saved.effective_tax(),
        ResolvedProductTax {
            gst_rate: 1800,
            cess: 0,
            hsn: "22011010",
        }
    );
}

#[test]
fn l09_01_duplicate_warns_with_existing_name_without_mutating_fixture() {
    let mut fixture = fixture();
    let mut existing = fixture.open_product_entry(Role::Owner, ID).unwrap();
    existing.item = "Synthetic Soda".into();
    existing.brand = "Grove".into();
    fixture.save_product_entry(Role::Owner, &existing).unwrap();
    let mut duplicate = fixture.begin_product_entry(Role::Owner, OTHER).unwrap();
    duplicate.item = "  synthetic sODA  ".into();
    duplicate.brand = "Other Brand".into();
    let before = fixture.clone();

    let warning = fixture
        .save_product_entry(Role::Owner, &duplicate)
        .unwrap_err();

    assert_eq!(
        warning.duplicate_warning_text().as_deref(),
        Some(concat!(
            "A product called “Synthetic Soda” already exists — ",
            "use a different name (e.g. add the pack size)"
        ))
    );
    assert_eq!(fixture, before);
}

#[test]
fn l09_01_brand_filter_selects_exact_matches_without_mutating_fixture() {
    let mut fixture = fixture();
    let mut soda = fixture.open_product_entry(Role::Owner, ID).unwrap();
    soda.item = "Synthetic Soda".into();
    soda.brand = "Grove".into();
    fixture.save_product_entry(Role::Owner, &soda).unwrap();
    let mut juice = fixture.begin_product_entry(Role::Owner, OTHER).unwrap();
    juice.item = "Synthetic Juice".into();
    juice.brand = "grove".into();
    fixture.save_product_entry(Role::Owner, &juice).unwrap();
    let before = fixture.clone();

    let matches = fixture.filter_products_by_brand(Role::Owner, Some("Grove"));

    assert_eq!(matches, Ok(vec![ID]));
    assert_eq!(fixture, before);
}

#[test]
fn l09_01_existing_product_save_preserves_stock_lots_and_movement_epoch() {
    let mut fixture = fixture();
    let mut form = fixture.open_product_entry(Role::Owner, ID).unwrap();
    form.item = "Synthetic Water".into();
    form.apply_price(ProductPriceInput::Sell(Money::from_minor_units(225)))
        .unwrap();
    let before = &fixture.catalog.products[&ID.value()];
    let lots = before.lots.clone();
    let movement_epoch = fixture.movement_epoch;

    fixture.save_product_entry(Role::Owner, &form).unwrap();

    assert_eq!(fixture.catalog.products[&ID.value()].lots, lots);
    assert_eq!(fixture.product_stock(ID), Some(11));
    assert_eq!(fixture.movement_epoch, movement_epoch);
}

#[test]
fn l09_01_denied_saves_leave_the_entire_fixture_unchanged() {
    let mut fixture = fixture();
    let mut edit = fixture.open_product_entry(Role::Owner, ID).unwrap();
    edit.item = "Synthetic Water".into();
    let mut add = fixture
        .begin_product_entry(Role::Owner, ProductId::new(43))
        .unwrap();
    add.item = "Synthetic Juice".into();
    let before = fixture.clone();

    assert_eq!(
        fixture.save_product_entry(Role::Manager, &edit),
        Err(ProductEntryError::AccessDenied)
    );
    assert_eq!(
        fixture.save_product_entry(Role::Manager, &add),
        Err(ProductEntryError::AccessDenied)
    );
    assert_eq!(fixture, before);
}
