use serde_json::{Value, json};

use super::{COLLECTIONS, LegacyBackup, LegacyError, MigrateClock};

pub(crate) const A: &[u8] = include_bytes!("../../tests/legacy/rehearsal-a.json");
pub(crate) const B: &[u8] = include_bytes!("../../tests/legacy/rehearsal-b.json");

pub(crate) const CLOCK: MigrateClock<'static> = MigrateClock {
    now_ms: 1_767_900_000_000,
    today: "2026-01-08",
};

#[test]
fn restore_requires_a_json_object_with_products() {
    assert_eq!(
        LegacyBackup::parse(b"not json", CLOCK),
        Err(LegacyError::NotJson)
    );
    assert_eq!(
        LegacyBackup::parse(b"[1]", CLOCK),
        Err(LegacyError::NotJson)
    );
    for missing in [&b"{}"[..], b"{\"products\": null}", b"{\"products\": 0}"] {
        assert_eq!(
            LegacyBackup::parse(missing, CLOCK),
            Err(LegacyError::NoProducts)
        );
    }
    // `[]` is truthy in JavaScript: an empty shop restores.
    let empty = LegacyBackup::parse(b"{\"products\": []}", CLOCK).unwrap();
    assert_eq!(empty.counts().len(), COLLECTIONS.len());
    // `migrate()` logs the v1.3 upgrade; nothing else is created.
    assert!(
        empty
            .counts()
            .iter()
            .all(|(name, n)| *n == usize::from(matches!(*name, "meta" | "log")))
    );
}

#[test]
fn the_import_clock_comes_from_the_file_so_a_reparse_is_identical() {
    let b = LegacyBackup::parse_for_import(B).unwrap();
    assert_eq!(b, LegacyBackup::parse_for_import(B).unwrap());
    // The latest stamp in rehearsal-b is the log entry's ts (2025-06-01).
    let migrated = b
        .stock_log()
        .into_iter()
        .find(|e| e.reason == "Migrated balance (old adjustments)")
        .unwrap();
    assert_eq!(migrated.id, "adj_mig5");
    assert_eq!(
        b.collection("stockLog")[1]["date"],
        json!("2025-06-01"),
        "{:?}",
        b.collection("stockLog")
    );
}

#[test]
fn every_collection_is_read_with_its_records() {
    let a = LegacyBackup::parse(A, CLOCK).unwrap();
    assert_eq!(a.source_schema, 3);
    assert_eq!(a.digest, super::sha256_hex(A));
    assert_eq!(
        a.counts(),
        [
            ("products", 4),
            ("customers", 3),
            ("suppliers", 2),
            ("sales", 7),
            ("purchases", 1),
            ("purchaseOrders", 1),
            ("expenses", 2),
            ("payments", 5),
            ("stock", 3),
            ("crates", 1),
            ("priceSuppliers", 1),
            ("gstBills", 2),
            ("stockLog", 3),
            ("log", 1),
            ("godown", 1),
            ("deleted", 2),
            ("meta", 1),
        ]
    );
    // A current backup needs no structural fix-up; only the HTML's derived defaults.
    assert_eq!(a.fixes, ["products: openingCost from avgCost/buyRate"]);
    let sales = a.sales();
    let crate_bill = sales.iter().find(|s| s.no == 1004.0).unwrap();
    let (name, components) = crate_bill.lines[0].crate_line.as_ref().unwrap();
    assert_eq!((name.as_str(), components.len()), ("Party Crate", 2));
    assert!(sales.iter().find(|s| s.no == 1003.0).unwrap().cancelled);
    assert_eq!(a.cash_days()[1].misc_desc, "tea");
    assert_eq!(a.stock_entries()[0].key, "p_guava");
    assert_eq!(
        a.purchase_lots().len(),
        4,
        "two purchase lines and two PO receipts"
    );
    assert_eq!(
        a.line_packets(&crate_bill.lines[0]),
        [
            ("p_guava".to_owned(), "Synthetic Guava".to_owned(), 4.0),
            ("p_tonic".to_owned(), "Synthetic Tonic".to_owned(), 6.0)
        ]
    );
}

fn product_id(b: &LegacyBackup, item: &str) -> String {
    b.product_by_item(item).unwrap().id
}

