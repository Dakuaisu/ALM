use std::num::NonZeroU32;

use alm_core::confirmation::{
    ConfirmationFixture, ProductEntry, ProductEntryError, ProductPriceInput, ProductTaxOverrides,
    ResolvedProductTax,
};
use alm_core::{DraftId, DraftLine, Money, ProductId, ProvisionalDraft, Role};

const OWNER: Role = Role::Owner;
const SEED: ProductId = ProductId::new(1);
const SODA: ProductId = ProductId::new(2);
const JUICE: ProductId = ProductId::new(3);
const OTHER: ProductId = ProductId::new(4);

const fn money(minor: i64) -> Money {
    Money::from_minor_units(minor)
}

fn fixture() -> ConfirmationFixture {
    ConfirmationFixture::with_ordered_lots(SEED, &[(4, money(80)), (7, money(90))], money(200))
        .unwrap()
        .with_product_reorder_default(13)
}

fn entry(fixture: &ConfirmationFixture, id: ProductId, item: &str, brand: &str) -> ProductEntry {
    let mut form = fixture.begin_product_entry(OWNER, id).unwrap();
    form.item = item.into();
    form.brand = brand.into();
    form
}

fn pricing(form: &ProductEntry) -> (Money, Money, Money) {
    let price = form.pricing();
    (price.buy(), price.margin(), price.sell())
}

const fn tax(gst_rate: u32, cess: u32, hsn: &str) -> ResolvedProductTax<'_> {
    ResolvedProductTax {
        gst_rate,
        cess,
        hsn,
    }
}

#[test]
fn l09_01_brand_filter_is_exact_and_does_not_change_entries() {
    let mut fixture = fixture();
    let soda = entry(&fixture, SODA, "  Synthetic Soda ", " Grove ");
    let juice = entry(&fixture, JUICE, "Synthetic Juice", "grove");
    fixture.save_product_entry(OWNER, &soda).unwrap();
    fixture.save_product_entry(OWNER, &juice).unwrap();
    let before = fixture.open_product_entry(OWNER, SODA).unwrap();

    let matching = |brand| fixture.filter_products_by_brand(OWNER, brand).unwrap();
    assert_eq!(matching(Some("Grove")), [SODA]);
    assert_eq!(matching(Some("grove")), [JUICE]);
    assert!(matching(Some("Gro")).is_empty());
    assert_eq!(matching(None), [SODA, JUICE]);
    assert_eq!(fixture.open_product_entry(OWNER, SODA).unwrap(), before);
    assert_eq!((&*before.item, &*before.brand), ("Synthetic Soda", "Grove"));
    assert_eq!(fixture.product_stock(SODA), Some(0));
}

#[test]
fn l09_01_duplicate_warning_is_global_and_save_excludes_its_own_id() {
    let mut fixture = fixture();
    let soda = entry(&fixture, SODA, "Synthetic Soda", "Grove");
    fixture.save_product_entry(OWNER, &soda).unwrap();
    let duplicate = entry(&fixture, OTHER, "  sYNTHETIC sODA  ", "Other Brand");

    assert_eq!(
        fixture.save_product_entry(OWNER, &duplicate),
        Err(ProductEntryError::DuplicateName {
            existing: "Synthetic Soda".into(),
        })
    );
    assert_eq!(fixture.product_stock(OTHER), None);
    let mut reopened = fixture.open_product_entry(OWNER, SODA).unwrap();
    reopened.item = "  SYNTHETIC SODA  ".into();
    fixture.save_product_entry(OWNER, &reopened).unwrap();
    assert_eq!(
        fixture.open_product_entry(OWNER, SODA).unwrap().item,
        "SYNTHETIC SODA"
    );
}

