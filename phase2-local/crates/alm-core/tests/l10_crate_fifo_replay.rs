use std::num::NonZeroU32;

use alm_core::confirmation::manual_cashbook::ManualCashBook;
use alm_core::confirmation::{
    ConfirmationError, ConfirmationFixture, CrateTemplate, ProductTerms, PurchaseDraft,
    PurchaseLine, PurchaseProduct, PurchaseSupplier, ReceiveLine, ReceiveRequest, ReceiveResult,
    RequestId, StockLotRow,
};
use alm_core::{
    BillDate, CrateComponent, DraftCustomer, DraftCustomerId, DraftId, DraftLine, Money, ProductId,
    ProvisionalDraft, Role, SaleUnit,
};

#[path = "l10_crate_fifo_replay/source.rs"]
mod source;

const PRODUCT: ProductId = ProductId::new(1);
const ITEM: &str = "Synthetic Repeated Lime";
const SALE: DraftId = DraftId::new(1);
const OWNER: Role = Role::Owner;

const fn money(minor: i64) -> Money {
    Money::from_minor_units(minor)
}

const fn qty(count: u32) -> NonZeroU32 {
    NonZeroU32::new(count).expect("positive fixture quantity")
}

fn feb(day: u8) -> BillDate {
    BillDate::new(2025, 2, day).unwrap()
}

fn received_layers() -> ConfirmationFixture {
    let mut fixture = ConfirmationFixture::new(PRODUCT, 0, money(500)).unwrap();
    let mut item = fixture.open_product_entry(OWNER, PRODUCT).unwrap();
    item.item = ITEM.into();
    fixture.save_product_entry(OWNER, &item).unwrap();
    fixture
        .register_purchase_product(PurchaseProduct {
            id: PRODUCT,
            item: ITEM.into(),
            buy_rate: money(999),
        })
        .unwrap();
    fixture
        .set_checkout_product(
            PRODUCT,
            ProductTerms {
                pack: qty(4),
                piece_price: money(99),
                case_price: money(500),
            },
        )
        .unwrap();
    let po = fixture
        .save_purchase_order(
            OWNER,
            &PurchaseDraft {
                supplier: Some(PurchaseSupplier {
                    id: None,
                    name: "Synthetic Supplier".into(),
                }),
                date: Some(feb(10)),
                note: String::new(),
                items: vec![PurchaseLine {
                    item: ITEM.into(),
                    product_id: Some(PRODUCT),
                    ordered: qty(8),
                    rate: money(300),
                }],
            },
        )
        .unwrap();
    for (day, quantity, rate) in [(12, 5, 275), (11, 3, 125)] {
        assert_eq!(
            fixture.receive_purchase_order(ReceiveRequest {
                role: OWNER,
                id: po,
                date: Some(feb(day)),
                today: feb(12),
                lines: &[ReceiveLine {
                    quantity,
                    rate: money(rate)
                }]
            }),
            Ok(ReceiveResult::Received)
        );
    }
    fixture
}

fn docket(fixture: &mut ConfirmationFixture, components: [CrateComponent; 2]) {
    let template = fixture
        .save_crate_template(
            OWNER,
            CrateTemplate {
                name: "Synthetic Mixed Crate".into(),
                components: components.to_vec(),
                price: money(500),
            },
        )
        .unwrap();
    let mut draft = ProvisionalDraft::new(SALE, vec![])
        .with_bill_date(Some(feb(13)))
        .with_customer(DraftCustomer::Selected {
            id: DraftCustomerId::new("synthetic-crate-buyer".into()),
            name: "Synthetic Buyer".into(),
        });
    fixture
        .load_crate_template(OWNER, template, qty(2), &mut draft)
        .unwrap();
    assert_eq!(draft.crate_rows()[0].components(), components);
    fixture.register_draft(draft).unwrap();
}

const fn component(count: u32, unit: SaleUnit) -> CrateComponent {
    CrateComponent::new(PRODUCT, qty(count), unit)
}

fn lot(day: u8, count: u32, rate: i64, value: i64) -> StockLotRow {
    StockLotRow {
        date: Some(feb(day)),
        remaining: qty(count),
        unit_cost: Some(money(rate)),
        value: Some(money(value)),
    }
}

fn movement(fixture: &ConfirmationFixture, day: u8) -> (i64, i64, i64, i64, i64) {
    let rows = fixture
        .open_daily_movement_register(OWNER, feb(day))
        .unwrap();
    let [row] = rows.as_slice() else {
        panic!("expected a single product movement row: {rows:?}")
    };
    assert_eq!((row.product, row.item), (PRODUCT, Some(ITEM)));
    (
        row.opening,
        row.received,
        row.dispatched,
        row.adjustment,
        row.closing,
    )
}

