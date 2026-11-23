use super::{date, money, post};
use crate::confirmation::{
    ConfirmationError, ConfirmationFixture, DeadStockRow, PostedLineItem, ProductPriceInput,
    RequestId, StockLotRow,
};
use crate::{BillDate, DraftCustomer, DraftId, DraftLine, ProductId, ProvisionalDraft, Role};
use std::num::NonZeroU32;

const OWNER: Role = Role::Owner;
const LIVE: ProductId = ProductId::new(1);
const RETIRED: ProductId = ProductId::new(91);
const LABEL: &str = "Synthetic reused label";
const SALE: DraftId = DraftId::new(2);

fn fixture(product: ProductId) -> ConfirmationFixture {
    let mut f = ConfirmationFixture::with_ordered_lots(
        product,
        &[(2, money(80)), (3, money(140))],
        money(500),
    )
    .unwrap();
    let mut entry = f.open_product_entry(OWNER, product).unwrap();
    entry.item = LABEL.into();
    entry
        .apply_price(ProductPriceInput::Buy(money(325)))
        .unwrap();
    f.save_product_entry(OWNER, &entry).unwrap();
    f
}

fn row(on_hand: u32, last_sale: Option<BillDate>, value: i64) -> DeadStockRow {
    DeadStockRow {
        product: LIVE,
        item: Some(LABEL.into()),
        on_hand,
        last_sale,
        buy_rate: money(325),
        locked_value: money(value),
    }
}

#[test]
fn public_posting_rejects_undated_and_unresolved_drafts() {
    let mut f = fixture(LIVE);
    let draft = |product| {
        ProvisionalDraft::new(SALE, vec![DraftLine::new(product, NonZeroU32::MIN)])
            .with_customer(DraftCustomer::Typed("Synthetic buyer".into()))
    };
    f.register_draft(draft(LIVE)).unwrap();
    let review = f.review(OWNER, SALE).unwrap();
    let before = f.clone();
    assert_eq!(
        f.confirm(OWNER, RequestId::new(2), &review),
        Err(ConfirmationError::MissingBillDate)
    );
    assert_eq!(f, before);
    f.register_draft(draft(RETIRED).with_bill_date(Some(date(4))))
        .unwrap();
    let before = f.clone();
    assert_eq!(f.review(OWNER, SALE), Err(ConfirmationError::InvalidDraft));
    assert_eq!(
        f.current_sale(OWNER, SALE),
        Err(ConfirmationError::DraftNotFound)
    );
    assert_eq!(f.posted_count(), 0);
    assert_eq!(f, before);
    println!(
        "Unavailable through public posting: no date -> MissingBillDate; unknown product ID -> InvalidDraft before posting"
    );
}

#[test]
fn seeded_undated_sale_keeps_stock_depletion_without_a_last_sale_date() {
    for (older, remaining, estimate, fifo, last) in [
        (false, 4, 1_300, 500, None),
        (true, 3, 975, 420, Some(date(3))),
    ] {
        let mut f = fixture(LIVE);
        if older {
            post(&mut f, 1, 1, 1, 3);
        }
        post(&mut f, 2, 1, 1, 4);
        let dated = f.dead_stock_report(OWNER, date(4)).unwrap();
        assert!(
            dated.is_empty(),
            "action-created sale at cutoff must exclude"
        );
        assert_eq!(
            f.dead_stock_report(OWNER, date(5)).unwrap(),
            [row(remaining, Some(date(4)), estimate)]
        );

        // Seed missing historical date only; keep the actual posted line, receipt and FIFO depletion.
        let (review, _, _) = f.current_sales.get_mut(&SALE).unwrap();
        review.draft = review.draft.clone().with_bill_date(None);
        let before = f.clone();
        assert_eq!(f.lifecycle.get(&SALE).map(|entry| entry.0), Some(false));
        let (review, receipt) = f.current_sale(OWNER, SALE).unwrap();
        assert_eq!(review.draft().bill_date(), None);
        assert_eq!(
            (review.draft().lines()[0].product_id(), receipt.quantity),
            (LIVE, 1)
        );
        assert_eq!(f.posted_count(), if older { 2 } else { 1 });
        assert_eq!(
            f.posted_sale_items(OWNER, SALE).unwrap()[0].product_id,
            LIVE
        );

        let lots = f.open_stock_lots(OWNER, LIVE).unwrap();
        assert_eq!(
            (lots.on_hand, lots.total_value),
            (remaining, Some(money(fifo)))
        );
        let latest_layer = StockLotRow {
            date: None,
            remaining: NonZeroU32::new(3).unwrap(),
            unit_cost: Some(money(140)),
            value: Some(money(420)),
        };
        let first_layer = StockLotRow {
            date: None,
            remaining: NonZeroU32::MIN,
            unit_cost: Some(money(80)),
            value: Some(money(80)),
        };
        if older {
            assert_eq!(lots.rows, [latest_layer]);
            assert!(f.dead_stock_report(OWNER, date(3)).unwrap().is_empty());
        } else {
            assert_eq!(lots.rows, [first_layer, latest_layer]);
            assert_eq!(
                f.dead_stock_report(OWNER, date(3)).unwrap(),
                [row(remaining, last, estimate)]
            );
        }
        for day in [4, 5] {
            assert_eq!(
                f.dead_stock_report(OWNER, date(day)).unwrap(),
                [row(remaining, last, estimate)]
            );
        }
        assert_eq!(f, before);
        println!(
            "Seeded historical undated: current sale exists, quantity=1; Oct 4: remaining={remaining} last={last:?} buy=325 estimate={estimate} FIFO={fifo} paise"
        );
    }
}

#[test]
fn seeded_dangling_id_does_not_attach_to_live_product_with_same_name() {
    let mut f = fixture(LIVE);
    post(&mut f, 1, 1, 1, 4);
    let mut former = fixture(RETIRED);
    post(&mut former, 2, 91, 1, 6);

    // Seed only report history from a genuine posting against a separate, former catalog.
    f.current_sales.extend(former.current_sales);
    f.posted_sale_items.extend(former.posted_sale_items);
    f.lifecycle.extend(former.lifecycle);
    let before = f.clone();
    assert_eq!(f.product_stock(RETIRED), None);
    assert_eq!(f.find_product_by_exact_name(OWNER, LABEL), Ok(Some(LIVE)));
    assert_eq!(
        f.posted_sale_items(OWNER, SALE).unwrap(),
        [PostedLineItem {
            product_id: RETIRED,
            item: Some(LABEL.into())
        }]
    );
    let (review, historical) = f.current_sale(OWNER, SALE).unwrap();
    assert_eq!(
        (review.draft().bill_date(), historical.quantity),
        (Some(date(6)), 1)
    );
    assert!(f.dead_stock_report(OWNER, date(4)).unwrap().is_empty());
    for day in [5, 6, 7] {
        assert_eq!(
            f.dead_stock_report(OWNER, date(day)).unwrap(),
            [row(4, Some(date(4)), 1_300)]
        );
    }
    let lots = f.open_stock_lots(OWNER, LIVE).unwrap();
    assert_eq!((lots.on_hand, lots.total_value), (4, Some(money(500))));
    assert_eq!(f, before);
    println!(
        "Action-created ID 1 sold Oct 4; seeded ID 91 sold Oct 6 with same label. Oct 4 excludes ID 1; Oct 5/6/7 keep Oct 4, 4 packets, estimate=1300 vs FIFO=500 paise"
    );
}
