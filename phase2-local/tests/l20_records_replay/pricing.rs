use std::collections::BTreeMap;

use super::model::{Cursor, Db, DeleteTick, Envelope, Product, Record, Row, Stamp, Supplier, Tick};
use super::price_columns::PriceSupplierEntry;
use super::replay::{Page, Replica, SheetCopy, SheetPush};
use super::saved::Saved;

#[path = "pricing_deltas.rs"]
mod deltas;

const SENDER: &str = "synthetic-sender";
const RECEIVER: &str = "synthetic-receiver";
const RETIRED: &str = "Retired Quote";
const RETIRED_ID: &str = "s-retired";
const PRODUCT: &str = "p-quote";

fn supplier(id: &str, name: &str) -> Row<Supplier> {
    Row::new(
        id,
        Supplier { name: name.into() },
        Stamp::new(1, 5, RECEIVER),
    )
}

fn product(quotes: &[(&str, Option<i64>)]) -> Row<Product> {
    Row::new(
        PRODUCT,
        Product {
            item: "Synthetic Bottle".into(),
            supplier_prices: quotes
                .iter()
                .map(|&(name, quote)| (name.into(), quote))
                .collect(),
        },
        Stamp::new(3, 10, RECEIVER),
    )
}

const fn tick(m: u64) -> Tick<'static> {
    Tick { m, d: SENDER }
}