#[test]
fn migrate_fixes_every_legacy_shape_as_the_html_does() {
    let b = LegacyBackup::parse(B, CLOCK).unwrap();
    assert_eq!(b.source_schema, 0);
    let meta = b.meta();
    assert_eq!(meta["schema"], json!(3));
    assert_eq!(meta["gstRate"], json!(18));
    assert_eq!(meta["gstHsn"], json!("2202"));
    assert_eq!(meta["defaultReorder"], json!(5));
    assert_eq!(meta["rev"], json!(0));
    assert_eq!(meta["_d"], json!("legacy"));
    // The shop identity defaults are not copied from the HTML: blank when missing.
    assert_eq!(meta["name"], json!("Synthetic Old Shop"));
    for key in ["owner", "mobile", "address", "gst", "fssai", "msme"] {
        assert_eq!(meta[key], json!(""), "{key}");
    }
    // Non-records dropped, names made text, ids assigned in order.
    let products = b.products();
    let items: Vec<_> = products.iter().map(|p| p.item.as_str()).collect();
    assert_eq!(
        items,
        ["Synthetic Mango", "Synthetic Mango", "Synthetic Berry", "7"]
    );
    let ids: Vec<_> = products.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(ids, ["p_mig1", "p_mig2", "p_mig3", "p_mig4"]);
    let berry = &products[2];
    assert_eq!(
        (berry.avg_cost, berry.opening_cost, berry.sell_rate),
        (11.0, 11.0, 15.0)
    );
    assert_eq!(products[0].avg_cost, 20.0, "avgCost from buyRate");
    let customers = b.customers();
    assert_eq!(customers.len(), 2);
    assert_eq!(customers[1].name, "404");
    assert_eq!(customers[0].opening, 25.0);
    let suppliers = b.suppliers();
    assert_eq!(
        suppliers
            .iter()
            .map(|s| (s.id.as_str(), s.name.as_str()))
            .collect::<Vec<_>>(),
        [("s_mig7", "Synthetic Old Supplier"), ("s_mig8", "")]
    );
    // Lines: broken item lists repaired, pid linked by exact name (first of duplicates wins).
    let sales = b.sales();
    assert_eq!(sales[0].lines.len(), 1);
    assert_eq!(sales[0].lines[0].pid.as_deref(), Some("p_mig3"));
    assert_eq!(
        (sales[0].no, sales[0].lines[0].qty, sales[0].paid),
        (7.0, 3.0, 10.0)
    );
    assert!(sales[1].lines.is_empty());
    let order = &b.collection("purchaseOrders")[0];
    assert_eq!(order["supId"], json!("s_mig7"));
    assert_eq!(order["items"][0]["pid"], json!("p_mig1"));
    // Stock re-keyed by product ID; the old adjust counters become effective log entries.
    let stock = b.stock_entries();
    let keys: Vec<_> = stock.iter().map(|s| s.key.as_str()).collect();
    assert_eq!(keys, ["n:Old Cola", "p_mig1", "p_mig3"]);
    assert_eq!((stock[2].opening, stock[2].fridge), (4.0, 1.0));
    let log: Vec<_> = b
        .stock_log()
        .into_iter()
        .map(|e| (e.key, e.delta, e.eff))
        .collect();
    assert_eq!(
        log,
        [
            ("p_mig3".to_owned(), 1.0, false),
            ("n:Old Cola".to_owned(), 1.0, true),
            ("p_mig3".to_owned(), -2.0, true),
        ]
    );
    // Not-a-list collections start empty; godown item made text.
    assert!(b.crates().is_empty());
    assert!(b.collection("deleted").is_empty());
    assert_eq!(b.godown()[0].item, "");
    // The hygiene and upgrade notes the HTML logs.
    let messages: Vec<String> = b
        .collection("log")
        .iter()
        .map(|e| e["msg"].as_str().unwrap_or_default().to_owned())
        .collect();
    assert_eq!(messages.len(), 3);
    assert!(messages[1].contains("duplicate product name(s): Synthetic Mango."));
    assert!(
        messages[1]
            .contains("1 old product name(s) still hold stock history from renames: Old Cola")
    );
    assert!(messages[2].starts_with("Upgraded data to v1.3"));
    assert!(
        b.collection("log")
            .iter()
            .all(|e| e["id"].is_string() && e["_v"] == json!(1))
    );
    // Deterministic: the same file and clock give the same result.
    assert_eq!(LegacyBackup::parse(B, CLOCK).unwrap(), b);
    assert_eq!(product_id(&b, "Synthetic Berry"), "p_mig3");
    assert_eq!(b.key_of_name("Old Cola"), "n:Old Cola");
    let _: &Value = &b.db["meta"];
}

