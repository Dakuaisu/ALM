use std::fs;

use alm_phase2_local::Role;
use alm_phase2_local::confirmation::{GstBillId, GstBillInvoice, GstBillNumber};
use alm_phase2_local::synthetic_read_model::{Error, Records, Store};

use super::{artifacts::Sandbox, fixture};

#[test]
fn restore_rejects_duplicate_standalone_gst_ids_without_changing_store() {
    // Given: a valid backup with separate internal IDs and invoice numbers.
    let sandbox = Sandbox::new();
    let first_path = sandbox.path().join("first.json");
    let first = fixture::reopened(1);
    let mut second_record = first.record.clone();
    second_record.id = GstBillId(92);
    second_record.number = GstBillNumber(48);
    let expected = [first.record.clone(), second_record];
    let records = Records {
        reorder_default: 0,
        products: Vec::new(),
        customers: Vec::new(),
        sales: Vec::new(),
        receipts: Vec::new(),
    };
    let source =
        Store::create_with_standalone_gst(&first_path, Role::Owner, records, expected.to_vec())
            .unwrap();
    let original = source.backup(Role::Owner).unwrap();
    let mut archive: serde_json::Value = serde_json::from_slice(&original).unwrap();
    assert_eq!(archive["standalone_gst"][0]["id"], 91);
    assert_eq!(archive["standalone_gst"][1]["id"], 92);
    assert_eq!(archive["standalone_gst"][0]["number"], 47);
    assert_eq!(archive["standalone_gst"][1]["number"], 48);

    // When: only the second saved bill's internal ID is changed to the first ID.
    archive["standalone_gst"][1]["id"] = serde_json::json!(91);
    let duplicate = serde_json::to_vec(&archive).unwrap();
    let fresh_path = sandbox.path().join("fresh.json");
    let fresh_result = Store::restore(&fresh_path, Role::Owner, &duplicate);

    // Then: fresh restore rejects the archive and leaves no destination behind.
    assert!(matches!(fresh_result, Err(Error::InvalidArchive)));
    assert!(!fresh_path.try_exists().unwrap());

    // When: the same archive is restored over an occupied valid store.
    let occupied_path = sandbox.path().join("occupied.json");
    drop(Store::restore(&occupied_path, Role::Owner, &original).unwrap());
    let occupied_bytes = fs::read(&occupied_path).unwrap();
    let occupied_result = Store::restore(&occupied_path, Role::Owner, &duplicate);

    // Then: occupied restore rejects without changing bytes; reopening emits both originals.
    assert!(matches!(occupied_result, Err(Error::InvalidArchive)));
    assert_eq!(fs::read(&occupied_path).unwrap(), occupied_bytes);
    let reopened = Store::open(&occupied_path, Role::Owner).unwrap();
    for bill in expected {
        let mut emitted: Option<GstBillInvoice> = None;
        reopened
            .emit_standalone_gst(Role::Owner, bill.id, &mut |invoice| {
                emitted = Some(invoice.clone());
            })
            .unwrap();
        let emitted = emitted.unwrap();
        assert_eq!(emitted.record, bill);
        assert_eq!(emitted.totals, first.totals);
    }
    println!(
        "Duplicate internal GST ID 91 (invoice numbers 47 and 48): InvalidArchive at both paths; fresh path absent, occupied bytes and original invoices unchanged"
    );
}