#[test]
fn l20_07_full_copies_keep_raw_columns_and_quotes_and_replace_receiver_state() {
    // Given: PHASE1_CASES L20-07 / HTML 451, 471-489, 7750-7758, 7851-7859, 7941-7948, 10466-10479.
    let mut sender = Db {
        price_suppliers: vec!["Seed Column".into()],
        suppliers: vec![
            supplier("s-contact", "Contact Only"),
            supplier(RETIRED_ID, RETIRED),
            supplier("s-null", "Null Quote"),
        ],
        products: vec![product(&[(RETIRED, Some(0)), ("Null Quote", None)])],
        ..Db::default()
    };
    let divergent = Replica {
        db: Db {
            price_suppliers: vec!["Receiver Raw".into()],
            suppliers: vec![supplier("receiver-contact", "Receiver Contact")],
            ..Db::default()
        },
        cursor: Cursor {
            seq: 77,
            pushed_m: 500,
        },
    };

    // When: the two pricing-editor branches share the same append/stamp effect, then contacts are removed locally.
    assert!(sender.add_pricing_supplier("s-entry", " Entry Editor ", tick(20)));
    assert!(sender.add_pricing_supplier("s-compare", "Comparison Editor", tick(21)));
    assert!(!sender.add_pricing_supplier("unused", "Contact Only", tick(22)));
    sender.delete_contact(
        RETIRED_ID,
        DeleteTick {
            ts: 29,
            write: tick(30),
        },
    );
    sender.delete_contact(
        "s-null",
        DeleteTick {
            ts: 30,
            write: tick(31),
        },
    );

    // Then: full copies replace raw columns (HTML 9640-9650, 9711-9730, 9843-9849, 9967-10049).
    let raw = ["Seed Column", "Entry Editor", "Comparison Editor", RETIRED];
    let display = [
        "Seed Column",
        "Entry Editor",
        "Comparison Editor",
        RETIRED,
        "Contact Only",
    ];
    let quotes = BTreeMap::from([(RETIRED.into(), Some(0)), ("Null Quote".into(), None)]);
    let saved = Saved::new();
    saved.write("full.json", &sender);
    saved.write("sheet-push.json", &SheetPush::new(&sender));
    saved.write("sheet-copy.json", &SheetCopy { db: sender.clone() });
    let after_editors = sender
        .suppliers
        .iter()
        .map(|row| (&*row.id, &row.value.stamp))
        .collect::<Vec<_>>();
    assert_eq!(
        after_editors,
        [
            ("s-contact", &Stamp::new(1, 5, RECEIVER)),
            ("s-entry", &Stamp::new(1, 20, SENDER)),
            ("s-compare", &Stamp::new(1, 21, SENDER))
        ]
    );

    let full = saved.bytes("full.json");
    let mut restored = divergent.clone();
    restored.restore_json(&full).unwrap();
    assert_eq!(
        restored.cursor,
        Cursor {
            seq: 77,
            pushed_m: 0
        }
    );
    assert!(
        restored
            .db
            .changed(divergent.cursor.pushed_m, RECEIVER)
            .is_empty()
    );
    assert!(
        !restored
            .db
            .changed(restored.cursor.pushed_m, RECEIVER)
            .is_empty()
    );
    let mut rebuilt = divergent.clone();
    rebuilt.rebuild(&full, 81, 700).unwrap();
    assert_eq!(
        rebuilt.cursor,
        Cursor {
            seq: 81,
            pushed_m: 700
        }
    );
    let mut sheets = divergent.clone();
    sheets
        .accept_sheet_copy(&saved.bytes("sheet-copy.json"))
        .unwrap();
    assert_eq!(sheets.cursor, divergent.cursor);
    let pushed: SheetCopy = saved.read("sheet-push.json");
    for (file, db) in [
        ("restore.json", &restored.db),
        ("rebuild.json", &rebuilt.db),
        ("sheets.json", &sheets.db),
    ] {
        saved.write(file, db);
        assert_eq!(saved.read::<Db>(file), *db);
    }
    for readback in [
        &sender,
        &saved.read("full.json"),
        &pushed.db,
        &restored.db,
        &rebuilt.db,
        &sheets.db,
    ] {
        assert_eq!(readback, &sender);
        assert_eq!(readback.price_suppliers, raw.map(PriceSupplierEntry::from));
        assert_eq!(readback.columns(), display);
        assert_eq!(readback.products[0].value.data.supplier_prices, quotes);
    }

    let arrival = supplier("catchup", "Catch-up Contact");
    let entry = Envelope::from_row(&arrival, Record::Suppliers(arrival.clone()), RECEIVER);
    assert_eq!(
        rebuilt.pull(
            &Page {
                max_seq: 82,
                records: vec![entry]
            },
            SENDER
        ),
        [true]
    );
    assert_eq!(
        rebuilt.cursor,
        Cursor {
            seq: 82,
            pushed_m: 700
        }
    );
    assert_eq!(
        rebuilt.db.price_suppliers,
        raw.map(PriceSupplierEntry::from)
    );
    assert_eq!(
        rebuilt.db.columns().last().map(String::as_str),
        Some("Contact Only")
    );
    assert_eq!(rebuilt.db.columns()[raw.len()], "Catch-up Contact");
    saved.write("after-catch-up.json", &rebuilt.db);
    assert_eq!(saved.read::<Db>("after-catch-up.json"), rebuilt.db);
}

#[test]
fn l20_07_migrates_missing_and_nonarray_raw_lists_without_moving_quotes() {
    // Given: two old synthetic snapshots, per HTML 471-489.
    let fixtures = [
        r#"{"products":[{"id":"p","item":"Bottle","supplierPrices":{"Off-column":400}}],"suppliers":[{"id":"s","name":"Fallback"}]}"#,
        r#"{"priceSuppliers":{"old":true},"products":[{"id":"p","item":"Bottle","supplierPrices":{"Off-column":400}}],"suppliers":[{"id":"s","name":"Fallback"}]}"#,
    ];
    let saved = Saved::new();
    for input in fixtures {
        // When: a valid old copy is read and written in its migrated form.
        let db: Db = serde_json::from_str(input).unwrap();
        saved.write("migrated.json", &db);
        // Then: no raw columns are fabricated from contacts or quote keys.
        assert!(Db::default().price_suppliers.is_empty());
        assert!(db.price_suppliers.is_empty());
        assert_eq!(db.columns(), ["Fallback"]);
        assert_eq!(
            db.products[0].value.data.supplier_prices,
            BTreeMap::from([("Off-column".into(), Some(400))])
        );
        assert_eq!(saved.read::<Db>("migrated.json"), db);
    }
}
