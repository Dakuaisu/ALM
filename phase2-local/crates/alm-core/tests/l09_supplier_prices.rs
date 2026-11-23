use std::num::NonZeroU32;

use alm_core::confirmation::{
    ConfirmationFixture, ProductEntryError, PurchaseDraft, PurchaseLine, PurchaseProduct,
    PurchaseReceipt, PurchaseSupplier, PurchaseSupplierId, ReceiveLine, ReceiveRequest, RequestId,
    SupplierPriceEdit, SupplierPriceError,
};
use alm_core::{
    BillDate, DraftCustomer, DraftId, DraftLine, Money, ProductId, ProvisionalDraft, Role,
};

const OWNER: Role = Role::Owner;
const PRODUCT: ProductId = ProductId::new(9);
const SALE: DraftId = DraftId::new(21);
const ITEM: &str = "Synthetic Orange";
const RECEIPT_SUPPLIER: &str = "Grove Supply";

const fn money(minor: i64) -> Money {
    Money::from_minor_units(minor)
}

fn date(day: u8) -> BillDate {
    BillDate::new(2025, 1, day).unwrap()
}

fn fixture() -> ConfirmationFixture {
    let mut f =
        ConfirmationFixture::with_ordered_lots(PRODUCT, &[(10, money(800))], money(2_000)).unwrap();
    let mut product = f.open_product_entry(OWNER, PRODUCT).unwrap();
    product.item = ITEM.into();
    f.save_product_entry(OWNER, &product).unwrap();
    f.register_purchase_product(PurchaseProduct {
        id: PRODUCT,
        item: ITEM.into(),
        buy_rate: money(800),
    })
    .unwrap();
    let draft = PurchaseDraft {
        supplier: Some(PurchaseSupplier {
            id: None,
            name: RECEIPT_SUPPLIER.into(),
        }),
        date: Some(date(1)),
        note: "Existing order".into(),
        items: vec![PurchaseLine {
            item: ITEM.into(),
            product_id: Some(PRODUCT),
            ordered: NonZeroU32::new(5).unwrap(),
            rate: money(950),
        }],
    };
    let order = f.save_purchase_order(OWNER, &draft).unwrap();
    f.receive_purchase_order(ReceiveRequest {
        role: OWNER,
        id: order,
        date: Some(date(2)),
        today: date(3),
        lines: &[ReceiveLine {
            quantity: 2,
            rate: money(1_000),
        }],
    })
    .unwrap();
    let line = DraftLine::new(PRODUCT, NonZeroU32::new(3).unwrap());
    let draft = ProvisionalDraft::new(SALE, vec![line])
        .with_customer(DraftCustomer::Typed("Synthetic buyer".into()))
        .with_bill_date(Some(date(4)));
    f.register_draft(draft).unwrap();
    let review = f.review(OWNER, SALE).unwrap();
    f.confirm(OWNER, RequestId::new(51), &review).unwrap();
    f
}

const fn edit(supplier: PurchaseSupplierId, rate: Option<Money>) -> SupplierPriceEdit {
    SupplierPriceEdit {
        product: PRODUCT,
        supplier,
        rate,
    }
}

fn first_editor_rate(f: &ConfirmationFixture) -> Option<Money> {
    f.open_product_entry(OWNER, PRODUCT)
        .unwrap()
        .supplier_prices()[0]
        .rate
}

