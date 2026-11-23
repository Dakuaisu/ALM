use std::num::NonZeroU32;

use crate::confirmation::{
    ConfirmationFixture, PurchaseDraft, PurchaseLine, PurchaseOrderId, PurchaseProduct,
    PurchaseSupplier, ReceiveLine, ReceiveRequest, ReceiveResult, SupplierPriceEdit,
};
use crate::{DraftId, DraftLine, Money, ProductId, ProvisionalDraft};

use super::{OWNER, ProductPriceInput, ProductSeed, date, money};

const PRODUCT: ProductId = ProductId::new(20);
const ITEM: &str = "Synthetic Receipt Soda";
const SUPPLIER: &str = "Synthetic Receipt Supply";
const ORIGINAL_BUY: Money = money(950);
const FIRST_RECEIPT: Money = money(1_235);

fn catalog_fixture() -> ConfirmationFixture {
    let mut fixture = ConfirmationFixture::new(ProductId::new(99), 0, money(100)).unwrap();
    ProductSeed {
        id: PRODUCT,
        item: ITEM,
        stock: 2,
        level: 5,
        buy: 950,
    }
    .insert(&mut fixture);
    fixture
}

fn purchase_fixture() -> (ConfirmationFixture, PurchaseOrderId) {
    let mut fixture = catalog_fixture();
    fixture
        .register_purchase_product(PurchaseProduct {
            id: PRODUCT,
            item: ITEM.into(),
            buy_rate: money(720),
        })
        .unwrap();
    let order = fixture
        .save_purchase_order(
            OWNER,
            &PurchaseDraft {
                supplier: Some(PurchaseSupplier {
                    id: None,
                    name: SUPPLIER.into(),
                }),
                date: Some(date()),
                note: String::new(),
                items: vec![PurchaseLine {
                    item: ITEM.into(),
                    product_id: Some(PRODUCT),
                    ordered: NonZeroU32::new(2).unwrap(),
                    rate: money(1100),
                }],
            },
        )
        .unwrap();
    (fixture, order)
}

fn receive_and_clear(fixture: &mut ConfirmationFixture, order: PurchaseOrderId, received: Money) {
    assert_eq!(
        fixture.receive_purchase_order(ReceiveRequest {
            role: OWNER,
            id: order,
            date: Some(date()),
            today: date(),
            lines: &[ReceiveLine {
                quantity: 1,
                rate: received,
            }],
        }),
        Ok(ReceiveResult::Received)
    );
    let supplier = fixture.purchase_suppliers(OWNER).unwrap()[0].id;
    let comparison = fixture
        .compare_supplier_prices(OWNER, Some(PRODUCT))
        .unwrap();
    assert_eq!(comparison.rows[0].best, Some((supplier, received)));
    assert_eq!(comparison.rows[0].our_buy, received);
    fixture
        .edit_comparison_price(
            OWNER,
            SupplierPriceEdit {
                product: PRODUCT,
                supplier,
                rate: None,
            },
        )
        .unwrap();
}

#[test]
fn l13_01_uses_receipt_buy_rate_after_clearing_its_comparison_quote() {
    let (mut fixture, order) = purchase_fixture();
    receive_and_clear(&mut fixture, order, FIRST_RECEIPT);
    let before = fixture.clone();

    assert_reorder_buy(&fixture, (12, FIRST_RECEIPT, money(14_820)));

    assert_eq!(fixture, before);
}

fn assert_reorder_buy(fixture: &ConfirmationFixture, expected: (u64, Money, Money)) {
    let rows = fixture.open_reorder_planner(OWNER).unwrap();
    let [row] = rows.as_slice() else {
        panic!("expected one reorder row, got {rows:?}");
    };
    assert_eq!(row.product, PRODUCT);
    assert_eq!(row.best_supplier, None);
    assert_eq!(
        (
            row.suggested_quantity.get(),
            row.unit_rate,
            row.estimated_cost
        ),
        expected
    );
}

