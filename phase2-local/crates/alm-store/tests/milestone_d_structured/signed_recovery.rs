use std::fs;
use std::path::Path;

use alm_store::synthetic_read_model::{AccountId, Error, Records, SaleId, Store};
use alm_store::{BillDate, Role};

pub(super) fn run(records: &Records, dir: &Path) {
    for (name, due, receipt, expected) in [
        ("positive-49", 849, 800, "₹0.00"),
        ("positive-50", 850, 800, "₹1.00"),
        ("positive-51", 851, 800, "₹1.00"),
        ("negative-49", 751, 800, "₹0.00"),
        ("negative-50", 750, 800, "₹0.00"),
        ("negative-51", 749, 800, "₹-1.00"),
        ("positive-150", 950, 800, "₹2.00"),
        ("negative-150", 650, 800, "₹-1.00"),
        ("aggregate-first", 849, 799, "₹1.00"),
    ] {
        let mut input = records.clone();
        input.sales[1].paid_minor = input.sales[1].total_minor - due;
        input.receipts[0].amount_minor = receipt;
        let mut linked = super::scope();
        linked.sales[1].1 = AccountId::new(super::super::fixture::BUYER_ID.into());
        linked.zero_openings.pop();
        let source = dir.join(format!("{name}-source.json"));
        let restored = dir.join(format!("{name}-restored.json"));
        let backup_path = dir.join(format!("{name}-backup.json"));
        let store = Store::create_scoped(&source, Role::Owner, input.clone(), linked).unwrap();
        let archive = store.backup(Role::Owner).unwrap();
        fs::write(&backup_path, &archive).unwrap();
        drop(store);
        let source_store = Store::open(&source, Role::Owner).unwrap();
        let source_html = document(&source_store, &source, &input, expected);
        fs::write(dir.join(format!("{name}-source.html")), &source_html).unwrap();
        drop(source_store);
        let saved_backup = fs::read(&backup_path).unwrap();
        assert_eq!(saved_backup, archive);
        Store::restore(&restored, Role::Owner, &saved_backup).unwrap();
        let restored_store = Store::open(&restored, Role::Owner).unwrap();
        assert_eq!(restored_store.backup(Role::Owner).unwrap(), saved_backup);
        let restored_html = document(&restored_store, &restored, &input, expected);
        fs::write(dir.join(format!("{name}-restored.html")), &restored_html).unwrap();
        assert_eq!(source_html, restored_html);
    }
}

fn document(store: &Store, path: &Path, input: &Records, expected: &str) -> String {
    let before = fs::read(path).unwrap();
    let backup = store.backup(Role::Owner).unwrap();
    let id = SaleId::new("sale-cancelled-id".into());
    let date = BillDate::new(2025, 3, 1).unwrap();
    assert_eq!(store.records(Role::Owner).unwrap(), input);
    assert!(matches!(
        store.records(Role::Manager),
        Err(Error::AccessDenied)
    ));
    assert!(matches!(
        store.backup(Role::Manager),
        Err(Error::AccessDenied)
    ));
    assert!(matches!(
        store.estimate_html(Role::Manager, &id, date),
        Err(Error::AccessDenied)
    ));
    let html = store.estimate_html(Role::Owner, &id, date).unwrap();
    for anchor in [
        "<tr><th>Bill Amount</th><td>₹410.00</td></tr>".to_owned(),
        "<tr><th>Paid on this bill</th><td>₹100.00</td></tr>".to_owned(),
        format!("<tr><th>Total Due (as on 01-Mar-2025)</th><td>{expected}</td></tr>"),
    ] {
        assert!(html.contains(&anchor), "missing {anchor}: {html}");
    }
    assert_eq!(store.backup(Role::Owner).unwrap(), backup);
    assert_eq!(fs::read(path).unwrap(), before);
    html
}
