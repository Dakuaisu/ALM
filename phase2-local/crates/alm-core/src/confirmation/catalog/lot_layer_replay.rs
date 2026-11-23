//! Legacy case L11-01.
use std::num::NonZeroU32;

use crate::confirmation::{
    ConfirmationFixture, DraftId, Money, ProductId, ProvisionalDraft, PurchaseDraft, PurchaseLine,
    PurchaseOrderId, PurchaseProduct, PurchaseSupplier, ReceiveLine, ReceiveRequest, ReceiveResult,
    RequestId, Role, StockLotRow, StockLots, StockLotsError,
};
use crate::{BillDate, DraftCustomer, DraftCustomerId, DraftLine};

const PRODUCT: ProductId = ProductId::new(1);
const MISSING: ProductId = ProductId::new(2);
const SALE: DraftId = DraftId::new(1);
const SELL: Money = Money::from_minor_units(500);

fn date(day: u8) -> BillDate {
    BillDate::new(2025, 2, day).unwrap()
}

fn purchase_fixture(opening: u32, ordered: NonZeroU32) -> (ConfirmationFixture, PurchaseOrderId) {
    let mut fixture = ConfirmationFixture::new(PRODUCT, opening, SELL).unwrap();
    fixture
        .register_purchase_product(PurchaseProduct {
            id: PRODUCT,
            item: "Synthetic Orange Soda".into(),
            buy_rate: Money::from_minor_units(80),
        })
        .unwrap();
    let order = fixture
        .save_purchase_order(
            Role::Owner,
            &PurchaseDraft {
                supplier: Some(PurchaseSupplier {
                    id: None,
                    name: "Synthetic Orchard Supply".into(),
                }),
                date: Some(date(1)),
                note: String::new(),
                items: vec![PurchaseLine {
                    item: "Synthetic Orange Soda".into(),
                    product_id: Some(PRODUCT),
                    ordered,
                    rate: Money::from_minor_units(79),
                }],
            },
        )
        .unwrap();
    (fixture, order)
}

fn receive(
    fixture: &mut ConfirmationFixture,
    order: PurchaseOrderId,
    (day, quantity, rate): (u8, u32, i64),
) {
    assert_eq!(
        fixture.receive_purchase_order(ReceiveRequest {
            role: Role::Owner,
            id: order,
            date: Some(date(day)),
            today: date(20),
            lines: &[ReceiveLine {
                quantity,
                rate: Money::from_minor_units(rate)
            }],
        }),
        Ok(ReceiveResult::Received)
    );
}

fn sell_four(fixture: &mut ConfirmationFixture) {
    let draft = ProvisionalDraft::new(
        SALE,
        vec![DraftLine::new(PRODUCT, NonZeroU32::new(4).unwrap())],
    )
    .with_customer(DraftCustomer::Selected {
        id: DraftCustomerId::new("l11-01-buyer".into()),
        name: "Synthetic Buyer".into(),
    })
    .with_bill_date(Some(date(21)));
    fixture.register_draft(draft).unwrap();
    let review = fixture.review_named(Role::Owner, SALE, SELL).unwrap();
    fixture
        .confirm(Role::Owner, RequestId::new(1), &review)
        .unwrap();
}

fn open(fixture: &ConfirmationFixture, stage: &str) -> StockLots {
    let snapshot = fixture.clone();
    let lots = fixture.open_stock_lots(Role::Owner, PRODUCT).unwrap();
    assert_eq!(*fixture, snapshot);
    println!("l11_01 {stage}: {lots:?}");
    lots
}

fn known_row(day: u8, (quantity, rate, value): (u32, i64, i64)) -> StockLotRow {
    StockLotRow {
        date: Some(date(day)),
        remaining: NonZeroU32::new(quantity).unwrap(),
        unit_cost: Some(Money::from_minor_units(rate)),
        value: Some(Money::from_minor_units(value)),
    }
}

#[test]
fn l11_01_backdated_receipt_sale_and_cancel() {
    let (mut fixture, order) = purchase_fixture(0, NonZeroU32::new(8).unwrap());
    receive(&mut fixture, order, (17, 5, 230));
    receive(&mut fixture, order, (4, 3, 110));

    let before = open(&fixture, "backdated before sale");
    assert_eq!(before.on_hand, 8);
    assert_eq!(before.total_value, Some(Money::from_minor_units(1_480)));
    assert_eq!(
        before.rows,
        [known_row(4, (3, 110, 330)), known_row(17, (5, 230, 1_150))]
    );
    assert_eq!(fixture.catalog.lot_snapshot(PRODUCT), [0, 5, 3]);

    sell_four(&mut fixture);
    let posted = open(&fixture, "backdated after sale");
    assert_eq!(posted.on_hand, 4);
    assert_eq!(posted.total_value, Some(Money::from_minor_units(920)));
    assert_eq!(posted.rows, [known_row(17, (4, 230, 920))]);
    assert_eq!(fixture.catalog.lot_snapshot(PRODUCT), [0, 4, 0]);
    assert_eq!(fixture.catalog.allocated_cost(SALE), Some(Some(560)));

    fixture.cancel_sale(Role::Owner, SALE).unwrap();
    assert_eq!(open(&fixture, "backdated after cancellation"), before);
    assert_eq!(fixture.catalog.lot_snapshot(PRODUCT), [0, 5, 3]);
}

