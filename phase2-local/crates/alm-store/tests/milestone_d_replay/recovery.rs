//! Synthetic, versioned read-model archive only. It cannot restore posting authority.
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use rusqlite::Connection;

use alm_store::confirmation::{ConfirmationFixture, GstJurisdiction, GstLineMetadata, GstRequest};
use alm_store::{BillDate, DraftCustomerId, DraftId, ProductId, Role};

use super::sqlite_list::{open_read_only, sqlite_list};

const SCHEMA: &str =
    "CREATE TABLE records (name TEXT PRIMARY KEY, value TEXT NOT NULL) WITHOUT ROWID";
const NAMES: &str = "account|cashbook_feb|cashbook_mar|fifo_a|fifo_b|gst_invoice|profile|purchase_orders|sale|sales_csv|settings";
const CONTROLS: &str = "SELECT CASE WHEN (SELECT value FROM records WHERE name='settings') LIKE '%show_profile=1%' THEN (SELECT value FROM records WHERE name='profile') ELSE '' END; SELECT CASE WHEN (SELECT value FROM records WHERE name='settings') LIKE 'receipt_mode=online|%' THEN 'online' ELSE 'cash' END;";

fn sqlite(path: &Path, sql: &str, readonly: bool) -> Result<String, ()> {
    let conn = if readonly {
        open_read_only(path)
    } else {
        Connection::open(path)
    }
    .map_err(|_| ())?;
    sqlite_list(&conn, sql).map_err(|_| ())
}

fn snapshot(fixture: &ConfirmationFixture) -> Vec<(&'static str, String)> {
    let owner = Role::Owner;
    let buyer = DraftCustomerId::new("milestone-c-buyer".into());
    let sale = DraftId::new(21);
    let feb = BillDate::new(2025, 2, 21).unwrap();
    let mar = BillDate::new(2025, 3, 1).unwrap();
    let csv = fixture.sales_csv(owner, sale).unwrap();
    assert!(csv.lines().nth(1).is_some());
    assert!(csv.lines().skip(1).all(|row| row.ends_with(",CANCELLED")));
    let metadata = [
        GstLineMetadata {
            product: ProductId::new(1),
            name: "Synthetic A",
            product_rate: Some(1_800),
            product_cess: 0,
            product_hsn: Some("22029920"),
            line_rate: None,
            line_hsn: None,
        },
        GstLineMetadata {
            product: ProductId::new(2),
            name: "Synthetic B",
            product_rate: Some(1_200),
            product_cess: 0,
            product_hsn: Some("21069099"),
            line_rate: None,
            line_hsn: None,
        },
    ];
    let mut invoices = Vec::new();
    fixture
        .emit_gst_invoice(
            GstRequest {
                role: owner,
                draft_id: sale,
                jurisdiction: GstJurisdiction::Intra,
                lines: &metadata,
            },
            &mut |invoice| invoices.push(invoice),
        )
        .expect("project cancelled GST invoice");
    let [gst] = invoices.as_slice() else {
        panic!("expected one GST invoice")
    };
    assert!(gst.cancelled, "GST projection must be cancelled");
    vec![
        (
            "account",
            format!(
                "{:?};{:?}",
                fixture.customer_ledger(owner, &buyer).unwrap(),
                fixture.customer_due(buyer.value())
            ),
        ),
        (
            "cashbook_feb",
            format!("{:?}", fixture.cashbook_day_close(owner, feb).unwrap()),
        ),
        (
            "cashbook_mar",
            format!("{:?}", fixture.cashbook_day_close(owner, mar).unwrap()),
        ),
        (
            "fifo_a",
            format!(
                "{:?}",
                fixture.open_stock_lots(owner, ProductId::new(1)).unwrap()
            ),
        ),
        (
            "fifo_b",
            format!(
                "{:?}",
                fixture.open_stock_lots(owner, ProductId::new(2)).unwrap()
            ),
        ),
        ("gst_invoice", format!("{gst:?}")),
        (
            "profile",
            "Synthetic Recovery Shop|Synthetic Address".into(),
        ),
        (
            "purchase_orders",
            format!("{:?}", fixture.purchase_orders(owner).unwrap()),
        ),
        (
            "sale",
            format!("{:?}", fixture.current_sale(owner, sale).unwrap()),
        ),
        ("sales_csv", csv),
        ("settings", "receipt_mode=online|show_profile=1".into()),
    ]
}

fn readback(path: &Path) -> Result<String, ()> {
    sqlite(
        path,
        "SELECT name || ':' || hex(value) FROM records ORDER BY name;",
        true,
    )
}

fn valid(path: &Path) -> Result<(), ()> {
    let check = sqlite(
        path,
        "PRAGMA trusted_schema=OFF; PRAGMA integrity_check; PRAGMA user_version; SELECT count(*) FROM sqlite_schema WHERE name NOT LIKE 'sqlite_%'; SELECT sql FROM sqlite_schema WHERE type='table' AND name='records'; SELECT group_concat(name, '|') FROM (SELECT name FROM records ORDER BY name); SELECT count(*) FROM records WHERE typeof(name) != 'text' OR typeof(value) != 'text' OR length(value)=0;",
        true,
    )?;
    let expected = format!("ok\n2\n1\n{SCHEMA}\n{NAMES}\n0\n");
    if check != expected {
        return Err(());
    }
    Ok(())
}

