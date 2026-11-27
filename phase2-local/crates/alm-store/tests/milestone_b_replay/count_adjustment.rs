use std::num::NonZeroI64;

use alm_core::confirmation::manual_cashbook::ManualCashBook;
use alm_core::confirmation::{
    ConfirmationError, ConfirmationFixture, DailyMovementError, DailyMovementRow, DeadStockRow,
    PhysicalCount, PhysicalCountInput, ProductEntryError, PurchaseError, ReorderStatus,
    ReportsError, StockCountMovement, StockCountReason, StockLotRow, StockLots, StockLotsError,
};
use alm_core::{BillDate, DraftId, DraftLine, LocalSales, ProductId, ProvisionalDraft};

use super::manager_sqlite::{SqliteEvidence, databases};
use super::target_scratch::Scratch;
use super::{
    A, B, ITEM_A, ITEM_B, MANAGER, OWNER, checked_register, feb, fifo, lot, money, movement, qty,
    stock,
};
use alm_store::SqliteSales;

pub(super) fn assert_reorder(
    fixture: &ConfirmationFixture,
    expected: &[(ProductId, u32, u32, u64)],
) {
    let rows = fixture.open_reorder_planner(OWNER).unwrap();
    let actual: Vec<_> = rows
        .iter()
        .map(|row| {
            assert_eq!(row.status, ReorderStatus::Low);
            (
                row.product,
                row.on_hand,
                row.reorder_level,
                row.suggested_quantity.get(),
            )
        })
        .collect();
    assert_eq!(actual, expected);
    println!("Reorder rows (product, on hand, threshold, suggested): {actual:?}");
}

pub(super) fn replay(fixture: &mut ConfirmationFixture, sale: DraftId, supplier: &str) {
    // Given stock exactly at both reorder thresholds after the crate and godown checks.
    assert_reorder(fixture, &[(B, 3, 3, 6), (A, 4, 4, 8)]);
    let book = ManualCashBook::default();
    let january = BillDate::new(2025, 1, 31).unwrap();
    let before_month = fixture
        .monthly_profit_report(OWNER, &book, feb(14))
        .unwrap();
    let through_january = fixture
        .monthly_profit_report(OWNER, &book, january)
        .unwrap();
    assert_eq!(through_january.len(), 6);
    assert!(
        through_january
            .iter()
            .all(|row| row.bills.is_empty() && row.revenue == money(0) && row.cost == money(0))
    );
    let february = before_month.last().unwrap();
    assert_eq!((february.year, february.month), (2025, 2));
    assert_eq!(february.bills, [sale]);
    assert_eq!(
        (february.revenue, february.cost, february.gross),
        (money(1_000), money(860), money(140))
    );
    let before = assert_dead_stock(fixture, (3, 570));

    let input = PhysicalCountInput {
        product: B,
        counted: "5",
        fridge: None,
        date: feb(15),
        reason: Some(StockCountReason::CountCorrection),
        note: Some("  two loose packets found  "),
    };
    let initial_count = PhysicalCount {
        on_hand: 3,
        fridge: 0,
        movements: &[],
    };
    assert_eq!(fixture.open_physical_count(OWNER, B), Ok(initial_count));

    // When a Manager (DEC-034) records the extra packets, without any receipt or known cost.
    fixture.submit_physical_count(MANAGER, input).unwrap();
    assert_scoped_manager_draft(supplier);

    // Then B leaves the reorder list while A stays visible at equality.
    assert_eq!(stock(fixture), [4, 5]);
    assert_reorder(fixture, &[(A, 4, 4, 8)]);
    let count = fixture.open_physical_count(OWNER, B).unwrap();
    assert_eq!((count.on_hand, count.fridge), (5, 0));
    let adjustment = StockCountMovement {
        date: feb(15),
        from: 3,
        to: 5,
        delta: 2,
        reason: Some(StockCountReason::CountCorrection),
        note: Some("two loose packets found".into()),
    };
    assert_eq!(count.movements, [adjustment]);
    let untouched = fixture.open_physical_count(OWNER, A).unwrap();
    assert!(untouched.movements.is_empty());
    let lots = fifo(fixture);
    let found = StockLotRow {
        date: Some(feb(15)),
        remaining: qty(2),
        unit_cost: None,
        value: None,
    };
    let expected_lots = [
        StockLots {
            on_hand: 4,
            rows: vec![lot(12, 4, 130)],
            total_value: Some(money(520)),
        },
        StockLots {
            on_hand: 5,
            rows: vec![lot(12, 3, 210), found],
            total_value: None,
        },
    ];
    assert_eq!(lots, expected_lots);
    let _ = checked_register(fixture);
    let register = [15, 16].map(|day| {
        fixture
            .open_daily_movement_register(OWNER, feb(day))
            .unwrap()
    });
    let a = movement((A, ITEM_A), (4, 0, 0, 4));
    let adjusted_b = DailyMovementRow {
        adjustment: 2,
        display_received: NonZeroI64::new(2),
        ..movement((B, ITEM_B), (3, 0, 0, 5))
    };
    assert_eq!(register[0], [a, adjusted_b]);
    assert_eq!(register[1], [a, movement((B, ITEM_B), (5, 0, 0, 5))]);
    println!("Feb 15 physical count: {count:?}; FIFO after adjustment: {lots:?}");
    println!("Feb 15-16 registers (counts do not become receipts): {register:?}");

    let after = assert_dead_stock(fixture, (5, 950));
    assert_eq!((before, after), (1_010, 1_390));
    let after_month = fixture
        .monthly_profit_report(OWNER, &book, feb(16))
        .unwrap();
    assert_eq!(after_month, before_month);
    println!(
        "Dead stock Feb 14 cutoff: {before} -> {after}; FIFO A=520, B=630 known + 2 unknown; February frozen bill cost=860, gross=140 (minor units)"
    );

    assert_manager_boundary(fixture, &book, supplier);
}

