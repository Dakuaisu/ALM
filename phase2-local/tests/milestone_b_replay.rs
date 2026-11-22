use std::num::{NonZeroI64, NonZeroU32};

use alm_phase2_local::confirmation::{
    ConfirmationError, ConfirmationFixture, CrateTemplate, CrateTemplateError, DailyMovementError,
    DailyMovementRow, ManualGodownError, ManualGodownInput, ProductTerms, PurchaseOrderStatus,
    RequestId, StockLotRow, StockLots, StockLotsError,
};
use alm_phase2_local::{
    BillDate, CrateComponent, DraftCustomer, DraftCustomerId, DraftId, DraftLine, Money, ProductId,
    ProvisionalDraft, Role, SaleUnit,
};

#[path = "milestone_b_replay/count_adjustment.rs"]
mod count_adjustment;
#[path = "milestone_b_replay/l07_03.rs"]
mod l07_03;
#[path = "milestone_b_replay/purchasing.rs"]
mod purchasing;
use purchasing::{receive_partials, save_planned_order};

const OWNER: Role = Role::Owner;
const MANAGER: Role = Role::Manager;
const A: ProductId = ProductId::new(1);
const B: ProductId = ProductId::new(2);
const ITEM_A: &str = "Synthetic A";
const ITEM_B: &str = "Synthetic B";
const SELL: Money = money(500);

const fn money(value: i64) -> Money {
    Money::from_minor_units(value)
}

fn qty(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).unwrap()
}

fn feb(day: u8) -> BillDate {
    BillDate::new(2025, 2, day).unwrap()
}

#[test]
fn milestone_b_reconciles_planned_purchase_backdated_receipts_and_crate_sale() {
    // Given the same zero-opening fixture for the entire replay.
    let mut fixture = ConfirmationFixture::new(A, 0, SELL).unwrap();
    fixture.add_product(B, 0, SELL).unwrap();
    let selection = save_planned_order(&mut fixture);
    let closed = receive_partials(&mut fixture, selection);
    assert_eq!(closed.status, PurchaseOrderStatus::Completed);
    count_adjustment::assert_reorder(&fixture, &[]);
    let before_sale = fifo(&fixture);
    assert_eq!(before_sale[0].rows, [lot(11, 3, 110), lot(12, 5, 130)]);
    assert_eq!(before_sale[1].rows, [lot(11, 1, 190), lot(12, 4, 210)]);

    fixture
        .set_checkout_product(
            B,
            ProductTerms {
                pack: qty(6),
                piece_price: money(70),
                case_price: SELL,
            },
        )
        .unwrap();
    let template = fixture
        .save_crate_template(
            OWNER,
            CrateTemplate {
                name: "Synthetic Mixed Crate".into(),
                components: vec![
                    CrateComponent::new(A, qty(2), SaleUnit::Case),
                    CrateComponent::new(B, qty(6), SaleUnit::Piece),
                ],
                price: SELL,
            },
        )
        .unwrap();
    let sale = DraftId::new(1);
    let mut bill = ProvisionalDraft::new(sale, vec![])
        .with_bill_date(Some(feb(13)))
        .with_customer(DraftCustomer::Selected {
            id: DraftCustomerId::new("milestone-b-buyer".into()),
            name: "Synthetic Buyer".into(),
        });
    assert_eq!(
        fixture.load_crate_template(MANAGER, template, qty(2), &mut bill),
        Err(CrateTemplateError::AccessDenied)
    );
    fixture
        .load_crate_template(OWNER, template, qty(2), &mut bill)
        .unwrap();
    fixture.register_draft(bill).unwrap();
    let review = fixture
        .review_checkout(OWNER, sale, &[], money(1_000))
        .unwrap();
    let demands = review.crate_packet_demands().unwrap().unwrap();
    assert_eq!(
        demands,
        [DraftLine::new(A, qty(4)), DraftLine::new(B, qty(2))]
    );
    let receipt = fixture.confirm(OWNER, RequestId::new(1), &review).unwrap();
    assert_eq!((receipt.quantity, receipt.total), (6, money(1_000)));
    assert_eq!(
        fixture.current_sale(MANAGER, sale),
        Err(ConfirmationError::AccessDenied)
    );
    assert_eq!(fixture.view_purchase_order(selection, &[]).unwrap(), closed);

    let after_sale = fifo(&fixture);
    assert_eq!(stock(&fixture), [4, 3]);
    assert_eq!(after_sale[0].rows, [lot(12, 4, 130)]);
    assert_eq!(after_sale[1].rows, [lot(12, 3, 210)]);
    let remaining_value = fifo_value(&after_sale);
    let depletion = fifo_value(&before_sale) - remaining_value;
    assert_eq!((remaining_value, depletion), (1_150, 860));
    println!(
        "Feb 13: crate packet demands={demands:?}; remaining={after_sale:?}; FIFO={remaining_value}, depletion={depletion}"
    );
    assert_eq!(
        fixture.open_stock_lots(MANAGER, A),
        Err(StockLotsError::AccessDenied)
    );
    assert_eq!(
        fixture.open_daily_movement_register(MANAGER, feb(13)),
        Err(DailyMovementError::AccessDenied)
    );
    for (day, rows) in (10..=14).zip(checked_register(&fixture)) {
        println!("Feb {day} register: {rows:?}");
    }

    let posted_ledgers = (fixture.ledger(), fixture.received_ledger());
    assert_eq!(posted_ledgers, (money(1_000), money(1_000)));
    for (text, expected, previous) in [("31", 31, None), ("47", 47, Some(31))] {
        let input = ManualGodownInput {
            item: ITEM_A,
            cases: text,
            note: "back rack",
        };
        assert_eq!(
            fixture.save_manual_godown(MANAGER, input),
            Err(ManualGodownError::AccessDenied)
        );
        let rows = fixture.open_manual_godown(OWNER).unwrap();
        assert_eq!(rows.first().map(|row| row.cases.get()), previous);
        fixture.save_manual_godown(OWNER, input).unwrap();
        assert_eq!(
            fixture.open_manual_godown(MANAGER),
            Err(ManualGodownError::AccessDenied)
        );
        let rows = fixture.open_manual_godown(OWNER).unwrap();
        let [row] = rows.as_slice() else {
            panic!("Godown Save or edit should leave exactly one row");
        };
        assert_eq!((row.item.as_str(), row.cases.get()), (ITEM_A, expected));
        assert_eq!(fixture.current_sale(OWNER, sale), Ok((&review, &receipt)));
        assert_eq!(
            (fixture.ledger(), fixture.received_ledger()),
            posted_ledgers
        );
        assert_eq!(fixture.posted_count(), 1);
        assert_eq!(stock(&fixture), [4, 3]);
        assert_eq!(fifo(&fixture), after_sale);
        assert_eq!(fixture.view_purchase_order(selection, &[]).unwrap(), closed);
        let _ = checked_register(&fixture);
        println!(
            "Godown reopened: {} cases for {}; billing, FIFO, register and PO unchanged",
            row.cases.get(),
            row.item
        );
    }

    let closed_orders = fixture.purchase_orders(OWNER).unwrap().to_vec();
    let godown: Vec<_> = fixture
        .open_manual_godown(OWNER)
        .unwrap()
        .into_iter()
        .cloned()
        .collect();
    count_adjustment::replay(&mut fixture, sale, &closed.supplier);

    assert_eq!(fixture.view_purchase_order(selection, &[]).unwrap(), closed);
    assert_eq!(fixture.purchase_orders(OWNER).unwrap(), closed_orders);
    let godown_after = fixture.open_manual_godown(OWNER).unwrap();
    assert!(godown_after.into_iter().eq(&godown));
    assert_eq!(fixture.current_sale(OWNER, sale), Ok((&review, &receipt)));
    assert_eq!(
        (fixture.ledger(), fixture.received_ledger()),
        posted_ledgers
    );
    assert_eq!(fixture.posted_count(), 1);
    println!(
        "After count: closed PO {} and manual godown (47 cases) unchanged",
        closed.number
    );
}