fn backup(role: Role, source: &Path, archive: &Path) -> Result<(), ()> {
    if role != Role::Owner || archive.exists() {
        return Err(());
    }
    valid(source)?;
    let name = archive.to_str().ok_or(())?;
    Connection::open(source)
        .and_then(|conn| conn.execute("VACUUM INTO ?1", [name]))
        .map_err(|_| ())?;
    Ok(())
}

fn restore(role: Role, archive: &Path, target: &Path) -> Result<(), ()> {
    if role != Role::Owner || target.exists() {
        return Err(());
    }
    let bytes = fs::read(archive).map_err(|_| ())?;
    valid(archive)?;
    if fs::read(archive).map_err(|_| ())? != bytes {
        return Err(());
    }
    let staging = target.with_extension("recovery-stage");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&staging)
        .map_err(|_| ())?;
    let result = (|| {
        file.write_all(&bytes).map_err(|_| ())?;
        file.sync_all().map_err(|_| ())?;
        fs::hard_link(&staging, target).map_err(|_| ())
    })();
    drop(file);
    fs::remove_file(staging).map_err(|_| ())?;
    result
}

pub(super) fn exercise(fixture: &ConfirmationFixture, dir: &Path) {
    let source = dir.join("d-read-model-source.sqlite");
    let archive = dir.join("d-read-model-v2.sqlite");
    let restored = dir.join("d-read-model-restored.sqlite");
    let rows = snapshot(fixture);
    assert!(!source.exists() && !archive.exists() && !restored.exists());
    let mut conn = Connection::open(&source).unwrap();
    let tx = conn.transaction().unwrap();
    tx.execute_batch(&format!("PRAGMA user_version=2; {SCHEMA};"))
        .unwrap();
    for (name, value) in &rows {
        tx.execute("INSERT INTO records VALUES (?1, ?2)", (name, value))
            .unwrap();
    }
    tx.commit().unwrap();
    drop(conn);
    let before = readback(&source).unwrap();
    let controls = sqlite(&source, CONTROLS, true).expect("read source settings controls");
    assert_eq!(
        controls,
        "Synthetic Recovery Shop|Synthetic Address\nonline\n"
    );
    assert!(backup(Role::Manager, &source, &archive).is_err());
    assert!(!archive.exists());
    backup(Role::Owner, &source, &archive).unwrap();
    assert!(restore(Role::Manager, &archive, &restored).is_err());
    assert!(!restored.exists());
    let bad = dir.join("d-invalid.sqlite");
    fs::write(&bad, b"not a sqlite archive").unwrap();
    assert!(restore(Role::Owner, &bad, &restored).is_err());
    assert!(!restored.exists());
    let wrong_version = dir.join("d-wrong-version.sqlite");
    fs::copy(&archive, &wrong_version).unwrap();
    sqlite(&wrong_version, "PRAGMA user_version=3;", false).unwrap();
    assert!(restore(Role::Owner, &wrong_version, &restored).is_err());
    assert!(!restored.exists());
    assert_eq!(readback(&source).unwrap(), before);
    restore(Role::Owner, &archive, &restored).unwrap();
    assert_eq!(readback(&restored).unwrap(), before);
    assert!(restore(Role::Owner, &archive, &restored).is_err());
    assert_eq!(readback(&restored).unwrap(), before);
    assert_eq!(
        sqlite(&restored, CONTROLS, true).expect("read restored settings controls"),
        controls
    );
    let hex = sqlite(
        &restored,
        "SELECT hex(value) FROM records WHERE name='sales_csv';",
        true,
    )
    .expect("read restored Sales CSV");
    let encoded = hex.trim_end().as_bytes();
    assert_eq!(encoded.len() % 2, 0, "invalid restored CSV encoding");
    let csv = encoded
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let digits = std::str::from_utf8(pair).expect("SQLite hex is UTF-8");
            u8::from_str_radix(digits, 16).expect("SQLite returned valid hex")
        })
        .collect::<Vec<_>>();
    let exported = dir.join("d-restored-sales.csv");
    fs::write(&exported, &csv).expect("write restored Sales CSV export");
    assert_eq!(
        csv,
        fs::read(dir.join("sales.csv")).expect("read original Sales CSV")
    );
}

#[test]
fn archive_captures_separate_sales_csv_and_cancelled_gst_projection() {
    let fixture = super::fixture::cancelled_fixture();
    let rows = snapshot(&fixture);
    let value = |name| {
        rows.iter()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| value)
    };
    assert_eq!(
        value("sales_csv"),
        Some(&fixture.sales_csv(Role::Owner, DraftId::new(21)).unwrap())
    );
    let gst = value("gst_invoice").expect("actual GST projection must be archived");
    assert!(gst.contains("cancelled: true"));
    assert!(gst.contains("22029920") && gst.contains("21069099"));
}
