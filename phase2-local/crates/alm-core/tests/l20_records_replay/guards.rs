use std::collections::BTreeMap;

use super::model::{
    Cursor, Db, Envelope, GstBill, Marked, Meta, Origin, Record, Row, Stamp, Stock, Tick,
};
use super::replay::{Page, Replica};
use super::saved::Saved;

const LOCAL: &str = "local-device";
const FOREIGN: &str = "foreign-device";

#[test]
fn stamp_uses_previous_version_then_record_version_and_current_clock_and_device() {
    // Given: HTML 961-966, including the fallback when the previous version is zero.
    let tick = Tick { m: 101, d: LOCAL };
    for (start, prev, expected) in [(0, None, 1), (7, None, 8), (7, Some(0), 8), (7, Some(2), 3)] {
        // When: a synthetic write is stamped.
        let mut stamp = Stamp::new(start, 444, FOREIGN);
        stamp.write(prev, tick);
        // Then: the selected version increments, replacing both previous clock and device.
        assert_eq!(stamp, Stamp::new(expected, 101, LOCAL));
    }
}

#[test]
fn changes_sort_by_clock_and_use_strict_push_watermark_and_envelope_defaults() {
    // Given: HTML 9451, 9472-9505, 9581-9595; include exact-watermark and empty-array-ID controls.
    let row = |id, v, m, d| Row::new(id, GstBill { total: 700 }, Stamp::new(v, m, d));
    let db = Db {
        gst_bills: vec![
            row("later", 2, 54, FOREIGN),
            row("equal-a", 0, 53, ""),
            row("at", 1, 52, LOCAL),
            row("", 9, 955, LOCAL),
            row("equal-b", 4, 53, FOREIGN),
            row("older", 11, 50, LOCAL),
        ],
        stock: BTreeMap::from([(
            "p-stock".into(),
            Marked::new(
                Stock {
                    opening: 8,
                    fridge: 2,
                },
                Stamp::new(7, 53, FOREIGN),
            ),
        )]),
        meta: Marked::new(
            Meta {
                rev: 4,
                schema: 2,
                default_reorder: 6,
            },
            Stamp::new(8, 53, LOCAL),
        ),
        ..Db::default()
    };
    let mut replica = Replica {
        db,
        cursor: Cursor {
            seq: 19,
            pushed_m: 52,
        },
    };

    // When: collect a scripted successful push batch and acknowledge the sent envelope clocks.
    let changes = replica.db.changed(replica.cursor.pushed_m, LOCAL);
    let actual = changes
        .iter()
        .map(|e| (&*e.id, e.v, e.m, &*e.d))
        .collect::<Vec<_>>();
    assert_eq!(
        actual,
        [
            ("equal-a", 1, 53, LOCAL),
            ("equal-b", 4, 53, FOREIGN),
            ("p-stock", 7, 53, FOREIGN),
            ("meta", 8, 53, LOCAL),
            ("later", 2, 54, FOREIGN)
        ]
    );
    let Record::GstBills(defaulted) = &changes[0].record else {
        panic!("ordinary outgoing record")
    };
    assert_eq!(defaulted.value.stamp, Stamp::new(0, 53, ""));
    replica.acknowledge_push(&changes);
    replica.acknowledge_push(&changes[..1]);

    // Then: only the push watermark advances, and a re-scan sends none of the acknowledged records.
    assert_eq!(
        replica.cursor,
        Cursor {
            seq: 19,
            pushed_m: 54
        }
    );
    assert!(
        replica
            .db
            .changed(replica.cursor.pushed_m, LOCAL)
            .is_empty()
    );
    let saved = Saved::new();
    saved.write("outgoing.json", &changes);
    saved.write("cursor.json", &replica.cursor);
    assert_eq!(saved.read::<Vec<Envelope>>("outgoing.json"), changes);
    assert_eq!(saved.read::<Cursor>("cursor.json"), replica.cursor);
}

