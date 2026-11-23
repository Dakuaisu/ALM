use super::rpc::{Op, Page, Rejection, Reply, Request};
use super::{
    BASE, Client, Db, Device, Event, ITEM, Key, NOW, Notice, Progress, Row, Table, baseline, row,
};

#[test]
fn l20_04_first_upload_preserves_pull_cursor_and_saves_even_if_explicit_snapshot_rejects() {
    let unstamped = [
        Key(Table::Products, 33),
        Key(Table::Stock, 33),
        Key(Table::Meta, 0),
    ];
    let mut before = baseline();
    before
        .rows
        .extend(unstamped.map(|key| Row::new(key, 75, None)));
    for snapshot_ok in [true, false] {
        let mut client = Client::new(before.clone(), BASE, true);
        let reply = if snapshot_ok {
            Reply::SnapshotWritten(NOW + 20)
        } else {
            Reply::Rejected(Op::PutSnapshot, Rejection::Ordinary)
        };
        client.rpc.load([
            Reply::Pushed(vec![]),
            Reply::Pulled(Page::empty(72, false)),
            reply,
        ]);
        client.first_upload();
        let [
            Request::Push(batch),
            Request::Pull {
                since: 70,
                limit: 500,
            },
            Request::PutSnapshot { seq: 72, json },
        ] = client.rpc.calls.as_slice()
        else {
            panic!("unexpected first upload requests")
        };
        assert!(
            batch
                .iter()
                .map(|entry| (entry.key, entry.sent_time(), entry.device))
                .eq([
                    (unstamped[0], 1, Device::Legacy),
                    (unstamped[1], 1, Device::Legacy),
                    (unstamped[2], 1, Device::Legacy),
                    (ITEM, 5, Device::Local),
                ])
        );
        let mut stamped = baseline();
        stamped
            .rows
            .extend(unstamped.map(|key| row(key, (75, 1, 1), Device::Legacy)));
        assert_eq!(serde_json::from_str::<Db>(json).unwrap(), stamped);
        let mut saved = stamped;
        saved.rev = 41;
        saved.rows.push(row(
            Key(Table::Log, 1),
            (1, 1, if snapshot_ok { NOW + 20 } else { NOW }),
            Device::Local,
        ));
        assert_eq!(
            client.trace.first(),
            Some(&Event::Progress(Progress { pushed: 0, ..BASE }, true))
        );
        assert!(client.trace.ends_with(&[
            Event::Audit(1),
            Event::Save {
                rev: 41,
                mirror: true
            },
            Event::CloudKick,
            Event::Notice(Notice::Uploaded)
        ]));
        assert_eq!(
            client.files.progress(),
            Progress {
                pushed: 5,
                seq: 72,
                snap_at: if snapshot_ok { NOW + 20 } else { BASE.snap_at },
                ..BASE
            }
        );
        assert!(client.files.safety().is_none());
        assert_eq!(client.files.copies(), (before.clone(), saved.clone()));
        client.settle();
        assert_eq!(client.files.copies(), (saved.clone(), saved));
        client.rpc.assert_complete();
        println!(
            "L20-04 first upload snapshot accepted={snapshot_ok}: {:?}",
            client.trace
        );
    }
}

#[test]
fn l20_04_daily_snapshot_is_strictly_over_interval_and_rechecks_dirty_after_pull() {
    for (overdue, dirty, requested) in [
        (false, false, false),
        (true, true, false),
        (true, false, true),
    ] {
        let initial = Progress {
            snap_at: NOW - 86_400_000 - u64::from(overdue),
            ..BASE
        };
        let mut client = Client::new(baseline(), initial, true);
        let new = row(Key(Table::Customers, 77), (8, 1, 11), Device::Peer);
        let mut script = vec![Reply::Pulled(Page {
            max_seq: 73,
            more: false,
            rows: if dirty {
                vec![new.clone().into()]
            } else {
                vec![]
            },
        })];
        if requested {
            script.push(Reply::SnapshotWritten(NOW + 12));
        }
        client.rpc.load(script);
        client.sync(false);
        let mut calls = vec![Request::Pull {
            since: 70,
            limit: 500,
        }];
        if requested {
            calls.push(Request::PutSnapshot {
                seq: 73,
                json: serde_json::to_string(&baseline()).unwrap(),
            });
        }
        assert_eq!(client.rpc.calls, calls);
        assert_eq!(
            client.files.progress(),
            Progress {
                seq: 73,
                snap_at: if requested { NOW + 12 } else { initial.snap_at },
                ..initial
            }
        );
        assert_eq!(client.db().row(new.key), dirty.then_some(&new));
        assert_eq!(client.trace.last(), Some(&Event::Notice(Notice::Synced)));
        client.rpc.assert_complete();
        println!(
            "L20-04 daily snapshot overdue={overdue} pull inserted dirty={dirty}: {:?}",
            client.trace
        );
    }
}