fn stock(fixture: &ConfirmationFixture) -> [u32; 2] {
    [A, B].map(|id| fixture.product_stock(id).unwrap())
}

fn fifo(fixture: &ConfirmationFixture) -> [StockLots; 2] {
    [A, B].map(|id| fixture.open_stock_lots(OWNER, id).unwrap())
}

fn fifo_value(lots: &[StockLots; 2]) -> i64 {
    lots.iter()
        .map(|product| product.total_value.unwrap().minor_units())
        .sum()
}

fn lot(day: u8, packets: u32, rate: i64) -> StockLotRow {
    StockLotRow {
        date: Some(feb(day)),
        remaining: qty(packets),
        unit_cost: Some(money(rate)),
        value: Some(money(i64::from(packets) * rate)),
    }
}

fn checked_register(fixture: &ConfirmationFixture) -> [Vec<DailyMovementRow<'_>>; 5] {
    let rows = [10, 11, 12, 13, 14].map(|day| {
        fixture
            .open_daily_movement_register(OWNER, feb(day))
            .unwrap()
    });
    assert!(rows[0].is_empty(), "PO date register: {:?}", rows[0]);
    for (day, a, b) in [
        (11_u8, (0, 3, 0, 3), (0, 1, 0, 1)),
        (12, (3, 5, 0, 8), (1, 4, 0, 5)),
        (13, (8, 0, 4, 4), (5, 0, 2, 3)),
        (14, (4, 0, 0, 4), (3, 0, 0, 3)),
    ] {
        assert_eq!(
            rows[usize::from(day - 10)],
            [movement((A, ITEM_A), a), movement((B, ITEM_B), b)],
            "Feb {day} register"
        );
    }
    rows
}

const fn movement(
    (product, item): (ProductId, &'static str),
    (opening, received, dispatched, closing): (i64, i64, i64, i64),
) -> DailyMovementRow<'static> {
    DailyMovementRow {
        product,
        item: Some(item),
        opening,
        received,
        dispatched,
        adjustment: 0,
        display_received: NonZeroI64::new(received),
        display_dispatched: NonZeroI64::new(dispatched),
        closing,
    }
}
