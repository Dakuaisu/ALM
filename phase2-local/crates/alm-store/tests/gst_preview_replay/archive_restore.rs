use std::fs;

use alm_core::Role;
use alm_core::confirmation::{GstBillId, GstBillInvoice, GstBillNumber};
use alm_core::synthetic_read_model::{BusinessProfile, Error, Records};
use alm_store::synthetic_read_model::Store;

use super::{artifacts::Sandbox, fixture};

#[test]
fn owner_profile_rewrite_preserves_standalone_gst_bills_through_backup_and_restore() {
    // Given: standalone GST bills with distinct internal IDs, invoice numbers, and line counts.
    let sandbox = Sandbox::new();
    let source_path = sandbox.path().join("source.json");
    let first = fixture::reopened(1);
    let mut second = fixture::reopened(2);
    second.record.id = GstBillId(92);
    second.record.number = GstBillNumber(48);
    let records = Records {
        reorder_default: 0,
        products: Vec::new(),
        customers: Vec::new(),
        sales: Vec::new(),
        receipts: Vec::new(),
    };
    let mut source = Store::create_with_standalone_gst(
        &source_path,
        Role::Owner,
        records,
        vec![first.record.clone(), second.record.clone()],
    )
    .unwrap();
    let mut expected_invoices = Vec::new();
    for id in [GstBillId(91), GstBillId(92)] {
        source
            .emit_standalone_gst(Role::Owner, id, &mut |invoice| {
                expected_invoices.push(invoice.clone());
            })
            .unwrap();
    }
    assert_eq!(expected_invoices, [first, second]);

    // When: the Owner profile is rewritten, then the source is reopened before backup.
    let expected_profile = BusinessProfile {
        fssai: "FSSAI-REWRITE-42".into(),
        gstin: "27ABCDE1234F1Z5".into(),
        show_fssai: Some(true),
        show_gstin: Some(true),
        ..BusinessProfile::default()
    };
    source
        .save_business_profile(
            Role::Owner,
            &expected_profile,
            1_800_000_123,
            "rewrite-device",
        )
        .unwrap();
    drop(source);
    let reopened_source = Store::open(&source_path, Role::Owner).unwrap();
    let expected_profile = BusinessProfile {
        version: 1,
        modified_at: 1_800_000_123,
        device: "rewrite-device".into(),
        ..expected_profile
    };
    assert_eq!(
        reopened_source.business_profile(Role::Owner).unwrap(),
        &expected_profile
    );
    let backup = reopened_source.backup(Role::Owner).unwrap();
    drop(reopened_source);

    // Then: a fresh restore and disk reopen preserve the profile and both full invoices by ID.
    let restored_path = sandbox.path().join("restored.json");
    drop(Store::restore(&restored_path, Role::Owner, &backup).unwrap());
    let restored = Store::open(&restored_path, Role::Owner).unwrap();
    assert_eq!(
        restored.business_profile(Role::Owner).unwrap(),
        &expected_profile
    );
    for expected in expected_invoices {
        let mut emitted: Option<GstBillInvoice> = None;
        restored
            .emit_standalone_gst(Role::Owner, expected.record.id, &mut |invoice| {
                emitted = Some(invoice.clone());
            })
            .unwrap();
        assert_eq!(emitted.unwrap(), expected);
    }
}

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
