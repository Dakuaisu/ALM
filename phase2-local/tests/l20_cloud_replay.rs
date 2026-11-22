#[path = "l20_cloud_replay/auth_tests.rs"]
mod auth_tests;
#[path = "l20_cloud_replay/client.rs"]
mod client;
#[path = "l20_cloud_replay/interruption_tests.rs"]
mod interruption_tests;
#[path = "l20_cloud_replay/merge_tests.rs"]
mod merge_tests;
#[path = "l20_cloud_replay/pull_tests.rs"]
mod pull_tests;
#[path = "l20_cloud_replay/records.rs"]
mod records;
#[path = "l20_cloud_replay/rpc.rs"]
mod rpc;
#[path = "l20_cloud_replay/snapshot.rs"]
mod snapshot;
#[path = "l20_cloud_replay/snapshot_tests.rs"]
mod snapshot_tests;
#[path = "l20_cloud_replay/storage.rs"]
mod storage;
#[path = "l20_cloud_replay/upload_tests.rs"]
mod upload_tests;

use client::{Client, Event, NOW, Notice};
use records::{Db, Device, Key, Row, Stamp, Table};
use storage::{Completed, Progress};

const BASE: Progress = Progress {
    pushed: 10,
    seq: 70,
    mirror_at: NOW - 50,
    snap_at: NOW - 100,
};
const ITEM: Key = Key(Table::Products, 1);

fn row(key: Key, (value, version, time): (i64, u64, u64), device: Device) -> Row {
    Row::new(
        key,
        value,
        Some(Stamp {
            version,
            time,
            device,
        }),
    )
}

fn baseline() -> Db {
    Db::new(40, vec![row(ITEM, (100, 2, 5), Device::Local)])
}

#[test]
fn l20_01_disabled_save_stays_local() {
    for missing in [None, Some(0), Some(1), Some(2)] {
        let mut client = Client::new(baseline(), BASE, missing.is_some());
        if let Some(field) = missing {
            client.present[field] = false;
        }
        let saved = row(ITEM, (200, 3, 90), Device::Local);
        client.db_mut().rows[0] = saved.clone();
        client.persist();
        client.sync(false);
        client.sheets_without_url();
        let expected = Db::new(41, vec![saved]);
        assert_eq!(client.files.copies(), (baseline(), expected.clone()));
        client.settle();
        assert_eq!(client.files.copies(), (expected.clone(), expected));
        assert_eq!(client.files.progress(), BASE);
        client.rpc.assert_complete();
        assert!(client.rpc.calls.is_empty());
        assert_eq!(
            client.trace,
            [
                Event::Save {
                    rev: 41,
                    mirror: true
                },
                Event::Notice(Notice::NoSheetsUrl),
                Event::Complete(Completed::Primary {
                    rev: 41,
                    written: true
                }),
            ]
        );
        println!(
            "L20-01 missing prerequisite {missing:?}: {:?}",
            client.trace
        );
    }
}