#[test]
fn pull_uses_payload_versions_preserves_local_meta_and_advances_cursor_on_skips() {
    // Given: HTML 9508-9530, 9588, 9599-9615. Envelope v/m deliberately disagree with payloads.
    let meta = |rev, schema, default_reorder, v| {
        Marked::new(
            Meta {
                rev,
                schema,
                default_reorder,
            },
            Stamp::new(v, 21, FOREIGN),
        )
    };
    let stock =
        |opening, fridge, v| Marked::new(Stock { opening, fridge }, Stamp::new(v, 22, FOREIGN));
    let base = Db {
        meta: meta(91, 7, 4, 5),
        stock: BTreeMap::from([("p".into(), stock(12, 6, 3))]),
        gst_bills: vec![Row::new(
            "g",
            GstBill { total: 500 },
            Stamp::new(4, 20, LOCAL),
        )],
        ..Db::default()
    };
    let mut replica = Replica {
        db: base.clone(),
        cursor: Cursor {
            seq: 80,
            pushed_m: 900,
        },
    };
    let inbound = |id: &str, record, v, d: &str| Envelope {
        id: id.into(),
        record,
        v,
        m: 9_999,
        d: d.into(),
    };
    let same_device_meta = inbound("meta", Record::Meta(meta(0, 1, 15, 7)), 10, LOCAL);
    let same_device_stock = inbound("p", Record::Stock(stock(39, 2, 6)), 10, LOCAL);
    let skipped = Page {
        max_seq: 88,
        records: vec![
            same_device_meta.clone(),
            same_device_stock.clone(),
            inbound("meta", Record::Meta(meta(1, 2, 99, 5)), 20, FOREIGN),
            inbound("p", Record::Stock(stock(100, 0, 2)), 20, FOREIGN),
            inbound("p", Record::Stock(stock(200, 0, 3)), 20, FOREIGN),
            inbound(
                "g",
                Record::GstBills(Row::new(
                    "g",
                    GstBill { total: 200 },
                    Stamp::new(3, 90, FOREIGN),
                )),
                20,
                FOREIGN,
            ),
            inbound(
                "g",
                Record::GstBills(Row::new(
                    "g",
                    GstBill { total: 300 },
                    Stamp::new(4, 90, FOREIGN),
                )),
                20,
                FOREIGN,
            ),
        ],
    };
    let applied = Page {
        max_seq: 87,
        records: vec![
            inbound("meta", Record::Meta(meta(1, 2, 9, 6)), 1, FOREIGN),
            inbound("p", Record::Stock(stock(17, 5, 4)), 1, FOREIGN),
            inbound("p-new", Record::Stock(stock(4, 1, 0)), 1, FOREIGN),
        ],
    };

    // When: replay a skipped page, then changes whose response cursor is behind the local cursor.
    let saved = Saved::new();
    saved.write("db.json", &base);
    assert_eq!(
        replica.pull(&skipped, LOCAL),
        vec![false; skipped.records.len()]
    );
    saved.write("cursor.json", &replica.cursor);
    assert_eq!(replica.db, base);
    assert_eq!(saved.read::<Db>("db.json"), base);
    assert_eq!(
        saved.read::<Cursor>("cursor.json"),
        Cursor {
            seq: 88,
            pushed_m: 900
        }
    );
    assert_eq!(replica.pull(&applied, LOCAL), [true, true, true]);
    assert_eq!(replica.db.meta, meta(91, 7, 9, 6));
    assert_eq!(replica.db.stock["p"], stock(17, 5, 4));
    assert_eq!(replica.db.stock["p-new"], stock(4, 1, 0));
    assert!(
        replica
            .db
            .apply(&same_device_meta, LOCAL, Origin::StalePush)
    );
    assert!(
        replica
            .db
            .apply(&same_device_stock, LOCAL, Origin::StalePush)
    );
    assert!(
        !replica
            .db
            .apply(&same_device_meta, LOCAL, Origin::StalePush)
    );
    assert!(
        !replica
            .db
            .apply(&same_device_stock, LOCAL, Origin::StalePush)
    );

    // Then: stale-push handling bypasses the echo check, not the version guard; meta rev/schema stay local.
    let expected = Db {
        meta: meta(91, 7, 15, 7),
        stock: BTreeMap::from([
            ("p".into(), stock(39, 2, 6)),
            ("p-new".into(), stock(4, 1, 0)),
        ]),
        ..base
    };
    assert_eq!(replica.db, expected);
    assert_eq!(
        replica.cursor,
        Cursor {
            seq: 88,
            pushed_m: 900
        }
    );
    saved.write("db.json", &replica.db);
    assert_eq!(saved.read::<Db>("db.json"), expected);
}
