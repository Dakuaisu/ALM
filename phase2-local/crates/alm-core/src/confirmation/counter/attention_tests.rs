use std::num::NonZeroU32;

use crate::confirmation::{
    ConfirmationError, ConfirmationFixture, OpeningBalance, PurchaseDraft, PurchaseLine,
    PurchaseOrderId, PurchaseOrderSelection, PurchaseOrderStatus, PurchaseProduct,
    PurchaseSupplier, ReceiveLine, ReceiveRequest, ReceiveResult,
};
use crate::{BillDate, DraftCustomerId, Money, ProductId, Role};

const PRODUCT: ProductId = ProductId::new(41);
const RATE: Money = Money::from_minor_units(100);

fn day() -> BillDate {
    BillDate::new(2025, 3, 14).unwrap()
}

fn read_stage(
    fixture: &ConfirmationFixture,
    order: PurchaseOrderId,
    label: &str,
) -> (usize, usize, usize) {
    let before = fixture.clone();
    let counter = fixture.counter(Role::Owner, day()).unwrap();
    assert_eq!(*fixture, before);
    let counts = (
        counter.attention.low_stock,
        counter.attention.open_purchase_orders,
        counter.attention.debtors,
    );
    assert_eq!(
        counter
            .debtor_ids
            .iter()
            .map(DraftCustomerId::value)
            .collect::<Vec<_>>(),
        ["due-a", "due-b"]
    );
    let stock = fixture.product_stock(PRODUCT).unwrap();
    assert_eq!(*fixture, before);
    let view = fixture
        .view_purchase_order(
            PurchaseOrderSelection {
                role: Role::Owner,
                id: order,
                number: 1,
            },
            &[],
        )
        .unwrap();
    assert_eq!(*fixture, before);
    let ledgers: Vec<_> = ["due-a", "due-b", "under-half"]
        .map(|id| {
            let customer = DraftCustomerId::new(id.into());
            let row = (
                id,
                fixture
                    .customer_ledger(Role::Owner, &customer)
                    .unwrap()
                    .closing
                    .minor_units(),
            );
            assert_eq!(*fixture, before);
            row
        })
        .into();
    assert_eq!(ledgers, [("due-a", 50), ("due-b", 151), ("under-half", 49)]);
    let expected = match label {
        "pending" => (5, PurchaseOrderStatus::Pending, 0, 3),
        "partial" => (6, PurchaseOrderStatus::PartiallyReceived, 1, 2),
        "completed" => (8, PurchaseOrderStatus::Completed, 3, 0),
        _ => panic!("unexpected replay stage"),
    };
    assert_eq!(
        (
            stock,
            view.status,
            view.lines[0].received,
            view.lines[0].remaining
        ),
        expected
    );
    println!(
        "attention {label}: counts={counts:?}; stock=[(41, {stock}, 5)]; order=PO-001 status={:?} received={} remaining={}; ledgers={ledgers:?}",
        view.status, view.lines[0].received, view.lines[0].remaining
    );
    counts
}

#[test]
fn attention_purchasing_replay() {
    let mut fixture = ConfirmationFixture::new(PRODUCT, 5, RATE).unwrap();
    fixture
        .register_purchase_product(PurchaseProduct {
            id: PRODUCT,
            item: "Synthetic item".into(),
            buy_rate: RATE,
        })
        .unwrap();
    for (id, paise) in [("due-a", 50), ("due-b", 151), ("under-half", 49)] {
        let customer = DraftCustomerId::new(id.into());
        fixture.register_customer(Role::Owner, &customer).unwrap();
        fixture
            .save_customer_opening(
                Role::Owner,
                &customer,
                OpeningBalance {
                    amount: Some(Money::from_minor_units(paise)),
                    date: Some(day()),
                },
            )
            .unwrap();
    }
    let order = fixture
        .save_purchase_order(
            Role::Owner,
            &PurchaseDraft {
                supplier: Some(PurchaseSupplier {
                    id: None,
                    name: "Synthetic supplier".into(),
                }),
                date: Some(day()),
                note: String::new(),
                items: vec![PurchaseLine {
                    item: "Synthetic item".into(),
                    product_id: Some(PRODUCT),
                    ordered: NonZeroU32::new(3).unwrap(),
                    rate: RATE,
                }],
            },
        )
        .unwrap();
    let before = fixture.clone();
    assert_eq!(
        fixture.counter(Role::Manager, day()),
        Err(ConfirmationError::AccessDenied)
    );
    assert_eq!(fixture, before);
    println!("attention populated Manager: AccessDenied; fixture unchanged");
    assert_eq!(read_stage(&fixture, order, "pending"), (1, 1, 2));
    for (quantity, expected, label) in [(1, (0, 1, 2), "partial"), (2, (0, 0, 2), "completed")] {
        assert_eq!(
            fixture.receive_purchase_order(ReceiveRequest {
                role: Role::Owner,
                id: order,
                date: Some(day()),
                today: day(),
                lines: &[ReceiveLine {
                    quantity,
                    rate: RATE
                }]
            }),
            Ok(ReceiveResult::Received)
        );
        assert_eq!(read_stage(&fixture, order, label), expected);
    }
    let empty = ConfirmationFixture::new(PRODUCT, 6, RATE).unwrap();
    let before = empty.clone();
    let attention = empty.counter(Role::Owner, day()).unwrap().attention;
    assert_eq!(
        (
            attention.low_stock,
            attention.open_purchase_orders,
            attention.debtors
        ),
        (0, 0, 0)
    );
    assert_eq!(empty, before);
    let stock = empty.product_stock(PRODUCT).unwrap();
    assert_eq!(stock, 6);
    assert_eq!(empty, before);
    let orders = empty.purchase_orders(Role::Owner).unwrap();
    assert!(orders.is_empty());
    assert_eq!(empty, before);
    println!(
        "attention zero control: counts=({}, {}, {}); stock=[(41, {stock}, 5)]; orders=[]; ledgers=[]",
        attention.low_stock, attention.open_purchase_orders, attention.debtors
    );
}
