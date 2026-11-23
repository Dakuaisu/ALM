use serde::{Deserialize, Serialize};

const PUSHED_M: u64 = 50_000;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct LogEntry {
    id: String,
    ts: u64,
    date: String,
    device: String,
    #[serde(rename = "type")]
    action: String,
    msg: String,
    _v: u32,
    _m: u64,
    _d: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct Database {
    log: Vec<LogEntry>,
    deleted: Vec<DeletedEntry>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct DeletedEntry {
    coll: String,
    id: String,
    rid: String,
    ts: u64,
    _v: u32,
    _m: u64,
    _d: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct PendingRecord {
    collection: String,
    id: String,
    json: serde_json::Value,
    v: u32,
    m: u64,
    d: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct PendingEnvelope {
    p_records: Vec<PendingRecord>,
}

fn entry(position: usize, modified: u64, timestamp: u64) -> LogEntry {
    LogEntry {
        id: format!("log-{position}"),
        ts: timestamp,
        date: format!("2024-01-{:02}", (position % 28) + 1),
        device: "synthetic-device".into(),
        action: "synthetic-action".into(),
        msg: format!("Details position {position}"),
        _v: 1,
        _m: modified,
        _d: "synthetic-device".into(),
    }
}

fn settings_details(log: &[LogEntry]) -> Vec<String> {
    log.iter()
        .rev()
        .take(200)
        .map(|row| row.msg.clone())
        .collect()
}

fn ids(log: &[LogEntry]) -> Vec<String> {
    log.iter().map(|row| row.id.clone()).collect()
}
fn has_id(log: &[LogEntry], id: &str) -> bool {
    log.iter().any(|row| row.id == id)
}

fn append(db: &mut Database, row: LogEntry) {
    db.log.push(row);
    if db.log.len() > 4_000 {
        db.log = db.log.split_off(db.log.len() - 3_000);
    }
}

fn changed(db: &Database, pushed_m: u64) -> PendingEnvelope {
    let mut p_records = db
        .log
        .iter()
        .filter(|row| !row.id.is_empty() && row._m > pushed_m)
        .map(|row| PendingRecord {
            collection: "log".into(),
            id: row.id.clone(),
            json: serde_json::to_value(row).unwrap(),
            v: row._v,
            m: row._m,
            d: row._d.clone(),
        })
        .collect::<Vec<_>>();
    p_records.extend(
        db.deleted
            .iter()
            .filter(|row| !row.id.is_empty() && row._m > pushed_m)
            .map(|row| PendingRecord {
                collection: "deleted".into(),
                id: row.id.clone(),
                json: serde_json::to_value(row).unwrap(),
                v: row._v,
                m: row._m,
                d: row._d.clone(),
            }),
    );
    PendingEnvelope { p_records }
}

fn database(log: Vec<LogEntry>) -> Database {
    Database {
        log,
        deleted: vec![],
    }
}

#[test]
fn l18_05_log_append_trims_by_position_and_keeps_all_stored_ids_in_order() {
    // Given: 3,999 distinct stored IDs, with the highest timestamp at position 1.
    let mut log = (1..=3_999)
        .map(|position| entry(position, 10_000 + position as u64, position as u64))
        .collect::<Vec<_>>();
    log[0].ts = 9_999_999;
    let mut db = database(log);

    // When: the 4,000th and then 4,001st activity entries are appended.
    append(&mut db, entry(4_000, 20_000, 4_000));
    let first_ids = ids(&db.log);
    append(&mut db, entry(4_001, 20_001, 4_001));

    // Then: 4,000 retains every first-append ID; 4,001 keeps positions 1,002..=4,001.
    assert_eq!(
        first_ids,
        (1..=4_000)
            .map(|position| format!("log-{position}"))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        ids(&db.log),
        (1_002..=4_001)
            .map(|position| format!("log-{position}"))
            .collect::<Vec<_>>()
    );
    println!(
        "L18-05 append trace: first={} [{}..{}]; after={} [{}..{}]",
        first_ids.len(),
        first_ids.first().unwrap(),
        first_ids.last().unwrap(),
        db.log.len(),
        db.log.first().unwrap().id,
        db.log.last().unwrap().id
    );
}

#[test]
fn l18_05_settings_projects_reverse_position_without_mutation_or_timestamp_sort() {
    // Given: independent 200- and 201-entry arrays, with inverted timestamps at the boundary.
    let rows_200 = (1..=200)
        .map(|position| entry(position, 30_000 + position as u64, position as u64))
        .collect::<Vec<_>>();
    let mut rows_201 = (1..=201)
        .map(|position| entry(position, 40_000 + position as u64, position as u64))
        .collect::<Vec<_>>();
    rows_201[0].ts = 9_999_999;
    rows_201[199].ts = 10;
    rows_201[200].ts = 9;
    let before_200 = rows_200.clone();
    let before_201 = rows_201.clone();

    // When: the Settings activity table projects the last 200 array positions in reverse.
    let details_200 = settings_details(&rows_200);
    let details_201 = settings_details(&rows_201);

    // Then: exact Details order follows positions, not timestamps, and both sources are unchanged.
    assert_eq!(
        details_200,
        (1..=200)
            .rev()
            .map(|position| format!("Details position {position}"))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        details_201,
        (2..=201)
            .rev()
            .map(|position| format!("Details position {position}"))
            .collect::<Vec<_>>()
    );
    assert_eq!(rows_200, before_200);
    assert_eq!(rows_201, before_201);
    println!(
        "L18-05 Settings: {} entries [200..1], {} entries [201..2]",
        details_200.len(),
        details_201.len()
    );
}

#[test]
fn l18_05_trim_does_not_create_tombstone_or_restore_trimmed_delta_record() {
    // Given: 4,000 records, empty deleted rows, and an external sync watermark.
    let mut rows = (1..=4_000)
        .map(|position| entry(position, 10_000 + position as u64, position as u64))
        .collect::<Vec<_>>();
    rows[3_800]._m = PUSHED_M + 1;
    rows[1_000]._m = PUSHED_M + 2;
    rows[3_999]._m = PUSHED_M;
    let mut db = database(rows);
    let before_backup: Database =
        serde_json::from_slice(&serde_json::to_vec(&db).unwrap()).unwrap();
    let before_outgoing: PendingEnvelope =
        serde_json::from_slice(&serde_json::to_vec(&changed(&db, PUSHED_M)).unwrap()).unwrap();

    // When: one new marked activity entry is appended, causing the positional trim.
    append(&mut db, entry(4_001, PUSHED_M + 3, 4_001));
    let after_details = settings_details(&db.log);
    let after_backup: Database = serde_json::from_slice(&serde_json::to_vec(&db).unwrap()).unwrap();
    let after_outgoing: PendingEnvelope =
        serde_json::from_slice(&serde_json::to_vec(&changed(&db, PUSHED_M)).unwrap()).unwrap();

    // Then: verify membership across the table, entire backup, delta, and explicit deleted list.
    assert_eq!(before_backup.log.len(), 4_000);
    assert!(before_backup.deleted.is_empty());
    assert_eq!(
        ids(&before_backup.log),
        (1..=4_000)
            .map(|position| format!("log-{position}"))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        before_outgoing
            .p_records
            .iter()
            .map(|row| (row.collection.as_str(), row.id.as_str()))
            .collect::<Vec<_>>(),
        [("log", "log-1001"), ("log", "log-3801")]
    );
    assert_eq!(
        ids(&after_backup.log),
        (1_002..=4_001)
            .map(|position| format!("log-{position}"))
            .collect::<Vec<_>>()
    );
    assert!(after_backup.deleted.is_empty());
    assert_eq!(
        after_outgoing
            .p_records
            .iter()
            .map(|row| (row.collection.as_str(), row.id.as_str()))
            .collect::<Vec<_>>(),
        [("log", "log-3801"), ("log", "log-4001")]
    );
    assert_eq!(
        after_details,
        (3_802..=4_001)
            .rev()
            .map(|position| format!("Details position {position}"))
            .collect::<Vec<_>>()
    );
    println!(
        "L18-05 payloads: backup before={}, after={}, after markers={:?}; unsent before={:?}, after={:?}; deleted={}",
        before_backup.log.len(),
        after_backup.log.len(),
        ["log-1001", "log-3801", "log-4000", "log-4001"]
            .map(|id| (id, has_id(&after_backup.log, id))),
        before_outgoing
            .p_records
            .iter()
            .map(|row| row.id.as_str())
            .collect::<Vec<_>>(),
        after_outgoing
            .p_records
            .iter()
            .map(|row| row.id.as_str())
            .collect::<Vec<_>>(),
        after_backup.deleted.len()
    );
}
