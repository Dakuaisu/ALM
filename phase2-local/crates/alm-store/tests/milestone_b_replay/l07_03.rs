#[path = "l07_03/artifact.rs"]
mod artifact;
#[path = "l07_03/branches.rs"]
mod branches;
#[path = "l07_03/capture.rs"]
mod capture;

use alm_store::confirmation::{ConfirmationFixture, PurchaseOrderStatus, PurchaseProductDisplay};

use super::purchasing::post_partials;
use super::{
    A, B, ITEM_A, ITEM_B, MANAGER, OWNER, SELL, fifo, lot, money, save_planned_order, stock,
};
use alm_store::ProductId;
use capture::capture_outputs;

const PRODUCTS: &[PurchaseProductDisplay<'_>] = &[
    PurchaseProductDisplay {
        id: ProductId::new(99),
        item: ITEM_A,
        pack_size: "11",
        uom: "jars",
        mrp: Some(money(7_175)),
    },
    PurchaseProductDisplay {
        id: A,
        item: "Renamed Synthetic A",
        pack_size: "12",
        uom: "bottles",
        mrp: Some(money(6_750)),
    },
    PurchaseProductDisplay {
        id: ProductId::new(98),
        item: ITEM_B,
        pack_size: "6",
        uom: "cans",
        mrp: Some(money(8_950)),
    },
];

#[test]
fn l07_03_supplier_outputs_preserve_milestone_b_progress_and_inventory() {
    // Given: the planned two-product order and a note entered while still pending.
    let mut fixture = ConfirmationFixture::new(A, 0, SELL).unwrap();
    fixture.add_product(B, 0, SELL).unwrap();
    let selection = save_planned_order(&mut fixture);
    let mut note = fixture
        .open_purchase_order_note(OWNER, selection.id)
        .unwrap();
    note.note = "Synthetic note & handling".into();
    fixture.save_purchase_order_note(OWNER, &note).unwrap();
    let pending = fixture.view_purchase_order(selection, PRODUCTS).unwrap();
    assert_eq!(pending.status, PurchaseOrderStatus::Pending);
    assert_eq!(progress_rows(&pending), [(12, 0, 0, 12), (9, 0, 0, 9)]);
    assert_eq!(stock(&fixture), [0, 0]);

    // When: the planned order is partially received, then all outputs are captured.
    post_partials(&mut fixture, selection);
    fixture
        .add_comparison_supplier(OWNER, "Synthetic Third Supplier")
        .unwrap();
    let progress = fixture.view_purchase_order(selection, PRODUCTS).unwrap();
    let orders = fixture.purchase_orders(OWNER).unwrap().to_vec();
    let ledgers = (
        fixture.ledger(),
        fixture.received_ledger(),
        fixture.posted_count(),
    );
    let stock_before = stock(&fixture);
    let fifo_before = fifo(&fixture);
    let output = capture_outputs(&fixture, selection, true);
    let mut traces = branches::exercise(&fixture, selection, &output.html);
    traces.extend(output.artifact_trace);

    // Then: outputs do not change progress, stock, or dated FIFO layers.
    assert_eq!(progress.status, PurchaseOrderStatus::PartiallyReceived);
    assert_eq!(progress_rows(&progress), [(12, 8, 0, 4), (9, 5, 0, 4)]);
    assert_eq!(
        (progress.lines[0].pack.as_deref(), progress.lines[0].mrp),
        (Some("11 jars"), Some(money(7_175)))
    );
    assert_eq!(
        (progress.lines[1].pack.as_deref(), progress.lines[1].mrp),
        (Some("6 cans"), Some(money(8_950)))
    );
    let stock_after = stock(&fixture);
    let lots = fifo(&fixture);
    assert_eq!(stock_before, [8, 5]);
    assert_eq!(stock_after, stock_before);
    assert_eq!(lots, fifo_before);
    assert_eq!(lots[0].rows, [lot(11, 3, 110), lot(12, 5, 130)]);
    assert_eq!(lots[1].rows, [lot(11, 1, 190), lot(12, 4, 210)]);
    let after_outputs = fixture.view_purchase_order(selection, PRODUCTS).unwrap();
    assert_eq!(after_outputs, progress);
    assert_eq!(fixture.purchase_orders(OWNER).unwrap(), orders);
    assert_eq!(
        (
            fixture.ledger(),
            fixture.received_ledger(),
            fixture.posted_count()
        ),
        ledgers
    );
    assert_eq!(
        fixture.close_remaining(MANAGER, selection.id, true),
        Err(alm_store::confirmation::PurchaseError::AccessDenied)
    );
    fixture.close_remaining(OWNER, selection.id, true).unwrap();
    let closed = fixture.view_purchase_order(selection, PRODUCTS).unwrap();
    assert_eq!(closed.status, PurchaseOrderStatus::Completed);
    assert_eq!(progress_rows(&closed), [(12, 8, 4, 0), (9, 5, 4, 0)]);
    assert_eq!(stock(&fixture), [8, 5]);
    assert_eq!(fifo(&fixture), lots);
    assert_eq!(
        capture_outputs(&fixture, selection, false).html,
        output.html
    );
    let after_closed_outputs = fixture.view_purchase_order(selection, PRODUCTS).unwrap();
    assert_eq!(after_closed_outputs, closed);
    assert_eq!(
        (stock(&fixture), fifo(&fixture)),
        (stock_before, fifo_before.clone())
    );

    let summary = format!(
        "Pending view: {pending:?}\nBefore outputs: {progress:?}\nAfter outputs: {after_outputs:?}\nAfter close: {closed:?}\nAfter closed output: {after_closed_outputs:?}\nStock before outputs: {stock_before:?}\nStock after outputs: {stock_after:?}\nStock after close and output: {:?}\nFIFO before outputs: {fifo_before:?}\nFIFO after outputs: {lots:?}\nFIFO after close and output: {:?}\nLedgers before and after outputs: {ledgers:?}\nBranches:\n{}\n",
        stock(&fixture),
        fifo(&fixture),
        traces.join("\n")
    );
    println!("{summary}");
    if let Some(dir) = output.artifact_dir {
        std::fs::write(dir.join("replay-trace.txt"), summary).unwrap();
        println!("Retained synthetic PO artifacts: {}", dir.display());
    }
}

fn progress_rows(view: &alm_store::confirmation::PurchaseOrderView) -> Vec<(u32, u32, u32, u32)> {
    view.lines
        .iter()
        .map(|line| (line.ordered, line.received, line.cancelled, line.remaining))
        .collect()
}