mod import {
    use super::{A, B, CLOCK};
    use crate::legacy::import::{EntryStatus, LegacyImports, apply, plan};
    use crate::legacy::{LegacyBackup, LineStatus, dry_run, reconcile};

    fn quarantine(imports: &LegacyImports, id: u64) -> Vec<(String, String, String)> {
        imports.manifests[usize::try_from(id - 1).unwrap()]
            .entries
            .iter()
            .filter(|e| e.status == EntryStatus::Quarantined)
            .map(|e| (e.kind.clone(), e.legacy_id.clone(), e.reason.clone()))
            .collect()
    }

    #[test]
    fn a_imports_with_a_manifest_and_quarantines_what_cannot_be_mapped() {
        let a = LegacyBackup::parse(A, CLOCK).unwrap();
        let p = plan(&a);
        assert_eq!(p.digest, a.digest);
        assert_eq!(p.version, "legacy-map-1");
        let mut imports = LegacyImports::default();
        let id = apply(&mut imports, &p);
        assert_eq!(id, 1);
        let q = quarantine(&imports, id);
        let q: Vec<_> = q
            .iter()
            .map(|(k, l, r)| (k.as_str(), l.as_str(), r.as_str()))
            .collect();
        assert_eq!(
            q,
            [
                ("stock_log", "adj_2", "delta -0.5 is not a whole quantity"),
                ("payment", "pay_4", "amount 10.005 is below a paisa"),
                (
                    "payment",
                    "pay_9",
                    "deleted in the same file (tombstone payments:pay_9)"
                ),
                ("sale", "b_7", "unresolved reference customer c_gone"),
            ]
        );
        let m = &imports.manifests[0];
        assert_eq!(m.count(EntryStatus::Imported), imports.records.len());
        assert_eq!(m.count(EntryStatus::Already), 0);
        // Every legacy record is accounted for: imported or quarantined.
        let total: usize = a
            .counts()
            .iter()
            .filter(|(name, _)| *name != "meta")
            .map(|(_, n)| n)
            .sum::<usize>()
            + 1;
        assert_eq!(m.entries.len(), total);
        let sale = imports
            .records
            .iter()
            .find(|r| r.legacy_id == "b_2")
            .unwrap();
        assert_eq!(sale.authority_id, "sale-5");
        assert_eq!(sale.body["total"], serde_json::json!(30_600));
        assert_eq!(sale.body["items"][1]["rate"], serde_json::json!(6_000));

        // The same file again: nothing changes, the same manifest.
        let before = imports.clone();
        assert_eq!(apply(&mut imports, &plan(&a)), 1);
        assert_eq!(imports, before);
    }

    #[test]
    fn a_changed_file_maps_known_records_and_quarantines_conflicts() {
        let a = LegacyBackup::parse(A, CLOCK).unwrap();
        let mut imports = LegacyImports::default();
        apply(&mut imports, &plan(&a));
        let records = imports.records.len();
        let edited = String::from_utf8(A.to_vec())
            .unwrap()
            .replace("\"amount\": 91,", "\"amount\": 92,");
        let edited = LegacyBackup::parse(edited.as_bytes(), CLOCK).unwrap();
        assert_ne!(edited.digest, a.digest);
        let id = apply(&mut imports, &plan(&edited));
        assert_eq!(id, 2);
        assert_eq!(imports.records.len(), records, "no duplicates");
        let m = &imports.manifests[1];
        assert_eq!(m.count(EntryStatus::Imported), 0);
        assert_eq!(m.count(EntryStatus::Already), records - 1);
        assert!(
            quarantine(&imports, 2).contains(&(
                "payment".into(),
                "pay_1".into(),
                "differs from the imported record payment-3".into()
            )),
            "{:?}",
            quarantine(&imports, 2)
        );
    }