#[test]
fn l09_03_product_editor_adds_supplier_now_but_waits_for_product_save_to_store_quote() {
    let mut f = fixture();
    let mut form = f.open_product_entry(OWNER, PRODUCT).unwrap();
    assert_eq!(form.supplier_prices()[0].supplier.name, RECEIPT_SUPPLIER);
    assert_eq!(form.supplier_prices()[0].rate, Some(money(1_000)));

    let added = f
        .add_product_entry_supplier(OWNER, &mut form, "  Orchard Supply  ")
        .unwrap()
        .unwrap();
    assert_eq!(added.name, "Orchard Supply");
    assert!(f.purchase_suppliers(OWNER).unwrap().contains(&added));
    assert_eq!(form.supplier_prices()[1].supplier, added);
    assert_eq!(form.supplier_prices()[1].rate, None);
    let blank = f.compare_supplier_prices(OWNER, Some(PRODUCT)).unwrap();
    assert_eq!(blank.suppliers[0], added);
    assert_eq!(blank.rows[0].prices, [None, Some(money(1_000))]);

    form.set_supplier_price(added.id, Some(money(1_250)))
        .unwrap();
    assert_eq!(first_editor_rate(&f), None);

    let mut invalid = form.clone();
    invalid.item.clear();
    assert_eq!(
        f.save_product_entry(OWNER, &invalid),
        Err(ProductEntryError::ItemNameRequired)
    );
    assert_eq!(
        f.compare_supplier_prices(OWNER, Some(PRODUCT)).unwrap(),
        blank
    );

    f.save_product_entry(OWNER, &form).unwrap();
    assert_eq!(first_editor_rate(&f), Some(money(1_250)));
    let saved = f.compare_supplier_prices(OWNER, Some(PRODUCT)).unwrap();
    assert_eq!(
        saved.rows[0].prices,
        [Some(money(1_250)), Some(money(1_000))]
    );
    assert_eq!(
        f.purchase_cost(OWNER, PRODUCT, &added.name).unwrap(),
        Some((money(833), money(1_000), Some(money(1_250))))
    );
}

#[test]
fn l09_02_comparison_reopens_with_new_cheapest_after_direct_edit_and_preserves_history() {
    let mut f = fixture();
    let (review, sale) = f.current_sale(OWNER, SALE).unwrap();
    let (review, sale) = (review.clone(), sale.clone());
    assert_eq!(
        (sale.invoice, sale.quantity, sale.total),
        (1, 3, money(6_000))
    );
    assert_eq!(f.product_stock(PRODUCT), Some(9));
    let orders = f.purchase_orders(OWNER).unwrap().to_vec();
    assert_eq!(orders[0].items[0].line.ordered.get(), 5);
    let receipt = PurchaseReceipt {
        date: date(2),
        quantity: NonZeroU32::new(2).unwrap(),
        rate: money(1_000),
    };
    assert_eq!(orders[0].items[0].receipts(), &[receipt]);

    let preferred = f
        .add_comparison_supplier(OWNER, "  Market Supply ")
        .unwrap()
        .unwrap();
    assert_eq!(preferred.name, "Market Supply");
    assert!(f.purchase_suppliers(OWNER).unwrap().contains(&preferred));
    let blank = f
        .add_comparison_supplier(OWNER, "Unquoted Supply")
        .unwrap()
        .unwrap();
    let before = f.compare_supplier_prices(OWNER, None).unwrap();
    let from_receipt = &f.purchase_suppliers(OWNER).unwrap()[0];
    assert_eq!(
        before.suppliers,
        [preferred.clone(), blank, from_receipt.clone()]
    );
    assert_eq!(before.rows[0].prices, [None, None, Some(money(1_000))]);
    assert_eq!(before.rows[0].best, Some((from_receipt.id, money(1_000))));
    let from_receipt = from_receipt.id;
    f.edit_comparison_price(OWNER, edit(preferred.id, Some(money(1_250))))
        .unwrap();
    let rated = f.compare_supplier_prices(OWNER, None).unwrap();
    assert_eq!(
        rated.rows[0].prices,
        [Some(money(1_250)), None, Some(money(1_000))]
    );
    assert_eq!(rated.rows[0].best, Some((from_receipt, money(1_000))));
    assert_eq!(
        [0, 1, 2].map(|i| rated.rows[0].is_highlighted(i)),
        [false, false, true]
    );

    f.edit_comparison_price(OWNER, edit(preferred.id, Some(money(875))))
        .unwrap();

    let reopened = f.compare_supplier_prices(OWNER, None).unwrap();
    assert_eq!(reopened.rows[0].item, ITEM);
    assert_eq!(reopened.rows[0].our_buy, money(1_000));
    assert_eq!(
        reopened.rows[0].prices,
        [Some(money(875)), None, Some(money(1_000))]
    );
    assert_eq!(reopened.rows[0].best, Some((preferred.id, money(875))));
    assert_eq!(
        [0, 1, 2].map(|i| reopened.rows[0].is_highlighted(i)),
        [true, false, false]
    );
    assert_eq!(first_editor_rate(&f), Some(money(875)));
    assert_eq!(f.current_sale(OWNER, SALE).unwrap(), (&review, &sale));
    assert_eq!(f.ledger(), money(6_000));
    assert_eq!(f.product_stock(PRODUCT), Some(9));
    assert_eq!(f.purchase_orders(OWNER).unwrap(), orders);
    assert_eq!(
        f.purchase_cost(OWNER, PRODUCT, &preferred.name).unwrap(),
        Some((money(833), money(1_000), Some(money(875))))
    );
}

