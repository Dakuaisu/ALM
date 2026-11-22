use super::*;
use crate::confirmation::{
    PurchaseDraft, PurchaseLine, PurchaseProduct, PurchaseSupplier, ReceiveLine, ReceiveRequest,
    ReceiveResult,
};

const PRODUCT: ProductId = ProductId::new(1);
const OWNER: Role = Role::Owner;

fn fixture() -> ConfirmationFixture {
    let mut f =
        ConfirmationFixture::with_ordered_lots(PRODUCT, &[(2, money(100))], money(100)).unwrap();
    let mut form = f.open_product_entry(OWNER, PRODUCT).unwrap();
    form.item = "Synthetic item".into();
    form.apply_price(ProductPriceInput::Buy(money(950)))
        .unwrap();
    f.save_product_entry(OWNER, &form).unwrap();
    f.register_purchase_product(PurchaseProduct {
        id: PRODUCT,
        item: "Synthetic item".into(),
        buy_rate: money(720),
    })
    .unwrap();
    f
}

#[test]
fn product_buy_edit_changes_report_without_changing_fifo() {
    let mut f = fixture();
    let mut form = f.open_product_entry(OWNER, PRODUCT).unwrap();
    form.apply_price(ProductPriceInput::Buy(money(845)))
        .unwrap();
    f.save_product_entry(OWNER, &form).unwrap();
    let before = f.clone();

    let catalog_buy = f
        .open_product_entry(OWNER, PRODUCT)
        .unwrap()
        .pricing()
        .buy();
    let lots = f.open_stock_lots(OWNER, PRODUCT).unwrap();
    let rows = f.dead_stock_report(OWNER, date(4)).unwrap();

    println!("product edit catalog_buy={catalog_buy:?} lots={lots:?} report={rows:?}");
    assert_eq!(catalog_buy, money(845));
    assert_eq!((lots.on_hand, lots.total_value), (2, Some(money(200))));
    assert_eq!(
        (rows[0].on_hand, rows[0].buy_rate, rows[0].locked_value),
        (2, money(845), money(1690))
    );
    assert_eq!(f, before);
}

#[test]
fn goods_receipt_changes_catalog_buy_but_report_is_not_fifo_value() {
    let mut f = fixture();
    let order = f
        .save_purchase_order(
            OWNER,
            &PurchaseDraft {
                supplier: Some(PurchaseSupplier {
                    id: None,
                    name: "Synthetic supplier".into(),
                }),
                date: Some(date(3)),
                note: String::new(),
                items: vec![PurchaseLine {
                    item: "Synthetic item".into(),
                    product_id: Some(PRODUCT),
                    ordered: NonZeroU32::new(1).unwrap(),
                    rate: money(1100),
                }],
            },
        )
        .unwrap();
    assert_eq!(
        f.receive_purchase_order(ReceiveRequest {
            role: OWNER,
            id: order,
            date: Some(date(3)),
            today: date(3),
            lines: &[ReceiveLine {
                quantity: 1,
                rate: money(1235)
            }],
        }),
        Ok(ReceiveResult::Received)
    );
    let before = f.clone();

    let catalog_buy = f
        .open_product_entry(OWNER, PRODUCT)
        .unwrap()
        .pricing()
        .buy();
    let lots = f.open_stock_lots(OWNER, PRODUCT).unwrap();
    let rows = f.dead_stock_report(OWNER, date(4)).unwrap();

    println!("receipt catalog_buy={catalog_buy:?} lots={lots:?} report={rows:?}");
    assert_eq!(catalog_buy, money(1235));
    assert_eq!((lots.on_hand, lots.total_value), (3, Some(money(1435))));
    assert_eq!(
        lots.rows
            .iter()
            .map(|row| (row.remaining.get(), row.unit_cost, row.value))
            .collect::<Vec<_>>(),
        [
            (2, Some(money(100)), Some(money(200))),
            (1, Some(money(1235)), Some(money(1235)))
        ]
    );
    assert_eq!(
        (rows[0].on_hand, rows[0].buy_rate, rows[0].locked_value),
        (3, money(1235), money(3705))
    );
    assert_eq!(f, before);
}