#[test]
fn l13_01_reopens_receipt_buy_and_margin_without_changing_checkout_review() {
    let (mut fixture, order) = purchase_fixture();
    let draft_id = DraftId::new(1);
    fixture
        .register_draft(ProvisionalDraft::new(
            draft_id,
            vec![DraftLine::new(PRODUCT, NonZeroU32::MIN)],
        ))
        .unwrap();
    let checkout_before = fixture.review(OWNER, draft_id).unwrap();
    assert_eq!(checkout_before.unit_price(), money(1_050));

    receive_and_clear(&mut fixture, order, FIRST_RECEIPT);

    assert_eq!(fixture.review(OWNER, draft_id).unwrap(), checkout_before);
    let pricing = fixture
        .open_product_entry(OWNER, PRODUCT)
        .unwrap()
        .pricing();
    assert_eq!(
        (pricing.buy(), pricing.margin(), pricing.sell()),
        (FIRST_RECEIPT, money(-185), money(1_050))
    );
}

#[test]
fn l13_01_explicit_same_value_buy_edit_replaces_receipt_buy_on_save() {
    let (mut fixture, order) = purchase_fixture();
    let mut form = fixture.open_product_entry(OWNER, PRODUCT).unwrap();
    assert_eq!(form.pricing().buy(), ORIGINAL_BUY);
    receive_and_clear(&mut fixture, order, FIRST_RECEIPT);
    form.apply_price(ProductPriceInput::Buy(ORIGINAL_BUY))
        .unwrap();
    assert_reorder_buy(&fixture, (12, FIRST_RECEIPT, money(14_820)));

    fixture.save_product_entry(OWNER, &form).unwrap();

    assert_reorder_buy(&fixture, (12, ORIGINAL_BUY, money(11_400)));
    assert_eq!(
        fixture.supplier_price_product(PRODUCT),
        Some((ITEM, ORIGINAL_BUY))
    );
}

#[test]
fn l13_01_unrelated_product_save_keeps_receipt_buy_rate() {
    let (mut fixture, order) = purchase_fixture();
    receive_and_clear(&mut fixture, order, FIRST_RECEIPT);
    let mut form = fixture.open_product_entry(OWNER, PRODUCT).unwrap();
    form.brand = "Synthetic New Brand".into();
    form.apply_price(ProductPriceInput::Sell(money(1_600)))
        .unwrap();

    fixture.save_product_entry(OWNER, &form).unwrap();

    assert_reorder_buy(&fixture, (12, FIRST_RECEIPT, money(14_820)));
    let saved = fixture.open_product_entry(OWNER, PRODUCT).unwrap();
    assert_eq!(saved.brand, "Synthetic New Brand");
    assert_eq!(saved.pricing().sell(), money(1_600));
}

#[test]
fn l13_01_later_receipt_replaces_manual_buy_rate_without_changing_sell() {
    let (mut fixture, order) = purchase_fixture();
    receive_and_clear(&mut fixture, order, FIRST_RECEIPT);
    let mut form = fixture.open_product_entry(OWNER, PRODUCT).unwrap();
    form.apply_price(ProductPriceInput::Buy(money(845)))
        .unwrap();
    assert_eq!(form.pricing().sell(), money(660));
    fixture.save_product_entry(OWNER, &form).unwrap();
    assert_reorder_buy(&fixture, (12, money(845), money(10_140)));

    receive_and_clear(&mut fixture, order, money(1_475));

    assert_reorder_buy(&fixture, (11, money(1_475), money(16_225)));
    let pricing = fixture
        .open_product_entry(OWNER, PRODUCT)
        .unwrap()
        .pricing();
    assert_eq!(
        (pricing.buy(), pricing.margin(), pricing.sell()),
        (money(1_475), money(-815), money(660))
    );
}

#[test]
fn l13_01_uses_edited_catalog_buy_rate_for_catalog_only_product() {
    let mut fixture = catalog_fixture();
    let mut form = fixture.open_product_entry(OWNER, PRODUCT).unwrap();
    form.apply_price(ProductPriceInput::Buy(money(845)))
        .unwrap();
    fixture.save_product_entry(OWNER, &form).unwrap();

    let rows = fixture.open_reorder_planner(OWNER).unwrap();

    let [row] = rows.as_slice() else {
        panic!("expected one reorder row, got {rows:?}");
    };
    assert_eq!(row.best_supplier, None);
    assert_eq!(row.suggested_quantity.get(), 13);
    assert_eq!(
        (row.unit_rate, row.estimated_cost),
        (money(845), money(10_985))
    );
}
