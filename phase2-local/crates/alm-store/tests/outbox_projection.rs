//! Outbox and projection storage: written before sending, unknown after a restart, scoped per
//! account and role, applied with checkpoint progress, and cost-free on a Manager device.
use alm_core::confirmation::manual_cashbook::CashBookAmounts;
use alm_core::projection::{
    OwnerValues, ProjectedCashDay, ProjectedCrate, ProjectedCustomer, ProjectedGodownRow,
    ProjectedGstBill, ProjectedProduct, ProjectedProductTerms, ProjectedPurchaseLine,
    ProjectedPurchaseOrder, ProjectedSale, ProjectedStock, ProjectedSupplier, ProjectionBatch,
    ProjectionSnapshot,
};
use alm_core::{BillDate, Money, Role};
use alm_store::{Outbox, OutboxError, OutboxStatus, ProjectionError, ProjectionStore};

mod scratch_retry {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../alm-store/tests/common/scratch_retry.rs"
    ));
}

struct Root(std::path::PathBuf);

impl Root {
    fn new(label: &str) -> Self {
        let path =
            scratch_retry::create(&std::env::temp_dir(), &format!("alm-store-{label}-")).unwrap();
        Self(path)
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn money(paise: i64) -> Money {
    Money::from_minor_units(paise)
}

fn owner_snapshot() -> ProjectionSnapshot {
    let day = BillDate::new(2025, 4, 30).unwrap();
    ProjectionSnapshot {
        sales_unsupported: None,
        sales: vec![ProjectedSale {
            draft_id: alm_core::DraftId::new(1),
            invoice: 1,
            date: day,
            customer: "Synthetic Customer".into(),
            items: vec!["Synthetic Lime".into(), "Synthetic Soda".into()],
            lines: 2,
            packets: 3,
            total: money(30_350),
            paid: money(2_000),
            online: false,
            cancelled: false,
            balance: money(28_350),
        }],
        receipts: Vec::new(),
        customers: vec![ProjectedCustomer {
            id: "batch1-customer".into(),
            name: "Synthetic Customer".into(),
            balance: money(28_350),
            mobile: "90000 00001".into(),
            addr1: "12 Synthetic Lane".into(),
            addr2: "Kanpur".into(),
        }],
        customer_names: Vec::new(),
        stock: vec![ProjectedStock {
            product: 1,
            item: "Synthetic Lime".into(),
            brand: String::new(),
            on_hand: 48,
            reorder_level: 5,
        }],
        catalog: vec![ProjectedProduct {
            id: 1,
            name: "Synthetic Lime".into(),
            rate: money(12_050),
        }],
        cash_days: vec![ProjectedCashDay {
            id: 1,
            date: day,
            amounts: CashBookAmounts {
                cash: money(2_000),
                ..CashBookAmounts::ZERO
            },
        }],
        products: vec![ProjectedProductTerms {
            id: 1,
            item: "Synthetic Lime".into(),
            brand: "Synthetic".into(),
            sell: money(12_050),
            reorder_level: 5,
        }],
        suppliers: vec![ProjectedSupplier {
            id: 0,
            name: "Synthetic Wholesale".into(),
        }],
        crates: vec![ProjectedCrate {
            id: 1,
            name: "Party crate".into(),
            price: money(50_000),
            components: 2,
        }],
        godown: vec![ProjectedGodownRow {
            item: "Synthetic Soda".into(),
            cases: 4,
            note: "back room".into(),
        }],
        gst_bills: vec![ProjectedGstBill {
            id: 1,
            number: 1,
            date: day,
            customer: "Synthetic GST Buyer".into(),
            cancelled: false,
        }],
        owner: Some(OwnerValues {
            sale_costs: vec![(
                alm_core::DraftId::new(1),
                money(30_350),
                Some(money(17_777)),
            )],
            stock_value: Some(money(333_333)),
            buy_rates: vec![(1, money(7_000))],
            supplier_prices: vec![(1, 0, money(6_900))],
            purchase_orders: vec![ProjectedPurchaseOrder {
                id: 0,
                supplier: "Synthetic Wholesale".into(),
                date: day,
                status: "PartiallyReceived".into(),
                lines: vec![ProjectedPurchaseLine {
                    item: "Synthetic Lime".into(),
                    product: Some(1),
                    ordered: 10,
                    received: 4,
                    remaining: 6,
                    rate: money(6_900),
                }],
            }],
            book: alm_core::projection::book::OwnerBook {
                settings: alm_core::projection::book::BookSettings {
                    reorder_default: 7,
                    ..Default::default()
                },
                gst_deleted: vec![3],
                ..Default::default()
            },
        }),
        manager: None,
    }
}

fn batch(role: Role, checkpoint: u64, snapshot: Option<ProjectionSnapshot>) -> ProjectionBatch {
    ProjectionBatch {
        role,
        checkpoint,
        as_of_ms: 1_700_000_000_000 + checkpoint,
        snapshot,
        rows: None,
    }
}

#[test]
fn outbox_stores_before_sending_and_marks_in_flight_intents_unknown_after_restart() {
    let root = Root::new("outbox");
    let mut outbox = Outbox::open_scoped(&root.0, "trial").unwrap();
    let id = outbox.next_request_id("trial").unwrap();
    assert_eq!(id, "trial-1");
    let entry = outbox.record(&id, "cancel draft=1").unwrap();
    assert_eq!(entry.status, OutboxStatus::Pending);
    assert_eq!(outbox.record(&id, "cancel draft=1").unwrap(), entry);
    assert!(matches!(
        outbox.record(&id, "cancel draft=2"),
        Err(OutboxError::Conflict)
    ));
    let done = outbox.next_request_id("trial").unwrap();
    outbox.record(&done, "restore draft=1").unwrap();
    outbox
        .mark(&done, OutboxStatus::Confirmed, Some("done"))
        .unwrap();
    drop(outbox);

    let outbox = Outbox::open_scoped(&root.0, "trial").unwrap();
    let entries = outbox.entries().unwrap();
    assert_eq!(
        entries[0].status,
        OutboxStatus::Unknown,
        "in flight at restart"
    );
    assert_eq!(entries[1].status, OutboxStatus::Confirmed);
    assert_eq!(entries[1].outcome.as_deref(), Some("done"));
    assert_eq!(outbox.unresolved().unwrap().len(), 1);
    assert_eq!(outbox.next_request_id("trial").unwrap(), "trial-3");
    let other = Outbox::open_scoped(&root.0, "other").unwrap();
    assert!(
        other.entries().unwrap().is_empty(),
        "accounts never share an outbox"
    );
}

#[test]
fn projection_applies_with_checkpoint_survives_restart_and_rebuilds() {
    let root = Root::new("projection");
    let mut store = ProjectionStore::open_scoped(&root.0, "trial", Role::Owner).unwrap();
    assert_eq!(store.load().unwrap(), None);
    store
        .apply(&batch(Role::Owner, 3, Some(owner_snapshot())))
        .unwrap();
    drop(store);
    let mut store = ProjectionStore::open_scoped(&root.0, "trial", Role::Owner).unwrap();
    let stored = store.load().unwrap().unwrap();
    assert_eq!((stored.checkpoint, stored.as_of_ms), (3, 1_700_000_000_003));
    assert_eq!(stored.snapshot, owner_snapshot());
    store.apply(&batch(Role::Owner, 4, None)).unwrap();
    let stored = store.load().unwrap().unwrap();
    assert_eq!(stored.checkpoint, 4);
    assert_eq!(
        stored.snapshot,
        owner_snapshot(),
        "no change keeps the rows"
    );
    assert!(matches!(
        store.apply(&batch(Role::Owner, 2, Some(ProjectionSnapshot::default()))),
        Err(ProjectionError::Rejected)
    ));
    assert!(matches!(
        store.apply(&batch(Role::Manager, 9, None)),
        Err(ProjectionError::Rejected)
    ));
    assert_eq!(store.load().unwrap().unwrap().checkpoint, 4);
    store.clear().unwrap();
    assert_eq!(store.load().unwrap(), None);
    store
        .apply(&batch(Role::Owner, 1, Some(owner_snapshot())))
        .unwrap();
    assert_eq!(store.load().unwrap().unwrap().snapshot, owner_snapshot());
    assert!(
        ProjectionStore::open_scoped(&root.0, "other", Role::Owner)
            .unwrap()
            .load()
            .unwrap()
            .is_none()
    );
}

#[test]
fn a_manager_projection_file_holds_no_costs_margins_or_fifo_values() {
    let root = Root::new("manager-projection");
    let mut store = ProjectionStore::open_scoped(&root.0, "trial", Role::Manager).unwrap();
    assert!(matches!(
        store.apply(&batch(Role::Manager, 1, Some(owner_snapshot()))),
        Err(ProjectionError::Rejected)
    ));
    let manager = ProjectionSnapshot {
        catalog: owner_snapshot().catalog,
        ..ProjectionSnapshot::default()
    };
    store
        .apply(&batch(Role::Manager, 1, Some(manager.clone())))
        .unwrap();
    assert_eq!(store.load().unwrap().unwrap().snapshot, manager);
    drop(store);
    let dir = root.0.canonicalize().unwrap().join("projections");
    let files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    let mut bytes = Vec::new();
    for file in &files {
        bytes.extend(std::fs::read(file).unwrap());
    }
    let text = String::from_utf8_lossy(&bytes).to_lowercase();
    for word in [
        "cost", "margin", "fifo", "profit", "owner_", "buy", "17777", "333333",
    ] {
        assert!(
            !text.contains(word),
            "{word} in the Manager projection file"
        );
    }
    assert!(text.contains("synthetic lime"));
}

/// A file written before the book rows and customer contacts (its Owner sections are the
/// simulation's old state) opens, gains the new tables and is rebuilt on the next pull.
#[test]
fn an_older_projection_file_is_upgraded_and_rebuilt() {
    let root = Root::new("older-file");
    let mut store = ProjectionStore::open_scoped(&root.0, "trial", Role::Owner).unwrap();
    store
        .apply(&batch(Role::Owner, 3, Some(owner_snapshot())))
        .unwrap();
    drop(store);
    let file = std::fs::read_dir(root.0.join("projections"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.to_string_lossy().ends_with("-owner.sqlite3"))
        .unwrap();
    let conn = rusqlite::Connection::open(&file).unwrap();
    conn.execute_batch(
        "DROP TABLE projected_customer_contacts;
         DELETE FROM owner_state_sections;
         INSERT INTO owner_state_sections VALUES ('catalog', '{\"products\": {}}');",
    )
    .unwrap();
    drop(conn);
    let mut store = ProjectionStore::open_scoped(&root.0, "trial", Role::Owner).unwrap();
    assert_eq!(store.load().unwrap(), None, "rebuilt from the authority");
    store
        .apply(&batch(Role::Owner, 4, Some(owner_snapshot())))
        .unwrap();
    assert_eq!(store.load().unwrap().unwrap().snapshot, owner_snapshot());
}

/// A file from before drafts carried their device (draft IDs in INTEGER columns) is recreated
/// with text draft IDs and rebuilt on the next pull; a device's draft ID then round-trips.
#[test]
fn a_projection_file_with_numeric_draft_ids_is_rebuilt_with_device_draft_ids() {
    let root = Root::new("numeric-drafts");
    drop(ProjectionStore::open_scoped(&root.0, "trial", Role::Owner).unwrap());
    let file = std::fs::read_dir(root.0.join("projections"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.to_string_lossy().ends_with("-owner.sqlite3"))
        .unwrap();
    let conn = rusqlite::Connection::open(&file).unwrap();
    conn.execute_batch(
        "DROP TABLE projected_sales; DROP TABLE owner_sale_costs; DROP TABLE projected_notes;
         CREATE TABLE projected_notes (id INTEGER PRIMARY KEY CHECK (id = 1),
             sales_unsupported INTEGER) STRICT;
         CREATE TABLE projected_sales (position INTEGER PRIMARY KEY, draft_id INTEGER NOT NULL,
             invoice INTEGER NOT NULL, date TEXT NOT NULL, customer TEXT NOT NULL,
             items TEXT NOT NULL, lines INTEGER NOT NULL, packets INTEGER NOT NULL,
             total INTEGER NOT NULL, paid INTEGER NOT NULL, online INTEGER NOT NULL,
             cancelled INTEGER NOT NULL, balance INTEGER NOT NULL) STRICT;
         CREATE TABLE owner_sale_costs (draft_id INTEGER PRIMARY KEY, revenue INTEGER NOT NULL,
             cost INTEGER) STRICT;
         INSERT INTO projection_checkpoint VALUES (1, 5, 0);",
    )
    .unwrap();
    drop(conn);
    let mut store = ProjectionStore::open_scoped(&root.0, "trial", Role::Owner).unwrap();
    assert_eq!(store.load().unwrap(), None, "rebuilt from the authority");
    let device = alm_core::DraftId::scoped("trial:dev-00000000000000aa", 1).unwrap();
    let mut snapshot = owner_snapshot();
    snapshot.sales[0].draft_id = device;
    snapshot.owner.as_mut().unwrap().sale_costs[0].0 = device;
    store
        .apply(&batch(Role::Owner, 6, Some(snapshot.clone())))
        .unwrap();
    assert_eq!(store.load().unwrap().unwrap().snapshot, snapshot);
}
