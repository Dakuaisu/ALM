use super::model::{
    Collection, Db, DeleteTick, Envelope, GstBill, Origin, Record, Row, Stamp, Supplier, Tick,
    Tombstone,
};
use super::saved::Saved;

const LOCAL: &str = "synthetic-local";
const REMOTE: &str = "synthetic-remote";
const GST: Collection = Collection::GstBills;
const SHARED: &str = "shared-id";

fn bill(id: &str, total: i64, stamp: Stamp) -> Row<GstBill> {
    Row::new(id, GstBill { total }, stamp)
}

fn marker(id: &str, ts: u64, stamp: Stamp) -> Row<Tombstone> {
    Row::new(
        &format!("gstBills:{id}"),
        Tombstone {
            coll: GST,
            rid: id.into(),
            ts,
        },
        stamp,
    )
}

const fn tick(ts: u64, m: u64, d: &str) -> DeleteTick<'_> {
    DeleteTick {
        ts,
        write: Tick { m, d },
    }
}

#[test]
fn l20_11_refreshes_one_marker_and_replays_versions_and_removal_independently() {
    // Given: PHASE1_CASES L20-11 / HTML 961-998, 9472-9485, 9532-9548.
    let original = bill(SHARED, 600, Stamp::new(4, 7, REMOTE));
    let mut sender = Db {
        gst_bills: vec![original.clone()],
        ..Db::default()
    };
    let first = marker(SHARED, 19, Stamp::new(1, 20, LOCAL));
    let second = marker(SHARED, 29, Stamp::new(2, 30, REMOTE));

    // When: the same helper is called twice, the second time on an absent target.
    sender.remove(GST, SHARED, tick(19, 20, LOCAL));
    assert_eq!(sender.deleted, std::slice::from_ref(&first));
    assert!(sender.gst_bills.is_empty());
    sender.remove(GST, SHARED, tick(29, 30, REMOTE));

    // Then: the marker keeps its slot and uses an independent version and clock.
    let sent = Envelope::from_row(&second, Record::Deleted(second.clone()), REMOTE);
    assert_eq!(sender.deleted, std::slice::from_ref(&second));
    assert_eq!(sender.changed(29, REMOTE), std::slice::from_ref(&sent));
    assert!(sender.changed(30, REMOTE).is_empty());
    let mut receiver = Db {
        deleted: vec![second.clone()],
        ..Db::default()
    };
    let older = Envelope::from_row(&first, Record::Deleted(first.clone()), LOCAL);
    let newer = marker(SHARED, 8, Stamp::new(3, 9, REMOTE));
    let newest = Envelope::from_row(&newer, Record::Deleted(newer.clone()), REMOTE);
    assert!(!receiver.apply(&sent, LOCAL, Origin::Pull));
    assert_eq!(receiver.deleted, std::slice::from_ref(&second));
    assert!(receiver.apply(&newest, LOCAL, Origin::Pull));
    assert_eq!(receiver.deleted, std::slice::from_ref(&newer));
    let older = Envelope {
        d: REMOTE.into(),
        ..older
    };
    assert!(!receiver.apply(&older, LOCAL, Origin::Pull));
    receiver.gst_bills.push(original);
    assert!(
        receiver.apply(&older, LOCAL, Origin::Pull),
        "the older marker still removes the target"
    );
    let expected = Db {
        deleted: vec![newer],
        ..Db::default()
    };
    let saved = Saved::new();
    saved.write("sender.json", &sender);
    saved.write("receiver.json", &receiver);
    assert_eq!(saved.read::<Db>("sender.json"), sender);
    assert_eq!(receiver, expected);
    assert_eq!(saved.read::<Db>("receiver.json"), expected);
}

