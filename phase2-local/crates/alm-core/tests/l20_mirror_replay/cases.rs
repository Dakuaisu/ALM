use serde_json::{Value, json};

use super::fixture::{NOON_MS, TODAY, date, seed};
use super::refresh::{
    Reply::{Rejected, Resolved},
    Round,
    Rpc::{Mirror, Pull, Push, Snapshot},
    Save,
};
use super::store::{State, Store};
use super::summary::{cutoff, tabs};

fn expected_tabs() -> Value {
    json!({
        "Day Summary": {
            "headers": ["Date", "Bills", "Sales ₹", "Collected ₹"],
            "rows": [["2025-04-17", 0, 0, 0], ["2025-03-03", 0, 0, 2], ["2025-03-02", 1, 125, 120], ["2025-03-01", 2, 251, 49]],
        },
        "Sales": {
            "headers": ["Date", "Bill", "Customer", "Amount ₹", "Paid ₹", "Mode", "Status"],
            "rows": [
                ["2025-04-16", 7, "Zero", 125, 3, "Cash", "CANCELLED"],
                ["2025-03-02", 4, "Buyer", 125, 120, "Online", ""],
                ["2025-03-01", 6, "Zero", 125, 3, "Cash", "CANCELLED"],
                ["2025-03-01", 3, "Buyer", 125, 24, "Online", ""],
                ["2025-03-01", 2, "Buyer", 125, 24, "Cash", ""],
            ],
        },
        "Udhaar": {
            "headers": ["Customer", "Mobile", "Balance ₹"],
            "rows": [["Buyer", "0000000001", 252], ["Opening debtor", "0000000002", 122], ["Advance", "", -1]],
        },
        "Stock": {
            "headers": ["Item", "On Hand (pkt)", "FIFO Value ₹"],
            "rows": [["Zinc Soda", "4", 19], ["Apple Soda", "0", 0]],
        },
        "Payments": {
            "headers": ["Date", "Customer", "Amount ₹", "Mode"],
            "rows": [["2025-04-17", "Rounded to zero", 0, "Cash"], ["2025-03-03", "Advance", 2, "Online"], ["2025-03-01", "Buyer", 0, "Cash"]],
        },
    })
}

#[test]
fn l20_15_projects_five_tabs_with_distinct_cutoff_and_rounding_rules() {
    let fixture = seed();
    assert_eq!(cutoff(TODAY), date("2025-03-01"));
    assert_eq!(cutoff(date("2024-04-14")), date("2024-02-29"));
    assert_eq!(cutoff(date("2025-01-10")), date("2024-11-26"));
    let actual = tabs(&fixture, TODAY);
    assert_eq!(actual, expected_tabs());
    let shifted = tabs(&fixture, date("2025-04-16"));
    assert_eq!(
        shifted["Day Summary"]["rows"],
        json!([
            ["2025-04-17", 0, 0, 0],
            ["2025-03-03", 0, 0, 2],
            ["2025-03-02", 1, 125, 120]
        ])
    );
    assert_eq!(
        shifted["Payments"]["rows"],
        json!([
            ["2025-04-17", "Rounded to zero", 0, "Cash"],
            ["2025-03-03", "Advance", 2, "Online"]
        ])
    );
    assert_eq!(shifted["Sales"]["rows"].as_array().unwrap().len(), 2);
    assert_eq!(shifted["Udhaar"], actual["Udhaar"]);
    assert_eq!(shifted["Stock"], actual["Stock"]);
}

#[test]
fn l20_15_only_dispatches_strictly_above_fifteen_minutes() {
    let fixture = seed();
    let start = State {
        mirror_at: NOON_MS - 900_000,
        snap_at: NOON_MS,
        seq: 42,
        pushed_m: 301,
    };
    let store = Store::seed(start);
    for now in [NOON_MS - 1, NOON_MS] {
        let mut round = Round::new(now, &[(Pull, 0, Resolved)]);
        round.run(&store, &fixture, true);
        assert_eq!(round.calls.len(), 1);
        assert_eq!(
            round.writes.iter().map(|w| w.stage).collect::<Vec<_>>(),
            [Save::Pull]
        );
        assert_eq!(store.read(), start);
        assert_eq!(round.toast, None);
    }
    let mut round = Round::new(NOON_MS, &[(Pull, 1, Resolved), (Mirror, 37, Resolved)]);
    round.run(&store, &fixture, false);
    let call = &round.calls[1];
    assert_eq!(call.name, "mirror_write");
    assert_eq!(
        (call.dispatched, call.completed),
        (NOON_MS + 1, NOON_MS + 38)
    );
    assert_eq!(
        call.body,
        json!({"p_token": "synthetic-mirror-token",
        "p_tabs": {"tabs": expected_tabs(), "updated": NOON_MS + 1}})
    );
    let attempted = &round.writes[1];
    assert_eq!(
        (
            attempted.stage,
            attempted.attempted.mirror_at,
            attempted.saved
        ),
        (Save::Mirror, NOON_MS + 38, true)
    );
    assert_eq!(
        store.read(),
        State {
            mirror_at: NOON_MS + 38,
            ..start
        }
    );
    assert_eq!(round.toast, Some("Cloud sync ✓"));

    let mut from_completion = Round::new(NOON_MS + 900_038, &[(Pull, 0, Resolved)]);
    from_completion.run(&store, &fixture, true);
    assert_eq!(store.read().mirror_at, NOON_MS + 38);
}

