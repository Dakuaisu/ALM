use alm_store::confirmation::{
    ConfirmationFixture, PurchaseError, PurchaseOrderSelection, PurchaseOrderStatus,
    PurchaseOrderView,
};

use super::{A, B, MANAGER, OWNER, SELL, feb, fifo, fifo_value, stock};

#[path = "../common/partial_po/plan.rs"]
mod plan;
#[path = "../common/partial_po/receipts.rs"]
mod receipts;

pub(super) use plan::save_planned_order;
pub(super) use receipts::post_partials;

pub(super) fn receive_partials(
    fixture: &mut ConfirmationFixture,
    selection: PurchaseOrderSelection,
) -> PurchaseOrderView {
    let partial = post_partials(fixture, selection);
    let posted_lines = |fixture: &ConfirmationFixture| {
        fixture
            .purchase_orders(OWNER)
            .unwrap()
            .iter()
            .find(|order| order.id == selection.id)
            .unwrap()
            .items
            .iter()
            .map(|line| (line.line.clone(), line.receipts().to_vec()))
            .collect::<Vec<_>>()
    };
    let movement_register = |fixture: &ConfirmationFixture| {
        [10, 11, 12, 13, 14].map(|day| {
            fixture
                .open_daily_movement_register(OWNER, feb(day))
                .unwrap()
                .into_iter()
                .map(|row| {
                    (
                        row.item.map(str::to_owned),
                        row.product,
                        row.opening,
                        row.received,
                        row.dispatched,
                        row.adjustment,
                        row.closing,
                    )
                })
                .collect::<Vec<_>>()
        })
    };
    let lines_before = posted_lines(fixture);
    let register_before = movement_register(fixture);
    let lots_before = fifo(fixture);
    let stock_before = stock(fixture);
    let ledger_before = (
        fixture.ledger(),
        fixture.received_ledger(),
        fixture.posted_count(),
    );

    assert_eq!(
        fixture.close_remaining(MANAGER, selection.id, true),
        Err(PurchaseError::AccessDenied)
    );
    fixture.close_remaining(OWNER, selection.id, false).unwrap();
    assert_eq!(fixture.view_purchase_order(selection, &[]), Ok(partial));
    fixture.close_remaining(OWNER, selection.id, true).unwrap();

    let closed = fixture.view_purchase_order(selection, &[]).unwrap();
    assert_eq!(closed.status, PurchaseOrderStatus::Completed);
    assert!(!closed.status.can_receive());
    assert_eq!(posted_lines(fixture), lines_before);
    assert_eq!(movement_register(fixture), register_before);
    assert_eq!(stock(fixture), stock_before);
    assert_eq!(fifo(fixture), lots_before);
    assert_eq!(
        (
            fixture.ledger(),
            fixture.received_ledger(),
            fixture.posted_count(),
        ),
        ledger_before
    );
    let progress = closed
        .lines
        .iter()
        .map(|line| (line.received, line.cancelled, line.remaining));
    println!(
        "{} CLOSED (received, cancelled, remaining)={:?}; stock={stock_before:?}, FIFO={}, no new receipts or movements",
        closed.number,
        progress.collect::<Vec<_>>(),
        fifo_value(&lots_before),
    );
    closed
}

#[test]
fn receive_partials_closes_only_the_unreceived_quantities() {
    // Given the saved plan on a zero-opening fixture.
    let mut fixture = ConfirmationFixture::new(A, 0, SELL).unwrap();
    fixture.add_product(B, 0, SELL).unwrap();
    let selection = save_planned_order(&mut fixture);

    // When the Milestone B purchasing replay finishes.
    let closed = receive_partials(&mut fixture, selection);

    // Then the returned view is closed, without treating cancellations as receipt.
    assert_eq!(closed.status, PurchaseOrderStatus::Completed);
    assert!(
        closed
            .lines
            .iter()
            .map(|line| (line.ordered, line.received, line.cancelled, line.remaining))
            .eq([(12, 8, 4, 0), (9, 5, 4, 0)])
    );
}
