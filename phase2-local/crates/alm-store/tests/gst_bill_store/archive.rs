//! Backup archive: new fields round-trip, markers keep deleted bills out, older archives default.
use alm_core::confirmation::{GstBillId, GstBillLedger, GstUnit};
use alm_core::synthetic_read_model::{Error, Records};
use alm_store::synthetic_read_model::Store;

use super::{OWNER, fixture, full_draft, target_scratch};

fn records() -> Records {
    Records {
        reorder_default: 0,
        products: Vec::new(),
        customers: Vec::new(),
        sales: Vec::new(),
        receipts: Vec::new(),
    }
}

fn ledger() -> (GstBillLedger, bool) {
    let mut fixture = fixture();
    let before = fixture.clone();
    for id in 1..=3 {
        fixture
            .save_gst_bill(OWNER, &full_draft(&fixture, id), &mut |_| {})
            .unwrap();
    }
    fixture.cancel_gst_bill(OWNER, GstBillId(2)).unwrap();
    fixture.delete_gst_bill(OWNER, GstBillId(3)).unwrap();
    let unchanged = fixture.without_gst_bills() == before.without_gst_bills();
    (fixture.gst_bill_ledger(OWNER).unwrap(), unchanged)
}

#[test]
fn gst_archive_round_trips_fields_and_markers_and_rejects_marked_bills() {
    let scratch = target_scratch::Scratch::new("gst-archive-");
    let (ledger, ordinary_unchanged) = ledger();
    assert!(ordinary_unchanged);
    let source = Store::create_with_standalone_gst_ledger(
        &scratch.path().join("source.json"),
        OWNER,
        records(),
        ledger.clone(),
    )
    .unwrap();
    let backup = source.backup(OWNER).unwrap();
    let restored = Store::restore(&scratch.path().join("restored.json"), OWNER, &backup).unwrap();
    assert_eq!(restored.standalone_gst_ledger(OWNER).unwrap(), ledger);
    assert_eq!(ledger.deleted, [GstBillId(3)]);

    // A deleted bill cannot come back inside an archive.
    let mut revived = ledger.clone();
    let mut copy = revived.bills[0].clone();
    copy.id = GstBillId(3);
    revived.bills.push(copy);
    let path = scratch.path().join("revived.json");
    assert!(matches!(
        Store::create_with_standalone_gst_ledger(&path, OWNER, records(), revived),
        Err(Error::InvalidArchive)
    ));
    assert!(!path.exists());
    let mut json: serde_json::Value = serde_json::from_slice(&backup).unwrap();
    json["standalone_gst"][0]["id"] = serde_json::json!(3);
    let path = scratch.path().join("revived-restore.json");
    assert!(matches!(
        Store::restore(&path, OWNER, &serde_json::to_vec(&json).unwrap()),
        Err(Error::InvalidArchive)
    ));
    assert!(!path.exists());
}

#[test]
fn older_gst_archives_restore_with_save_defaults() {
    let scratch = target_scratch::Scratch::new("gst-archive-old-");
    let (ledger, ordinary_unchanged) = ledger();
    assert!(ordinary_unchanged);
    let source = Store::create_with_standalone_gst_ledger(
        &scratch.path().join("source.json"),
        OWNER,
        records(),
        ledger,
    )
    .unwrap();
    // Reduce the archive to the first-version bill shape, legacy jurisdiction flag included.
    let mut json: serde_json::Value =
        serde_json::from_slice(&source.backup(OWNER).unwrap()).unwrap();
    json.as_object_mut()
        .unwrap()
        .remove("standalone_gst_deleted");
    for bill in json["standalone_gst"].as_array_mut().unwrap() {
        let bill = bill.as_object_mut().unwrap();
        for key in [
            "customer_id",
            "cust_addr",
            "cust_phone",
            "cust_email",
            "cust_state",
            "cust_state_code",
            "pos_state",
            "pos_code",
            "pay_mode",
            "pay_terms",
            "reverse_charge",
            "transporter",
            "vehicle_no",
            "lr_no",
            "terms_delivery",
            "dispatch",
            "created",
        ] {
            bill.remove(key);
        }
        bill.insert("jurisdiction".into(), serde_json::json!("Inter"));
        for line in bill["lines"].as_array_mut().unwrap() {
            let line = line.as_object_mut().unwrap();
            for key in ["product", "hsn", "unit", "packet_qty"] {
                line.remove(key);
            }
        }
    }
    let restored = Store::restore(
        &scratch.path().join("restored.json"),
        OWNER,
        &serde_json::to_vec(&json).unwrap(),
    )
    .unwrap();
    let ledger = restored.standalone_gst_ledger(OWNER).unwrap();
    assert!(ledger.deleted.is_empty());
    for bill in &ledger.bills {
        assert_eq!(
            [
                bill.cust_state.as_str(),
                &bill.cust_state_code,
                &bill.pos_state,
                &bill.pos_code,
                &bill.pay_mode,
                &bill.reverse_charge,
                &bill.cust_addr,
                &bill.transporter,
            ],
            [
                "Uttar Pradesh",
                "09",
                "Uttar Pradesh",
                "09",
                "Cash",
                "No",
                "",
                ""
            ]
        );
        assert_eq!((bill.customer_id.as_deref(), bill.created), (None, 0));
        for line in &bill.lines {
            assert_eq!(
                (
                    line.product,
                    line.unit,
                    line.packet_qty.get(),
                    line.hsn.as_str()
                ),
                (None, GstUnit::Case, 1, "")
            );
        }
    }
}
