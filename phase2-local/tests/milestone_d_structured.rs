use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use alm_phase2_local::confirmation::{CustomerLedgerKind, ReceiptMode};
use alm_phase2_local::synthetic_read_model::{
    Component, Customer, Line, LineKind, Product, Receipt, Records, Sale, Store,
};
use alm_phase2_local::{BillDate, DraftCustomerId, DraftId, Role};

#[path = "milestone_d_structured/assertions.rs"]
mod assertions;
#[path = "milestone_d_structured/estimate.rs"]
mod estimate;
#[path = "milestone_d_replay/fixture.rs"]
mod fixture;
#[path = "milestone_d_structured/gst.rs"]
mod gst;
#[path = "common/ordinary_gst_fixture.rs"]
mod ordinary_gst_fixture;

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
    let (review, receipt) = fixture.current_sale(Role::Owner, DraftId::new(21)).unwrap();
    let lines = review
        .draft()
        .lines()
        .iter()
        .zip(review.unit_prices())
        .enumerate()
        .map(|(i, (line, price))| Line {
            item: if i == 0 { "Synthetic A" } else { "Synthetic B" }.into(),
            pack: "12".into(),
            uom: "pkt".into(),
            packet_quantity: 12,
            kind: LineKind::Item,
            rate_minor: price.minor_units(),
            quantity: line.quantity().get(),
            amount_minor: price.minor_units() * i64::from(line.quantity().get()),
        })
        .collect();
    let cancelled = Sale {
        id: "sale-cancelled-id".into(),
        date: review.draft().bill_date().unwrap(),
        number: receipt.invoice,
        customer: "Synthetic Buyer".into(),
        mode: "Cash".into(),
        lines,
        total_minor: receipt.total.minor_units(),
        paid_minor: receipt.received.unwrap().minor_units(),
        cancelled: true,
    };
    let active = Sale {
        id: "sale-active-id".into(),
        date: BillDate::new(2025, 1, 1).unwrap(),
        number: 99,
        customer: "Control".into(),
        mode: "Online".into(),
        lines: vec![
            Line {
                item: "Control item".into(),
                pack: "12".into(),
                uom: "pc".into(),
                packet_quantity: 12,
                kind: LineKind::Item,
                rate_minor: 100,
                quantity: 6,
                amount_minor: 600,
            },
            Line {
                item: "Control bundle".into(),
                pack: "".into(),
                uom: "".into(),
                packet_quantity: 1,
                kind: LineKind::Crate {
                    components: vec![Component {
                        item: "hidden-component-id".into(),
                        quantity: 3,
                    }],
                },
                rate_minor: 250,
                quantity: 2,
                amount_minor: 500,
            },
        ],
        total_minor: 1100,
        paid_minor: 300,
        cancelled: false,
    };
    let ledger = fixture
        .customer_ledger(
            Role::Owner,
            &DraftCustomerId::new("milestone-c-buyer".into()),
        )
        .unwrap();
    let [receipt_entry] = ledger.entries.as_slice() else {
        panic!("expected C receipt")
    };
    let CustomerLedgerKind::Receipt { mode } = receipt_entry.kind else {
        panic!("expected receipt")
    };
    let mode = match mode {
        ReceiptMode::Cash => "Cash",
        ReceiptMode::Online => "Online",
    };
    let records = Records {
        reorder_default: 5,
        products: vec![Product {
            item: "Synthetic A".into(),
            company: "Synthetic".into(),
            pack: "12".into(),
            uom: "pkt".into(),
            packet_quantity: 12,
            mrp_minor: 20000,
            buy_minor: 10000,
            sell_minor: 17700,
            purchased: 5,
            sold: 0,
            on_hand: 5,
            reorder: 5,
        }],
        customers: vec![Customer {
            name: "Synthetic Buyer".into(),
            mobile: "".into(),
            address: "".into(),
            balance_minor: 0,
        }],
        sales: vec![cancelled, active],
        receipts: vec![Receipt {
            id: "receipt-standalone-id".into(),
            date: receipt_entry.date,
            mode: mode.into(),
            customer: "Synthetic Buyer".into(),
            amount_minor: receipt_entry.credit.minor_units(),
        }],
    };
    let tax = gst::snapshot(&records.sales[0], review.draft().lines());
    drop(fixture);
    let mut expected = records.clone();
    expected.reorder_default = 13;
    let mut store =
        Store::create_scoped_with_gst(&source, Role::Owner, records, estimate::scope(), vec![tax])
            .unwrap();
    estimate::save_profile(&mut store);
    assert!(store.set_reorder_default(Role::Manager, 13).is_err());
    store.set_reorder_default(Role::Owner, 13).unwrap();
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
    let regenerated_html = estimate::document(&reopened, &restored);
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
    fs::write(dir.join("regenerated-sales.csv"), &csv).unwrap();
    assert_eq!(
        csv,
        concat!(
            "Date,Bill No,Customer,Mode,Item,Pack,Rate,Pkt,Amount,Bill Total,Paid,Status\n",
            "2025-01-01,99,Control,Online,Control item,12pc,1,6,6,11,3,\n",
            "2025-01-01,99,Control,Online,Crate: Control bundle,,2.5,2,5,,,\n",
            "2025-02-21,1,Synthetic Buyer,Cash,Synthetic A,12pkt,177,2,354,410,100,CANCELLED\n",
            "2025-02-21,1,Synthetic Buyer,Cash,Synthetic B,12pkt,56,1,56,,,CANCELLED"
        )
    );
    let xml = reopened.workbook(Role::Owner).unwrap();
    assert_eq!(xml, original_xml);
    fs::write(dir.join("regenerated-workbook.xls"), &xml).unwrap();
    assertions::product_setting(&reopened);
    assertions::projections(&csv, &xml);
    if std::env::var_os("ALM_STRUCTURED_RETAIN").is_some() {
        println!("Structured replay evidence: {}", dir.display());
    } else {
        fs::remove_dir_all(dir).unwrap();
    }
}