    #[test]
    fn reconciliation_explains_every_difference_by_quarantine() {
        let a = LegacyBackup::parse(A, CLOCK).unwrap();
        let mut imports = LegacyImports::default();
        let id = apply(&mut imports, &plan(&a));
        let report = reconcile(&a, &imports, id, (1, 1)).unwrap();
        assert_eq!(report, dry_run(&a, &LegacyImports::default(), (1, 1)));
        let unexplained: Vec<_> = report
            .sections
            .iter()
            .flat_map(|s| s.lines.iter().map(move |l| (s.name, l)))
            .filter(|(_, l)| matches!(l.status, LineStatus::Unexplained(_)))
            .map(|(s, l)| format!("{s}: {} {:?} {:?}", l.key, l.legacy, l.authority))
            .collect();
        // Only the numbering: the authority's live series starts at 1 (OPEN-003).
        assert_eq!(
            unexplained,
            [
                "Numbering (shown, not issued): invoice: legacy next vs authority live series Some(\"1008\") Some(\"1\")",
                "Numbering (shown, not issued): GST number: legacy next vs authority live series Some(\"3\") Some(\"1\")",
            ]
        );
        let line = |section: &str, key: &str| {
            report
                .sections
                .iter()
                .find(|s| s.name == section)
                .and_then(|s| s.lines.iter().find(|l| l.key == key))
                .cloned()
                .unwrap()
        };
        let invoice = line("Invoices", "#1007 (b_7)");
        assert_eq!(invoice.status, LineStatus::Explained);
        assert_eq!(invoice.authority, None);
        // Without the quarantined bill the imported series ends at #1006; the numbering line
        // keeps the legacy file's next number (#1008), so #1007 is never reissued.
        let next = line("Invoices", "next invoice (not issued)");
        assert_eq!(
            (
                next.legacy.as_deref(),
                next.authority.as_deref(),
                next.status
            ),
            (Some("#1008"), Some("#1007"), LineStatus::Explained)
        );
        let balance = line("Customer balances", "c_cust Synthetic Customer");
        assert_eq!(balance.legacy.as_deref(), Some("₹190"));
        assert_eq!(balance.status, LineStatus::Explained);
        assert_eq!(balance.authority.as_deref(), Some("₹200"));
        let guava = line("Stock by item and location", "p_guava Synthetic Guava");
        assert_eq!(guava.status, LineStatus::Explained);
        assert_eq!(
            (guava.legacy.as_deref(), guava.authority.as_deref()),
            (Some("shop 22.5 · fridge 2"), Some("shop 23 · fridge 2"))
        );
        assert_eq!(line("Invoices", "#1002 (b_2)").status, LineStatus::Match);
        assert_eq!(
            line("FIFO lots and value", "total value").status,
            LineStatus::Explained
        );
        assert_eq!(report.unexplained(), 2);
        // Live series at or past the legacy next numbers: no unexplained difference.
        let continued = reconcile(&a, &imports, id, (1008, 3)).unwrap();
        assert_eq!(continued.unexplained(), 0);
    }

    #[test]
    fn b_migrated_shapes_import_and_reconcile() {
        let b = LegacyBackup::parse(B, CLOCK).unwrap();
        let mut imports = LegacyImports::default();
        let id = apply(&mut imports, &plan(&b));
        let q = quarantine(&imports, id);
        let q: Vec<_> = q
            .iter()
            .map(|(k, l, r)| (k.as_str(), l.as_str(), r.as_str()))
            .collect();
        assert_eq!(
            q,
            [
                ("stock_log", "stockLog[0]", "no legacy ID"),
                ("godown", "godown[0]", "no legacy ID"),
            ]
        );
        let report = reconcile(&b, &imports, id, (1, 1)).unwrap();
        // The invoice series only: no GST bills, so the GST numbering matches.
        assert_eq!(report.unexplained(), 1, "{report:#?}");
        let continued = reconcile(&b, &imports, id, (9, 1)).unwrap();
        assert_eq!(continued.unexplained(), 0, "{continued:#?}");
    }
}