#[test]
fn l09_02_only_positive_quotes_qualify_and_typed_blank_or_zero_clears() {
    let mut f = fixture();
    let received = f.purchase_suppliers(OWNER).unwrap()[0].id;
    let negative = f
        .add_comparison_supplier(OWNER, "Negative Supply")
        .unwrap()
        .unwrap();
    let mut form = f.open_product_entry(OWNER, PRODUCT).unwrap();
    form.set_supplier_price(negative.id, Some(money(-500)))
        .unwrap();
    f.save_product_entry(OWNER, &form).unwrap();
    let mixed = f.compare_supplier_prices(OWNER, None).unwrap();
    assert_eq!(
        mixed.rows[0].prices,
        [Some(money(-500)), Some(money(1_000))]
    );
    assert_eq!(mixed.rows[0].best, Some((received, money(1_000))));

    f.edit_comparison_price(OWNER, edit(received, Some(money(0))))
        .unwrap();
    let no_positive = f.compare_supplier_prices(OWNER, None).unwrap();
    assert_eq!(no_positive.rows[0].prices, [Some(money(-500)), None]);
    assert_eq!(no_positive.rows[0].best, None);

    let mut reopened = f.open_product_entry(OWNER, PRODUCT).unwrap();
    reopened.set_supplier_price(negative.id, None).unwrap();
    f.save_product_entry(OWNER, &reopened).unwrap();
    let all = f.compare_supplier_prices(OWNER, None).unwrap();
    assert!(all.rows.is_empty());
    let selected = f.compare_supplier_prices(OWNER, Some(PRODUCT)).unwrap();
    assert_eq!(selected.rows[0].prices, [None, None]);
    assert_eq!(selected.rows[0].best, None);
}

#[test]
fn l09_02_and_03_manager_cannot_open_or_mutate_either_editor() {
    let mut f = fixture();
    let manager = Role::Manager;
    let received = f.purchase_suppliers(OWNER).unwrap()[0].id;
    let mut form = f.open_product_entry(OWNER, PRODUCT).unwrap();
    form.set_supplier_price(received, Some(money(875))).unwrap();
    let before = f.compare_supplier_prices(OWNER, None).unwrap();

    assert_eq!(
        f.open_product_entry(manager, PRODUCT),
        Err(ProductEntryError::AccessDenied)
    );
    assert_eq!(
        f.save_product_entry(manager, &form),
        Err(ProductEntryError::AccessDenied)
    );
    assert_eq!(
        f.add_product_entry_supplier(manager, &mut form, "Denied Editor"),
        Err(SupplierPriceError::AccessDenied)
    );
    assert_eq!(
        f.compare_supplier_prices(manager, None),
        Err(SupplierPriceError::AccessDenied)
    );
    assert_eq!(
        f.add_comparison_supplier(manager, "Denied Compare"),
        Err(SupplierPriceError::AccessDenied)
    );
    assert_eq!(
        f.edit_comparison_price(manager, edit(received, Some(money(750)))),
        Err(SupplierPriceError::AccessDenied)
    );
    assert_eq!(f.compare_supplier_prices(OWNER, None).unwrap(), before);
    assert_eq!(f.purchase_suppliers(OWNER).unwrap(), before.suppliers);
}
