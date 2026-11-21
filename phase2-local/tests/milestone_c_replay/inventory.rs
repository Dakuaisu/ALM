use alm_phase2_local::confirmation::{
    ConfirmationFixture, PurchaseDraft, PurchaseError, PurchaseLine, PurchaseOrderSelection,
    PurchaseOrderStatus, PurchaseProduct, PurchaseReceipt, PurchaseSupplier, ReceiveLine,
    ReceiveRequest, ReceiveResult, StockLotRow, StockLotsError,
};

use super::{A, B, ITEM_A, ITEM_B, MANAGER, OWNER, feb, money, qty};

pub(super) fn receive_dated_stock(fixture: &mut ConfirmationFixture) {
    let products = [(A, ITEM_A, 4, 7_000), (B, ITEM_B, 2, 2_500)];
    for (id, name, _, rate) in products {
        fixture
            .register_purchase_product(PurchaseProduct {
                id,
                item: name.into(),
                buy_rate: money(rate),
            })
            .unwrap();
    }
    let draft = PurchaseDraft {
        supplier: Some(PurchaseSupplier {
            id: None,
            name: "Synthetic Supplier".into(),
        }),
        date: Some(feb(18)),
        note: String::new(),
        items: products
            .map(|(product, name, ordered, rate)| PurchaseLine {
                item: name.into(),
                product_id: Some(product),
                ordered: qty(ordered),
                rate: money(rate),
            })
            .into(),
    };
    let id = fixture.save_purchase_order(OWNER, &draft).unwrap();
    assert_eq!(
        [A, B].map(|product| fixture.product_stock(product)),
        [Some(0), Some(0)]
    );
    let selection = PurchaseOrderSelection {
        role: OWNER,
        id,
        number: 1,
    };

    let reverse_date_receipts = [
        (
            20,
            [(3, 7_000), (2, 2_500)],
            PurchaseOrderStatus::PartiallyReceived,
        ),
        (19, [(1, 6_000), (0, 2_500)], PurchaseOrderStatus::Completed),
    ];
    for (day, inputs, status) in reverse_date_receipts {
        let lines = inputs.map(|(quantity, rate)| ReceiveLine {
            quantity,
            rate: money(rate),
        });
        let request = ReceiveRequest {
            role: OWNER,
            id,
            date: Some(feb(day)),
            today: feb(20),
            lines: &lines,
        };
        assert_eq!(
            fixture.receive_purchase_order(ReceiveRequest {
                role: MANAGER,
                ..request
            }),
            Err(PurchaseError::AccessDenied)
        );
        assert_eq!(
            fixture.receive_purchase_order(request),
            Ok(ReceiveResult::Received)
        );
        let [order] = fixture.purchase_orders(OWNER).unwrap() else {
            panic!("expected one source PO");
        };
        let view = fixture.view_purchase_order(selection, &[]).unwrap();
        assert_eq!((order.id, view.status), (id, status));
        let saved = order
            .items
            .iter()
            .map(|line| line.receipts())
            .collect::<Vec<_>>();
        println!(
            "PO entered Feb {day}: {:?}; stored receipts={saved:?}",
            view.status
        );
    }
    let order = &fixture.purchase_orders(OWNER).unwrap()[0];
    assert_eq!(
        order.items[0].receipts(),
        [receipt(20, 3, 7_000), receipt(19, 1, 6_000)]
    );
    assert_eq!(order.items[1].receipts(), [receipt(20, 2, 2_500)]);
}

fn receipt(day: u8, count: u32, rate: i64) -> PurchaseReceipt {
    PurchaseReceipt {
        date: feb(day),
        quantity: qty(count),
        rate: money(rate),
    }
}

const OPEN_A: &[(u8, u32, i64, i64)] = &[(19, 1, 6_000, 6_000), (20, 3, 7_000, 21_000)];
const OPEN_B: &[(u8, u32, i64, i64)] = &[(20, 2, 2_500, 5_000)];
const SOLD_A: &[(u8, u32, i64, i64)] = &[(20, 2, 7_000, 14_000)];
const SOLD_B: &[(u8, u32, i64, i64)] = &[(20, 1, 2_500, 2_500)];

#[derive(Clone, Copy, Debug)]
pub(super) enum StockCheckpoint {
    Received,
    Posted,
    Edited,
    Cancelled,
}

pub(super) fn assert_stock(fixture: &ConfirmationFixture, checkpoint: StockCheckpoint) {
    let (on_hand, rows, values, total) = match checkpoint {
        StockCheckpoint::Received | StockCheckpoint::Cancelled => {
            ([4, 2], [OPEN_A, OPEN_B], [27_000, 5_000], 32_000)
        }
        StockCheckpoint::Posted => ([2, 2], [SOLD_A, OPEN_B], [14_000, 5_000], 19_000),
        StockCheckpoint::Edited => ([2, 1], [SOLD_A, SOLD_B], [14_000, 2_500], 16_500),
    };
    let lots = [A, B].map(|product| {
        assert_eq!(
            fixture.open_stock_lots(MANAGER, product),
            Err(StockLotsError::AccessDenied)
        );
        fixture.open_stock_lots(OWNER, product).unwrap()
    });
    let mut observed_total = 0;
    for (index, lot) in lots.iter().enumerate() {
        let expected_rows = rows[index]
            .iter()
            .map(|&(day, count, rate, value)| StockLotRow {
                date: Some(feb(day)),
                remaining: qty(count),
                unit_cost: Some(money(rate)),
                value: Some(money(value)),
            })
            .collect::<Vec<_>>();
        assert_eq!(lot.rows, expected_rows, "{checkpoint:?} product {index}");
        assert_eq!(lot.on_hand, on_hand[index]);
        assert_eq!(lot.total_value, Some(money(values[index])));
        observed_total += lot.total_value.unwrap().minor_units();
    }
    assert_eq!(observed_total, total);
    println!("FIFO {checkpoint:?}: value={observed_total}; remaining={lots:?}");
}
