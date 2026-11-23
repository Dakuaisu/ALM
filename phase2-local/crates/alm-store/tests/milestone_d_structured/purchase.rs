use std::fs;
use std::path::Path;
use std::time::Duration;

use alm_core::Role;
use alm_core::confirmation::{
    PopupDocument, PrintIntent, PurchaseOrderStatus, PurchaseOrderView, PurchaseProgressLine,
};
use alm_core::synthetic_read_model::{
    ArchivedPurchaseOrder, ArchivedPurchaseReceipt, ArchivedPurchaseStatus, Error, PurchaseCapture,
};
use alm_store::synthetic_read_model::Store;

use super::fixture::partial_po::partial_purchase_order_setup;
use super::fixture::{A, B, ITEM_A, ITEM_B, feb, money, qty};

const NUMBER: u64 = 1;
const SUPPLIER: &str = "Synthetic Winning Supplier";
const GOLDEN: &str = include_str!("../milestone_b_replay/l07_03/expected_supplier.html");
const PRINT_HEAD: &str = concat!(
    "<html><head><title>Order-PO-001-Synthetic Winning Supplier</title>",
    "<style>@page{margin:0}body{margin:0;padding:10mm}</style></head>",
    "<body style=\"margin:0\">",
);

pub(super) fn capture() -> ArchivedPurchaseOrder {
    let setup = partial_purchase_order_setup();
    let order = setup
        .fixture
        .purchase_orders(Role::Owner)
        .unwrap()
        .iter()
        .find(|order| order.id == setup.selection.id)
        .expect("the B fixture must contain the selected order");
    let view = setup
        .fixture
        .view_purchase_order(setup.selection, &setup.products)
        .unwrap();
    let archived = ArchivedPurchaseOrder::capture(
        Role::Owner,
        PurchaseCapture {
            order,
            number: setup.selection.number,
            view: &view,
            business: setup.business,
            products: &setup.products,
        },
    )
    .unwrap();
    drop(setup.fixture);
    archived
}

pub(super) fn document(store: &Store, path: &Path, artifact: &str) -> PopupDocument {
    let disk_before = fs::read(path).unwrap();
    let backup_before = store.backup(Role::Owner).unwrap();
    let order = store
        .purchase_order(Role::Owner, NUMBER)
        .unwrap()
        .expect("the selected Milestone B PO must be archived")
        .clone();
    for number in [NUMBER, u64::MAX] {
        assert!(matches!(
            store.purchase_order(Role::Manager, number),
            Err(Error::AccessDenied)
        ));
        assert!(matches!(
            store.view_purchase_order(Role::Manager, number),
            Err(Error::AccessDenied)
        ));
        assert!(matches!(
            store.print_purchase_order(Role::Manager, number),
            Err(Error::AccessDenied)
        ));
    }

    assert_eq!(order.status, ArchivedPurchaseStatus::PartiallyReceived);
    let [a, b] = order.lines.as_slice() else {
        panic!("expected two archived PO lines: {:?}", order.lines);
    };
    let receipt = |day, quantity, rate| ArchivedPurchaseReceipt {
        date: feb(day),
        quantity: qty(quantity),
        rate: money(rate),
    };
    assert_eq!((a.product_id, a.rate), (Some(A), money(110)));
    assert_eq!((b.product_id, b.rate), (Some(B), money(190)));
    assert_eq!(
        a.receipts,
        [receipt(12, 5, 130), receipt(11, 3, 110)],
        "line A receipts in entry order, including the backdated receipt"
    );
    assert_eq!(
        b.receipts,
        [receipt(12, 4, 210), receipt(11, 1, 190)],
        "line B receipts in entry order, including the backdated receipt"
    );

    let view = store
        .view_purchase_order(Role::Owner, NUMBER)
        .unwrap()
        .unwrap();
    assert_eq!(
        view,
        PurchaseOrderView {
            number: "PO-001".into(),
            supplier: SUPPLIER.into(),
            date: feb(10),
            status: PurchaseOrderStatus::PartiallyReceived,
            lines: vec![
                PurchaseProgressLine {
                    item: ITEM_A.into(),
                    pack: Some("11 jars".into()),
                    mrp: Some(money(7_175)),
                    ordered: 12,
                    received: 8,
                    cancelled: 0,
                    remaining: 4,
                },
                PurchaseProgressLine {
                    item: ITEM_B.into(),
                    pack: Some("6 cans".into()),
                    mrp: Some(money(8_950)),
                    ordered: 9,
                    received: 5,
                    cancelled: 0,
                    remaining: 4,
                },
            ],
        }
    );

    let output = store
        .print_purchase_order(Role::Owner, NUMBER)
        .unwrap()
        .unwrap();
    assert_eq!(output.target, "_p");
    assert_eq!(
        output.intent,
        PrintIntent::AfterLoad(Duration::from_millis(350))
    );
    assert_eq!(
        output.html,
        format!(
            "{PRINT_HEAD}{}</body></html>",
            GOLDEN.trim_end_matches('\n')
        ),
        "{artifact}: supplier print must match the L07-03 golden, including original quantities"
    );
    assert_eq!(
        output.html.matches('₹').count(),
        2,
        "only the two supplier MRPs"
    );
    assert_eq!(
        store.purchase_order(Role::Owner, NUMBER).unwrap(),
        Some(&order)
    );
    assert_eq!(store.backup(Role::Owner).unwrap(), backup_before);
    assert_eq!(fs::read(path).unwrap(), disk_before);

    fs::write(
        path.with_file_name(format!("{artifact}.html")),
        &output.html,
    )
    .unwrap();
    fs::write(
        path.with_file_name(format!("{artifact}-readback.txt")),
        format!("Internal progress:\n{view:#?}\n\nArchived order and receipts:\n{order:#?}\n"),
    )
    .unwrap();
    output
}