#[test]
fn l10_repeated_product_dispatch_uses_dated_fifo_and_reports_allocated_cost() {
    // Given the same dated layers for each crate, with the cheaper receipt entered last.
    for (second, packets, remaining, fifo_cost, gross, oracle) in [
        (
            component(1, SaleUnit::Case),
            2,
            (3, 825),
            925,
            75,
            [626.0, 926.0, 926.0, 6.0],
        ),
        (
            component(2, SaleUnit::Piece),
            1,
            (4, 1100),
            650,
            350,
            [500.0, 650.0, 652.0, 5.0],
        ),
    ] {
        let mut fixture = received_layers();
        docket(&mut fixture, [component(6, SaleUnit::Piece), second]);
        let before = fixture.open_stock_lots(OWNER, PRODUCT).unwrap();
        assert_eq!(before.rows, [lot(11, 3, 125, 375), lot(12, 5, 275, 1375)]);
        assert_eq!((before.on_hand, before.total_value), (8, Some(money(1750))));
        assert_eq!(movement(&fixture, 13), (8, 0, 0, 0, 8));

        // When the Owner reviews and confirms, only confirmation may dispatch stock.
        assert_eq!(
            fixture.review_checkout(Role::Manager, SALE, &[], money(1000)),
            Err(ConfirmationError::AccessDenied)
        );
        let review = fixture
            .review_checkout(OWNER, SALE, &[], money(1000))
            .expect("whole-packet repeated components are valid");
        assert_eq!(
            review.crate_packet_demands().unwrap().unwrap(),
            [
                DraftLine::new(PRODUCT, qty(3)),
                DraftLine::new(PRODUCT, qty(packets))
            ]
        );
        assert_eq!(review.unit_price(), money(500));
        assert_eq!(fixture.open_stock_lots(OWNER, PRODUCT).unwrap(), before);
        let request = RequestId::new(1);
        assert_eq!(
            fixture.confirm(Role::Manager, request, &review),
            Err(ConfirmationError::AccessDenied)
        );
        let receipt = fixture.confirm(OWNER, request, &review).unwrap();

        // Then receipt, movements, remaining lots, and saved report reconcile independently.
        assert_eq!(
            (receipt.quantity, receipt.total),
            (3 + packets, money(1000))
        );
        assert_eq!(fixture.current_sale(OWNER, SALE), Ok((&review, &receipt)));
        assert_eq!(movement(&fixture, 11), (0, 3, 0, 0, 3));
        assert_eq!(movement(&fixture, 12), (3, 5, 0, 0, 8));
        assert_eq!(
            movement(&fixture, 13),
            (8, 0, i64::from(3 + packets), 0, i64::from(remaining.0))
        );
        let after = fixture.open_stock_lots(OWNER, PRODUCT).unwrap();
        assert_eq!(after.rows, [lot(12, remaining.0, 275, remaining.1)]);
        assert_eq!(
            (after.on_hand, after.total_value),
            (remaining.0, Some(money(remaining.1)))
        );
        fixture.set_product_price(PRODUCT, money(800)).unwrap();
        let months = fixture
            .monthly_profit_report(OWNER, &ManualCashBook::default(), feb(13))
            .unwrap();
        let report = months.last().unwrap();
        assert_eq!(
            (report.year, report.month, report.bills.as_slice()),
            (2025, 2, &[SALE][..])
        );
        assert_eq!(
            (report.revenue, report.cost, report.gross),
            (money(1000), money(fifo_cost), money(gross))
        );
        assert_eq!(
            before.total_value.unwrap().minor_units() - after.total_value.unwrap().minor_units(),
            fifo_cost
        );

        source::assert_cost(packets, oracle);
        assert_ne!(report.cost, money(if packets == 2 { 626 } else { 500 }));
        println!(
            "L10-01 components=[3,{packets}] sale day movement={:?}; remaining fixed={remaining:?}, actual={after:?}; actual FIFO/report fixed={fifo_cost}, actual={}",
            movement(&fixture, 13),
            report.cost.minor_units()
        );
    }
}

#[test]
fn l10_repeated_components_preserve_fractional_and_shortage_guards() {
    for (comp, expected) in [
        (
            component(1, SaleUnit::Piece),
            ConfirmationError::InvalidAmount,
        ),
        (
            component(3, SaleUnit::Case),
            ConfirmationError::InsufficientStock,
        ),
    ] {
        let mut fixture = received_layers();
        docket(&mut fixture, [comp, comp]);
        let lots = fixture.open_stock_lots(OWNER, PRODUCT).unwrap();
        let outcome = fixture
            .review_checkout(OWNER, SALE, &[], money(1000))
            .and_then(|review| fixture.confirm(OWNER, RequestId::new(1), &review));
        assert_eq!(outcome, Err(expected));
        assert_eq!(fixture.open_stock_lots(OWNER, PRODUCT).unwrap(), lots);
        assert_eq!(movement(&fixture, 13), (8, 0, 0, 0, 8));
        assert_eq!((fixture.posted_count(), fixture.ledger()), (0, money(0)));
    }
    source::assert_unrepresentable_inputs();
}