#[test]
fn l20_12_update_then_delete_and_delete_then_update_reach_same_scoped_state() {
    // Given: PHASE1_CASES L20-12 / HTML 990-992, 9508-9562.
    let other = Row::new(
        SHARED,
        Supplier {
            name: "Synthetic survivor".into(),
        },
        Stamp::new(5, 6, LOCAL),
    );
    let base = Db {
        gst_bills: vec![bill(SHARED, 123, Stamp::new(1, 7, LOCAL))],
        suppliers: vec![other.clone()],
        ..Db::default()
    };
    let row = bill(SHARED, 456, Stamp::new(80, 50, REMOTE));
    let deleted = marker(SHARED, 48, Stamp::new(1, 49, REMOTE));
    let update = Envelope::from_row(&row, Record::GstBills(row.clone()), REMOTE);
    let delete = Envelope::from_row(&deleted, Record::Deleted(deleted.clone()), REMOTE);

    // When: the same inputs arrive in opposite order on independent copies.
    let mut update_first = base.clone();
    assert!(update_first.apply(&update, LOCAL, Origin::Pull));
    assert_eq!(update_first.gst_bills, [row]);
    assert!(update_first.apply(&delete, LOCAL, Origin::Pull));
    let mut delete_first = base;
    assert!(delete_first.apply(&delete, LOCAL, Origin::Pull));
    assert!(!delete_first.apply(&update, LOCAL, Origin::Pull));

    // Then: the marker wins regardless of the bill version, and the supplier survives.
    let expected = Db {
        suppliers: vec![other],
        deleted: vec![deleted],
        ..Db::default()
    };
    let saved = Saved::new();
    for (file, actual) in [
        ("update-first.json", update_first),
        ("delete-first.json", delete_first),
    ] {
        assert_eq!(actual, expected);
        saved.write(file, &actual);
        assert_eq!(saved.read::<Db>(file), expected);
    }
}

#[test]
fn l20_13_local_trim_uses_array_order_and_inbound_append_does_not_trim() {
    // Given: PHASE1_CASES L20-13 / HTML 974-998, 9532-9562.
    let mut local = Db {
        deleted: (1..=3_999)
            .map(|n| marker(&format!("bill-{n}"), n, Stamp::new(1, n, LOCAL)))
            .collect(),
        ..Db::default()
    };
    let saved = Saved::new();

    // When: a local marker reaches the cap, an early slot is refreshed, and a new ID exceeds it.
    local.remove(GST, "bill-4000", tick(4_000, 4_000, LOCAL));
    assert_eq!(local.deleted.len(), 4_000);
    local.remove(GST, "bill-2", tick(9_000, 9_000, LOCAL));
    assert_eq!(
        local.deleted[1],
        marker("bill-2", 9_000, Stamp::new(2, 9_000, LOCAL))
    );
    saved.write("at-cap.json", &local);
    let mut inbound = saved.read::<Db>("at-cap.json");
    assert_eq!(inbound, local);
    local.remove(GST, "bill-4001", tick(10_000, 10_000, LOCAL));
    let added = marker("bill-4001", 10_000, Stamp::new(1, 10_000, REMOTE));
    let wire = Envelope::from_row(&added, Record::Deleted(added.clone()), REMOTE);
    assert!(inbound.apply(&wire, LOCAL, Origin::Pull));

    // Then: an otherwise eligible update is blocked only if its marker survived on that copy.
    let retained_ids = (1_002..=4_001)
        .map(|n| format!("bill-{n}"))
        .collect::<Vec<_>>();
    assert_eq!(
        local
            .deleted
            .iter()
            .map(|row| row.value.data.rid.as_str())
            .collect::<Vec<_>>(),
        retained_ids
    );
    assert_eq!(inbound.deleted.len(), 4_001);
    assert_eq!(inbound.deleted.last(), Some(&added));
    assert_eq!(inbound.deleted[1].value.stamp, Stamp::new(2, 9_000, LOCAL));
    assert_eq!(
        local
            .changed(4_000, LOCAL)
            .iter()
            .map(|row| row.id.as_str())
            .collect::<Vec<_>>(),
        ["gstBills:bill-4001"]
    );
    for id in ["bill-1002", "bill-2"] {
        let row = bill(id, 900, Stamp::new(90, 11_000, REMOTE));
        let update = Envelope::from_row(&row, Record::GstBills(row.clone()), REMOTE);
        assert_eq!(local.apply(&update, LOCAL, Origin::Pull), id == "bill-2");
        assert!(!inbound.apply(&update, LOCAL, Origin::Pull));
    }
    assert_eq!(
        local.gst_bills,
        [bill("bill-2", 900, Stamp::new(90, 11_000, REMOTE))]
    );
    assert!(inbound.gst_bills.is_empty());
    for (file, actual) in [("local-trim.json", local), ("inbound-append.json", inbound)] {
        saved.write(file, &actual);
        assert_eq!(saved.read::<Db>(file), actual);
    }
}