#[test]
fn l11_01_equal_date_receipts_preserve_append_order_both_ways() {
    for (first, second, sold, left) in [(110, 230, 560, 460), (230, 110, 800, 220)] {
        let (mut fixture, order) = purchase_fixture(0, NonZeroU32::new(6).unwrap());
        receive(&mut fixture, order, (8, 3, first));
        receive(&mut fixture, order, (8, 3, second));

        let stage = format!("equal date {first} then {second}");
        let before = open(&fixture, &format!("{stage} before sale"));
        assert_eq!(before.on_hand, 6);
        assert_eq!(before.total_value, Some(Money::from_minor_units(1_020)));
        assert_eq!(
            before.rows,
            [
                known_row(8, (3, first, 3 * first)),
                known_row(8, (3, second, 3 * second))
            ]
        );

        sell_four(&mut fixture);
        let posted = open(&fixture, &format!("{stage} after sale"));
        assert_eq!(posted.on_hand, 2);
        assert_eq!(posted.total_value, Some(Money::from_minor_units(left)));
        assert_eq!(posted.rows, [known_row(8, (2, second, left))]);
        assert_eq!(fixture.catalog.allocated_cost(SALE), Some(Some(sold)));

        fixture.cancel_sale(Role::Owner, SALE).unwrap();
        assert_eq!(
            open(&fixture, &format!("{stage} after cancellation")),
            before
        );
    }
}

#[test]
fn l11_01_preserves_unknown_cost_alongside_valued_receipt() {
    let (mut fixture, order) = purchase_fixture(2, NonZeroU32::new(1).unwrap());
    receive(&mut fixture, order, (8, 1, 110));

    assert_eq!(
        open(&fixture, "unknown opening cost"),
        StockLots {
            on_hand: 3,
            rows: vec![
                StockLotRow {
                    date: None,
                    remaining: NonZeroU32::new(2).unwrap(),
                    unit_cost: None,
                    value: None,
                },
                known_row(8, (1, 110, 110))
            ],
            total_value: None,
        }
    );
}

#[test]
fn l11_01_empty_layers_have_zero_total_value() {
    let fixture = ConfirmationFixture::new(PRODUCT, 0, SELL).unwrap();
    assert_eq!(
        open(&fixture, "empty stock"),
        StockLots {
            on_hand: 0,
            rows: Vec::new(),
            total_value: Some(Money::from_minor_units(0))
        }
    );
}

#[test]
fn l11_01_denies_manager_before_product_lookup() {
    let fixture = ConfirmationFixture::with_ordered_lots(PRODUCT, &[(1, SELL)], SELL).unwrap();
    let snapshot = fixture.clone();
    for id in [PRODUCT, MISSING] {
        assert_eq!(
            fixture.open_stock_lots(Role::Manager, id),
            Err(StockLotsError::AccessDenied)
        );
        assert_eq!(fixture, snapshot);
    }
    assert_eq!(
        fixture.open_stock_lots(Role::Owner, MISSING),
        Err(StockLotsError::ProductNotFound)
    );
    assert_eq!(fixture, snapshot);
}

#[test]
fn l11_01_checks_row_multiplication_and_total_addition() {
    let maximum = Money::from_minor_units(i64::MAX);
    let one = Money::from_minor_units(1);
    for lots in [&[(2, maximum)][..], &[(1, maximum), (1, one)][..]] {
        let fixture = ConfirmationFixture::with_ordered_lots(PRODUCT, lots, SELL).unwrap();
        let snapshot = fixture.clone();
        assert_eq!(
            fixture.open_stock_lots(Role::Owner, PRODUCT),
            Err(StockLotsError::Overflow)
        );
        assert_eq!(fixture, snapshot);
        assert_eq!(
            fixture.open_stock_lots(Role::Manager, PRODUCT),
            Err(StockLotsError::AccessDenied)
        );
        assert_eq!(fixture, snapshot);
    }
}
