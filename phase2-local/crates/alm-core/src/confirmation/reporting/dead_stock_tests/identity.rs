use super::*;
use crate::confirmation::{ProductTerms, PurchaseProduct};
use crate::{CrateComponent, CrateRow, SaleUnit};

#[test]
fn l16_05_direct_and_crate_follow_retained_product_id_after_rename() {
    for crate_sale in [false, true] {
        let mut f = ConfirmationFixture::new(ProductId::new(99), 0, money(100)).unwrap();
        product(&mut f, 1, 3, 100);
        product(&mut f, 2, 3, 100);
        let old_name = f
            .open_product_entry(Role::Owner, ProductId::new(1))
            .unwrap()
            .item;
        if crate_sale {
            f.set_checkout_product(
                ProductId::new(1),
                ProductTerms {
                    pack: NonZeroU32::new(10).unwrap(),
                    piece_price: money(1),
                    case_price: money(100),
                },
            )
            .unwrap();
            f.set_crate_rate(money(100)).unwrap();
            let mut draft = ProvisionalDraft::new(DraftId::new(1), vec![])
                .with_customer(DraftCustomer::Selected {
                    id: DraftCustomerId::new("buyer".into()),
                    name: "Synthetic buyer".into(),
                })
                .with_bill_date(Some(date(5)));
            draft
                .add_crate_row(CrateRow::new(
                    NonZeroU32::new(2).unwrap(),
                    vec![CrateComponent::new(
                        ProductId::new(1),
                        NonZeroU32::new(5).unwrap(),
                        SaleUnit::Piece,
                    )],
                ))
                .unwrap();
            f.register_draft(draft).unwrap();
            let review = f
                .review_checkout(Role::Owner, DraftId::new(1), &[], money(1))
                .unwrap();
            assert_eq!(
                review.crate_packet_demands().unwrap().unwrap()[0]
                    .quantity()
                    .get(),
                1
            );
            f.confirm(Role::Owner, RequestId::new(1), &review).unwrap();
        } else {
            post(&mut f, 1, 1, 1, 5);
        }
        let mut form = f
            .open_product_entry(Role::Owner, ProductId::new(1))
            .unwrap();
        form.item = "Renamed".into();
        f.save_product_entry(Role::Owner, &form).unwrap();
        let before = f.clone();
        let stored = f.posted_sale_items(Role::Owner, DraftId::new(1)).unwrap();
        let rows = f.dead_stock_report(Role::Owner, date(4)).unwrap();
        println!(
            "L16-05 crate={crate_sale} stored={stored:?} old_name={old_name:?} stock1={:?} stock2={:?} rows={rows:?}",
            f.product_stock(ProductId::new(1)),
            f.product_stock(ProductId::new(2))
        );
        assert_eq!(stored[0].product_id, ProductId::new(1));
        assert_eq!(stored[0].item.as_deref(), Some(old_name.as_str()));
        assert_eq!(f.product_stock(ProductId::new(1)), Some(2));
        assert_eq!(
            rows.iter()
                .map(|row| (
                    row.product.value(),
                    row.on_hand,
                    row.locked_value.minor_units()
                ))
                .collect::<Vec<_>>(),
            [(2, 3, 300)]
        );
        assert_eq!(f, before);
    }
}

#[test]
fn catalog_buy_wins_over_conflicting_registered_buy() {
    let mut f = ConfirmationFixture::new(ProductId::new(99), 0, money(100)).unwrap();
    product(&mut f, 1, 2, 950);
    f.register_purchase_product(PurchaseProduct {
        id: ProductId::new(1),
        item: "Registered name".into(),
        buy_rate: money(720),
    })
    .unwrap();
    let before = f.clone();
    let catalog = f
        .open_product_entry(Role::Owner, ProductId::new(1))
        .unwrap()
        .pricing()
        .buy();
    let registered = f.supplier_price_product(ProductId::new(1)).unwrap().1;
    let rows = f.dead_stock_report(Role::Owner, date(4)).unwrap();
    println!("conflicting buys catalog={catalog:?} registered={registered:?} rows={rows:?}");
    assert_eq!((catalog, registered), (money(950), money(720)));
    assert_eq!(
        (
            rows[0].buy_rate.minor_units(),
            rows[0].locked_value.minor_units()
        ),
        (950, 1900)
    );
    assert_eq!(f, before);
}