#[test]
fn l20_15_rejected_mirror_does_not_undo_sync_or_skip_snapshot_and_can_retry() {
    let fixture = seed();
    let start = State {
        seq: 41,
        pushed_m: 300,
        mirror_at: NOON_MS - 900_001,
        snap_at: NOON_MS - 86_400_000,
    };
    let store = Store::seed(start);
    let mut round = Round::new(
        NOON_MS,
        &[
            (Push, 0, Resolved),
            (Pull, 0, Resolved),
            (Mirror, 19, Rejected),
            (Snapshot, 11, Resolved),
        ],
    );
    round.run(&store, &fixture, false);
    assert_eq!(
        round.calls.iter().map(|call| call.name).collect::<Vec<_>>(),
        ["sync_push", "sync_pull", "mirror_write", "put_snapshot"]
    );
    assert_eq!(round.toast, Some("Cloud sync ✓"));
    assert_eq!(round.calls[2].reply, Rejected);
    assert_eq!(
        round.writes.iter().map(|w| w.stage).collect::<Vec<_>>(),
        [Save::Push, Save::Pull, Save::Snapshot]
    );
    assert_eq!(
        store.read(),
        State {
            seq: 42,
            pushed_m: 301,
            snap_at: NOON_MS + 30,
            ..start
        }
    );

    let push = &round.calls[0].body;
    assert_eq!(push["p_token"], "synthetic-mirror-token");
    assert_eq!(push["p_device"], "synthetic-mirror-device");
    assert_eq!(push.as_object().unwrap().len(), 3);
    let [record] = push["p_records"].as_array().unwrap().as_slice() else {
        panic!("one changed record")
    };
    assert_eq!(
        record,
        &json!({"collection":"sales","id":"sale-1","v":4,"m":301,"d":"synthetic-mirror-device",
        "json":{"id":"sale-1","date":"2025-02-28","invoice":1,"totalMinor":12549,"paidMinor":4949,
            "cancelled":false,"paymentMode":"Cash","_v":4,"_m":301,"_d":"synthetic-mirror-device"}})
    );
    assert_eq!(round.calls[1].body["p_since"], 41);
    assert_eq!(
        round.calls[2].body,
        json!({"p_token":"synthetic-mirror-token",
        "p_tabs":{"tabs":expected_tabs(),"updated":NOON_MS}})
    );
    let snapshot = &round.calls[3];
    assert_eq!(
        (snapshot.dispatched, snapshot.completed),
        (NOON_MS + 19, NOON_MS + 30)
    );
    assert_eq!(snapshot.body["p_seq"], 42);
    assert_eq!(snapshot.body["p_token"], "synthetic-mirror-token");
    assert_eq!(snapshot.body.as_object().unwrap().len(), 3);
    let db: Value = serde_json::from_str(snapshot.body["p_json"].as_str().unwrap()).unwrap();
    assert_eq!(db["sales"].as_array().unwrap().len(), 7);
    assert!(db["sales"].as_array().unwrap().contains(&record["json"]));
    assert!(
        db["payments"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["amountMinor"] == 3_051)
    );
    assert_eq!(db["meta"]["fixture"], "L20-15");
    assert!(db.get("tabs").is_none());
    assert_eq!(db["meta"].get("lastPushAt"), None);

    let mut sheet_db = db.clone();
    sheet_db["meta"]["lastPushDevice"] = json!("synthetic-mirror-device");
    sheet_db["meta"]["lastPushAt"] = json!(NOON_MS);
    let sheets_post_body = json!({"action": "push", "db": sheet_db});
    assert_eq!(sheets_post_body["db"]["sales"], db["sales"]);
    assert_ne!(sheets_post_body["db"]["meta"], db["meta"]);
    for hub_field in ["p_token", "p_tabs", "p_seq", "p_records"] {
        assert!(sheets_post_body.get(hub_field).is_none());
    }

    let mut retry = Round::new(NOON_MS + 31, &[(Pull, 0, Resolved), (Mirror, 6, Resolved)]);
    retry.run(&store, &fixture, true);
    assert_eq!(retry.calls[0].body["p_since"], 42);
    assert_eq!(retry.calls[1].body["p_tabs"]["updated"], NOON_MS + 31);
    assert_eq!(store.read().mirror_at, NOON_MS + 37);
}