fn assert_dead_stock(fixture: &ConfirmationFixture, (b_on_hand, b_value): (u32, i64)) -> i64 {
    for day in [12, 13] {
        let rows = fixture.dead_stock_report(OWNER, feb(day)).unwrap();
        assert!(rows.is_empty(), "Feb {day} cutoff excludes the Feb 13 sale");
    }
    let expected = [
        DeadStockRow {
            product: B,
            item: Some(ITEM_B.into()),
            on_hand: b_on_hand,
            last_sale: Some(feb(13)),
            buy_rate: money(190),
            locked_value: money(b_value),
        },
        DeadStockRow {
            product: A,
            item: Some(ITEM_A.into()),
            on_hand: 4,
            last_sale: Some(feb(13)),
            buy_rate: money(110),
            locked_value: money(440),
        },
    ];
    // Cutoff filters last-sale dates, not stock as of that date. Feb 14 sees the current count.
    let rows = fixture.dead_stock_report(OWNER, feb(14)).unwrap();
    assert_eq!(rows, expected);
    assert_eq!(fixture.dead_stock_report(OWNER, feb(16)).unwrap(), expected);
    rows.iter().map(|row| row.locked_value.minor_units()).sum()
}

fn assert_manager_boundary(fixture: &ConfirmationFixture, book: &ManualCashBook, supplier: &str) {
    let lots = fixture.open_stock_lots(MANAGER, B);
    assert_eq!(lots, Err(StockLotsError::AccessDenied));
    let dead_stock = fixture.dead_stock_report(MANAGER, feb(14));
    assert_eq!(dead_stock, Err(ConfirmationError::AccessDenied));
    let monthly = fixture.monthly_profit_report(MANAGER, book, feb(16));
    assert_eq!(
        monthly,
        Err(ReportsError::Confirmation(ConfirmationError::AccessDenied))
    );
    let register = fixture.open_daily_movement_register(MANAGER, feb(15));
    assert_eq!(register, Err(DailyMovementError::AccessDenied));
    // DEC-034: a Manager's reorder list carries no supplier or cost, and it counts stock.
    let reorder = fixture.open_reorder_planner(MANAGER).unwrap();
    assert!(
        reorder
            .iter()
            .all(|row| row.best_supplier.is_none() && row.estimated_cost.is_zero())
    );
    let count = fixture.open_physical_count(MANAGER, B);
    assert_eq!(count, fixture.open_physical_count(OWNER, B));
    let product = fixture.open_product_entry(MANAGER, B);
    assert_eq!(product, Err(ProductEntryError::AccessDenied));
    let owner_cost = fixture.purchase_cost(OWNER, B, supplier).unwrap();
    let expected_cost = (money(206), money(190), Some(money(190)));
    assert_eq!(owner_cost, Some(expected_cost));
    let manager_cost = fixture.purchase_cost(MANAGER, B, supplier);
    assert_eq!(manager_cost, Err(PurchaseError::AccessDenied));
}

fn assert_scoped_manager_draft(supplier: &str) {
    const ACCOUNT: &str = "Batch2-Manager";
    let scratch = Scratch::new("milestone-b-manager-");
    let draft_id = DraftId::new(2);
    let expected = ProvisionalDraft::new(
        draft_id,
        vec![DraftLine::new(A, qty(1)), DraftLine::new(B, qty(2))],
    )
    .with_bill_date(Some(feb(16)));
    {
        let mut writer = LocalSales::open_manager_account(scratch.path(), ACCOUNT).unwrap();
        writer.save_draft(expected.clone()).unwrap();
    }
    let reader = LocalSales::open_manager_account(scratch.path(), ACCOUNT).unwrap();
    assert_eq!(reader.reopen_draft(draft_id).unwrap(), Some(expected));

    let [path] = databases(scratch.path())
        .try_into()
        .expect("exactly one scoped Manager SQLite database");
    let evidence = SqliteEvidence::read(&path);
    evidence.assert_account_binding(ACCOUNT);
    assert_eq!(
        evidence.rows(),
        "draft|2\ndate|2|2025-02-16\nline|2|0|1|1\nline|2|1|2|2\n",
        "only the local draft header, date and ordered lines; no customer or crate rows"
    );
    evidence.assert_no_markers(&[
        "cost",
        "profit",
        "buy_rate",
        "locked_value",
        "supplier",
        "fifo",
        "margin",
        "invoice",
        "payment",
        "stock",
        supplier,
        "milestone-b-buyer",
        "Synthetic Buyer",
        "Synthetic Mixed Crate",
        "two loose packets found",
    ]);
}
