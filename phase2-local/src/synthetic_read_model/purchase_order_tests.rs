use super::*;
use crate::Role;
use crate::confirmation::{
    ConfirmationFixture, PurchaseBusiness, PurchaseDraft, PurchaseLine, PurchaseOrderSelection,
    PurchaseOutputRequest, PurchaseProductDisplay, PurchaseSupplier, ReceiveLine, ReceiveRequest,
};
use crate::{BillDate, Money, ProductId};
use std::num::NonZeroU32;

#[test]
fn purchase_order_access_denies_manager_before_lookup() {
    let records = Records {
        reorder_default: 0,
        products: vec![],
        customers: vec![],
        sales: vec![],
        receipts: vec![],
    };
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(format!("alm-po-test-{}", std::process::id()));
    let store = Store::create(&path, Role::Owner, records).unwrap();
    assert!(matches!(
        store.purchase_order(Role::Manager, 99),
        Err(Error::AccessDenied)
    ));
    assert!(matches!(
        store.print_purchase_order(Role::Manager, 99),
        Err(Error::AccessDenied)
    ));
    std::fs::remove_file(path).unwrap();
}

#[test]
fn captured_purchase_order_survives_rewrite_and_reopens_without_fixture() {
    let date = BillDate::new(2024, 2, 9).unwrap();
    let receipt_date = BillDate::new(2024, 2, 10).unwrap();
    let mut fixture =
        ConfirmationFixture::new(ProductId::new(1), 0, Money::from_minor_units(100)).unwrap();
    let draft = PurchaseDraft {
        supplier: Some(PurchaseSupplier {
            id: None,
            name: "Supplier & Co".into(),
        }),
        date: Some(date),
        note: "Synthetic note".into(),
        items: vec![PurchaseLine {
            item: "Original".into(),
            product_id: Some(ProductId::new(1)),
            ordered: NonZeroU32::new(5).unwrap(),
            rate: Money::from_minor_units(123),
        }],
    };
    let id = fixture.save_purchase_order(Role::Owner, &draft).unwrap();
    fixture
        .receive_purchase_order(ReceiveRequest {
            role: Role::Owner,
            id,
            date: Some(receipt_date),
            today: receipt_date,
            lines: &[ReceiveLine {
                quantity: 2,
                rate: Money::from_minor_units(145),
            }],
        })
        .unwrap();
    let selection = PurchaseOrderSelection {
        role: Role::Owner,
        id,
        number: 7,
    };
    let products = [
        PurchaseProductDisplay {
            id: ProductId::new(2),
            item: "Original",
            pack_size: "name pack",
            uom: "pcs",
            mrp: Some(Money::from_minor_units(400)),
        },
        PurchaseProductDisplay {
            id: ProductId::new(1),
            item: "Renamed",
            pack_size: "id pack",
            uom: "box",
            mrp: Some(Money::from_minor_units(900)),
        },
    ];
    let business = PurchaseBusiness {
        name: "Business",
        address: "Address",
        mobile: "000",
    };
    let view = fixture.view_purchase_order(selection, &products).unwrap();
    let mut printed = Vec::new();
    fixture
        .print_purchase_order(
            PurchaseOutputRequest {
                selection,
                business,
                products: &products,
            },
            &mut |doc| printed.push(doc),
        )
        .unwrap();
    let snapshot = ArchivedPurchaseOrder::capture(
        Role::Owner,
        PurchaseCapture {
            order: &fixture.purchase_orders(Role::Owner).unwrap()[0],
            number: 7,
            view: &view,
            business,
            products: &products,
        },
    )
    .unwrap();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(format!("alm-po-reopen-{}", std::process::id()));
    let records = Records {
        reorder_default: 0,
        products: vec![],
        customers: vec![],
        sales: vec![],
        receipts: vec![],
    };
    let scope = AccountScope {
        sales: vec![],
        receipts: vec![],
        zero_openings: vec![],
    };
    let mut store = Store::create_scoped_with_gst_and_purchase_orders(
        &path,
        Role::Owner,
        ScopedArchiveInput {
            records,
            scope,
            gst: vec![],
            purchase_orders: vec![snapshot.clone()],
        },
    )
    .unwrap();
    store.set_reorder_default(Role::Owner, 3).unwrap();
    drop(store);
    drop(fixture);
    let reopened = Store::open(&path, Role::Owner).unwrap();
    assert_eq!(
        reopened.purchase_order(Role::Owner, 7).unwrap(),
        Some(&snapshot)
    );
    assert_eq!(
        reopened.view_purchase_order(Role::Owner, 7).unwrap(),
        Some(view)
    );
    assert_eq!(
        reopened
            .print_purchase_order(Role::Owner, 7)
            .unwrap()
            .unwrap(),
        printed[0]
    );
    assert_eq!(snapshot.lines[0].receipts[0].date, receipt_date);
    assert_eq!(
        snapshot.lines[0].receipts[0].rate,
        Money::from_minor_units(145)
    );
    assert_eq!(
        snapshot.lines[0].supplier.pack.as_deref(),
        Some("id pack box")
    );
    assert_eq!(
        snapshot.lines[0].internal.pack.as_deref(),
        Some("name pack pcs")
    );
    let mut inconsistent = snapshot.clone();
    inconsistent.lines[0].receipts[0].quantity = NonZeroU32::new(3).unwrap();
    assert!(matches!(
        purchase_order::validate_orders(Some(&[inconsistent])),
        Err(Error::InvalidArchive)
    ));
    let mut overflowing = snapshot.clone();
    overflowing.lines[0].receipts[0].quantity = NonZeroU32::new(u32::MAX).unwrap();
    overflowing.lines[0]
        .receipts
        .push(snapshot.lines[0].receipts[0].clone());
    assert!(matches!(
        purchase_order::validate_orders(Some(&[overflowing])),
        Err(Error::InvalidArchive)
    ));
    let backup = reopened.backup(Role::Owner).unwrap();
    assert!(
        !String::from_utf8(backup.clone())
            .unwrap()
            .contains("<html>")
    );
    let restored_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(format!("alm-po-restore-{}", std::process::id()));
    let restored = Store::restore(&restored_path, Role::Owner, &backup).unwrap();
    assert_eq!(
        restored.purchase_order(Role::Owner, 7).unwrap(),
        Some(&snapshot)
    );
    std::fs::remove_file(restored_path).unwrap();
    std::fs::remove_file(path).unwrap();
}