#[test]
fn l09_01_price_events_save_exact_values_to_the_catalog_and_reopen_them() {
    let mut fixture = fixture();
    let draft_id = DraftId::new(1);
    fixture
        .register_draft(ProvisionalDraft::new(
            draft_id,
            vec![DraftLine::new(SEED, NonZeroU32::MIN)],
        ))
        .unwrap();
    let old = fixture.review(OWNER, draft_id).unwrap();
    let mut form = fixture.open_product_entry(OWNER, SEED).unwrap();
    assert_eq!(form.pricing().sell(), money(200));
    form.item = "Synthetic Stocked Soda".into();
    form.brand = "Grove".into();

    form.apply_price(ProductPriceInput::Buy(money(1234)))
        .unwrap();
    assert_eq!(form.pricing().sell(), money(1434));
    form.apply_price(ProductPriceInput::Margin(money(267)))
        .unwrap();
    assert_eq!(form.pricing().sell(), money(1501));
    fixture.save_product_entry(OWNER, &form).unwrap();
    let mut form = fixture.open_product_entry(OWNER, SEED).unwrap();
    assert_eq!(pricing(&form), (money(1234), money(267), money(1501)));

    form.apply_price(ProductPriceInput::Sell(money(1425)))
        .unwrap();
    assert_eq!(form.pricing().margin(), money(191));
    let prior = fixture.review(OWNER, draft_id).unwrap();
    assert_eq!(prior.amounts().unwrap().subtotal, money(1501));
    fixture.save_product_entry(OWNER, &form).unwrap();
    let saved = fixture.open_product_entry(OWNER, SEED).unwrap();
    assert_eq!(pricing(&saved), (money(1234), money(191), money(1425)));
    assert_eq!(fixture.product_stock(SEED), Some(11));
    let review = fixture.review(OWNER, draft_id).unwrap();
    assert_eq!(old.amounts().unwrap().subtotal, money(200));
    assert_eq!(review.amounts().unwrap().subtotal, money(1425));
}

#[test]
fn l09_01_blank_reorder_defaults_and_blank_taxes_remain_optional() {
    let mut fixture = fixture();
    let mut soda = entry(&fixture, SODA, "Synthetic Soda", "Grove");
    soda.tax.hsn = Some("  ".into());
    fixture.save_product_entry(OWNER, &soda).unwrap();
    let saved = fixture.open_product_entry(OWNER, SODA).unwrap();
    assert_eq!(saved.reorder_level, Some(13));
    assert_eq!(saved.tax, ProductTaxOverrides::default());
    assert_eq!(saved.effective_tax(), tax(1800, 0, "22011010"));

    let mut juice = entry(&fixture, JUICE, "Synthetic Mango Juice", "Grove");
    juice.reorder_level = Some(0);
    juice.tax = ProductTaxOverrides {
        gst_rate: Some(0),
        cess: Some(0),
        hsn: Some(" 12345678 ".into()),
    };
    fixture.save_product_entry(OWNER, &juice).unwrap();
    let mut saved = fixture.open_product_entry(OWNER, JUICE).unwrap();
    assert_eq!(saved.reorder_level, Some(0));
    assert_eq!(saved.tax.gst_rate, Some(0));
    assert_eq!(saved.tax.cess, Some(0));
    assert_eq!(saved.tax.hsn.as_deref(), Some("12345678"));
    assert_eq!(saved.effective_tax(), tax(0, 0, "12345678"));

    saved.tax.cess = Some(125);
    saved.tax.hsn = None;
    fixture.save_product_entry(OWNER, &saved).unwrap();
    let saved = fixture.open_product_entry(OWNER, JUICE).unwrap();
    assert_eq!(saved.effective_tax(), tax(0, 125, "22029920"));
}

#[test]
fn l09_01_manager_gets_no_product_entry_data_and_writes_change_nothing() {
    let mut fixture = fixture();
    let mut soda = entry(&fixture, SODA, "Synthetic Soda", "Grove");
    soda.apply_price(ProductPriceInput::Buy(money(902)))
        .unwrap();
    soda.apply_price(ProductPriceInput::Margin(money(133)))
        .unwrap();
    fixture.save_product_entry(OWNER, &soda).unwrap();
    let before = fixture.open_product_entry(OWNER, SODA).unwrap();
    let mut edited = before.clone();
    edited
        .apply_price(ProductPriceInput::Sell(money(777)))
        .unwrap();
    let added = entry(&fixture, OTHER, "Synthetic Water", "Grove");
    let manager = Role::Manager;

    let denied = ProductEntryError::AccessDenied;
    assert_eq!(
        fixture.begin_product_entry(manager, OTHER),
        Err(denied.clone())
    );
    assert_eq!(
        fixture.filter_products_by_brand(manager, Some("Grove")),
        Err(denied.clone())
    );
    assert_eq!(
        fixture.open_product_entry(manager, SODA),
        Err(denied.clone())
    );
    assert_eq!(
        fixture.open_product_entry(manager, OTHER),
        Err(denied.clone())
    );
    assert_eq!(
        fixture.save_product_entry(manager, &edited),
        Err(denied.clone())
    );
    assert_eq!(fixture.save_product_entry(manager, &added), Err(denied));
    assert_eq!(fixture.open_product_entry(OWNER, SODA).unwrap(), before);
    assert_eq!(
        fixture.filter_products_by_brand(OWNER, None).unwrap(),
        [SODA]
    );
    assert_eq!(fixture.product_stock(SODA), Some(0));
    assert_eq!(fixture.product_stock(OTHER), None);
}
