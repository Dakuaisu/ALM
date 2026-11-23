use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use alm_store::Role;
use alm_store::synthetic_read_model::{Error, ScopedArchiveInput, Store};

#[path = "milestone_d_structured/assertions.rs"]
mod assertions;
#[path = "milestone_d_structured/bill.rs"]
mod bill;
#[path = "milestone_d_structured/device_recovery.rs"]
mod device_recovery;
#[path = "milestone_d_structured/estimate.rs"]
mod estimate;
#[path = "milestone_d_replay/fixture.rs"]
mod fixture;
#[path = "milestone_d_structured/gst.rs"]
mod gst;
#[path = "common/ordinary_gst_fixture.rs"]
mod ordinary_gst_fixture;
#[path = "milestone_d_structured/purchase.rs"]
mod purchase;
#[path = "milestone_d_structured/rate_list.rs"]
mod rate_list;
#[path = "milestone_d_structured/records.rs"]
mod records;

#[test]
fn structured_exports_survive_fresh_recovery() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = root.join(format!("structured-d-{}-{nonce}", std::process::id()));
    fs::create_dir(&dir).unwrap();
    let source = dir.join("source.json");
    let restored = dir.join("restored.json");
    let fixture = fixture::cancelled_fixture();
    let live_bill = bill::source(&fixture);
    let (records, tax) = records::capture(&fixture);
    drop(fixture);
    let mut expected = records.clone();
    expected.reorder_default = 13;
    let mut store = Store::create_scoped_with_gst_and_purchase_orders(
        &source,
        Role::Owner,
        ScopedArchiveInput {
            records,
            scope: estimate::scope(),
            gst: vec![tax],
            purchase_orders: vec![purchase::capture()],
        },
    )
    .unwrap();
    estimate::save_profile(&mut store);
    assert!(store.set_reorder_default(Role::Manager, 13).is_err());
    store.set_reorder_default(Role::Owner, 13).unwrap();
    let original_po = purchase::document(&store, &source, "purchase-order");
    let original_gst = gst::document(&store, &source);
    fs::write(dir.join("gst.json"), gst::json(&original_gst)).unwrap();
    drop(store);
    let store = Store::open(&source, Role::Owner).unwrap();
    assert_eq!(store.records(Role::Owner).unwrap(), &expected);
    assertions::product_setting(&store);
    assert!(store.backup(Role::Manager).is_err());
    assert!(store.sales_csv(Role::Manager).is_err());
    assert!(store.workbook(Role::Manager).is_err());
    let original_html = estimate::document(&store, &source);
    fs::write(dir.join("estimate.html"), &original_html).unwrap();
    let original_bill = bill::document(&store);
    assert_eq!(live_bill, original_bill);
    fs::write(dir.join("bill.svg"), &original_bill).unwrap();
    let original_rates = rate_list::document(&store, &source);
    original_rates.retain(&dir, "");
    let json = store.backup(Role::Owner).unwrap();
    let raw: serde_json::Value = serde_json::from_slice(&json).unwrap();
    assertions::json(&raw);
    let original_csv = store.sales_csv(Role::Owner).unwrap();
    let original_xml = store.workbook(Role::Owner).unwrap();
    fs::write(dir.join("sales.csv"), &original_csv).unwrap();
    fs::write(dir.join("workbook.xls"), &original_xml).unwrap();
    let backup = dir.join("backup.json");
    fs::write(&backup, &json).unwrap();
    assert!(Store::restore(&restored, Role::Manager, &json).is_err());
    assert!(!restored.exists());
    assert!(Store::restore(&restored, Role::Owner, b"not JSON").is_err());
    assert!(!restored.exists());
    let mut wrong_version = raw.clone();
    wrong_version["version"] = 2.into();
    assert!(
        Store::restore(
            &restored,
            Role::Owner,
            &serde_json::to_vec(&wrong_version).unwrap()
        )
        .is_err()
    );
    assert!(!restored.exists());
    let invalid_date = String::from_utf8(json.clone())
        .unwrap()
        .replace("2025-02-21", "2025-02-31");
    assert!(Store::restore(&restored, Role::Owner, invalid_date.as_bytes()).is_err());
    assert!(!restored.exists());
    drop(store);
    let saved_backup = fs::read(backup).unwrap();
    Store::restore(&restored, Role::Owner, &saved_backup).unwrap();
    let reopened = Store::open(&restored, Role::Owner).unwrap();
    assert_eq!(reopened.backup(Role::Owner).unwrap(), saved_backup);
    let regenerated_po = purchase::document(&reopened, &restored, "regenerated-purchase-order");
    assert_eq!(original_po, regenerated_po);
    let regenerated_html = estimate::document(&reopened, &restored);
    let regenerated_bill = bill::document(&reopened);
    assert_eq!(original_bill, regenerated_bill);
    fs::write(dir.join("regenerated-bill.svg"), &regenerated_bill).unwrap();
    let regenerated_rates = rate_list::document(&reopened, &restored);
    assert_eq!(original_rates, regenerated_rates);
    regenerated_rates.retain(&dir, "regenerated-");
    let regenerated_gst = gst::document(&reopened, &restored);
    assert_eq!(original_gst, regenerated_gst);
    fs::write(
        dir.join("regenerated-gst.json"),
        gst::json(&regenerated_gst),
    )
    .unwrap();
    gst::controls(&reopened, &restored, &dir);
    assert_eq!(original_html, regenerated_html);
    fs::write(dir.join("regenerated-estimate.html"), &regenerated_html).unwrap();
    estimate::controls(&reopened, &dir);
    assert_eq!(reopened.records(Role::Owner).unwrap(), &expected);
    assert!(Store::restore(&restored, Role::Owner, &json).is_err());
    assert_eq!(
        reopened.records(Role::Owner).unwrap().receipts[0].amount_minor,
        5000
    );
    assert_eq!(
        reopened.records(Role::Owner).unwrap().sales[0].lines.len(),
        2
    );
    let csv = reopened.sales_csv(Role::Owner).unwrap();
    assert_eq!(csv, original_csv);
    assert!(matches!(
        reopened.sales_csv(Role::Manager),
        Err(Error::AccessDenied)
    ));
    fs::write(dir.join("regenerated-sales.csv"), &csv).unwrap();
    let xml = reopened.workbook(Role::Owner).unwrap();
    assert_eq!(xml, original_xml);
    assert!(matches!(
        reopened.workbook(Role::Manager),
        Err(Error::AccessDenied)
    ));
    fs::write(dir.join("regenerated-workbook.xls"), &xml).unwrap();
    assertions::product_setting(&reopened);
    assertions::projections(&csv, &xml);
    device_recovery::run(&Store::open(&source, Role::Owner).unwrap(), &reopened, &dir);
    if std::env::var_os("ALM_STRUCTURED_RETAIN").is_some() {
        println!("Structured replay evidence: {}", dir.display());
    } else {
        fs::remove_dir_all(dir).unwrap();
    }
}
