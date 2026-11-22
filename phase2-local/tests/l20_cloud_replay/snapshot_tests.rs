use super::client::{Live, Screen};
use super::rpc::{Op, Page, Rejection, Reply, Request, Snapshot};
use super::storage::{Completed, Writes};
use super::{
    BASE, Client, Db, Device, Event, ITEM, Key, NOW, Notice, Progress, Table, baseline, row,
};

const FRESH: Key = Key(Table::Products, 66);
const LOG: Key = Key(Table::Log, 1);

fn replacement() -> Db {
    Db::new(4, vec![row(FRESH, (900, 3, 9), Device::Peer)])
}

fn rebuild_snapshot(seq: Option<u64>) -> Reply {
    Reply::SnapshotRead(Some(Snapshot {
        json: serde_json::to_string(&replacement()).unwrap(),
        seq,
    }))
}

#[test]
fn l20_04_rebuild_saves_replacement_before_log_and_second_catchup_save() {
    let mut client = Client::new(baseline(), BASE, true);
    client.files.safety_writes = false;
    let catchup = row(ITEM, (707, 1, 18), Device::Peer);
    client.rpc.load([
        rebuild_snapshot(Some(7)),
        Reply::Pulled(Page {
            max_seq: 9,
            more: false,
            rows: vec![catchup.clone().into()],
        }),
    ]);
    client.rebuild(true);
    let mut first = replacement();
    first.rev = 5;
    let mut final_db = Db::new(6, vec![catchup]);
    final_db.rows.extend(first.rows.clone());
    final_db.rows.push(row(LOG, (2, 1, NOW), Device::Local));
    assert_eq!(
        client.rpc.calls,
        [
            Request::GetSnapshot,
            Request::Pull {
                since: 7,
                limit: 500
            }
        ]
    );
    assert_eq!(client.db(), &final_db);
    assert_eq!(client.files.copies(), (baseline(), final_db.clone()));
    assert_eq!(
        client.trace,
        [
            Event::Rpc(Op::GetSnapshot),
            Event::SafetyStarted(40),
            Event::Progress(
                Progress {
                    pushed: NOW,
                    seq: 7,
                    ..BASE
                },
                true
            ),
            Event::Save {
                rev: 5,
                mirror: true
            },
            Event::Audit(2),
            Event::Rpc(Op::Pull),
            Event::Progress(
                Progress {
                    pushed: NOW,
                    seq: 9,
                    ..BASE
                },
                true
            ),
            Event::Save {
                rev: 6,
                mirror: true
            },
            Event::Redraw(Screen::Products),
            Event::Redraw(Screen::Dashboard),
            Event::Notice(Notice::Rebuilt),
        ]
    );
    client.settle();
    assert_eq!(
        client.files.progress(),
        Progress {
            pushed: NOW,
            seq: 9,
            ..BASE
        }
    );
    assert!(client.files.safety().is_none());
    assert_eq!(client.files.copies(), (final_db.clone(), final_db));
    assert!(client.trace.ends_with(&[
        Event::Complete(Completed::Safety {
            rev: 40,
            written: false
        }),
        Event::Complete(Completed::Primary {
            rev: 5,
            written: true
        }),
        Event::Complete(Completed::Primary {
            rev: 6,
            written: true
        }),
    ]));
    client.rpc.assert_complete();
    println!(
        "L20-04 rebuild and catch-up after failed safety copy: {:?}",
        client.trace
    );
}

#[test]
fn l20_04_rebuild_success_notice_does_not_prove_durable_replacement() {
    for (primary, mirror) in [(false, false), (true, false), (false, true), (true, true)] {
        let mut client = Client::new(baseline(), BASE, true);
        client
            .files
            .save_policy
            .push_back(Writes { primary, mirror });
        client.rpc.load([
            rebuild_snapshot(None),
            Reply::Rejected(Op::Pull, Rejection::Ordinary),
        ]);
        client.rebuild(true);
        assert_eq!(
            client.rpc.calls,
            [
                Request::GetSnapshot,
                Request::Pull {
                    since: 0,
                    limit: 500
                }
            ]
        );
        assert!(client.files.safety().is_none());
        assert_eq!(client.trace.last(), Some(&Event::Notice(Notice::Rebuilt)));
        assert_eq!(client.err, Some(Rejection::Ordinary));
        assert_eq!(
            client.files.progress(),
            Progress {
                pushed: NOW,
                seq: 0,
                ..BASE
            }
        );
        assert_eq!(client.db().rev, 5);
        assert!(client.db().row(LOG).is_some());
        let mut stored = replacement();
        stored.rev = 5;
        client.settle();
        assert_eq!(client.files.safety(), Some(baseline()));
        assert_eq!(
            client.files.copies(),
            (
                if primary { stored.clone() } else { baseline() },
                if mirror { stored.clone() } else { baseline() }
            )
        );
        client.reload();
        let mut expected = if primary && mirror {
            stored
        } else {
            baseline()
        };
        if !primary && mirror {
            expected.rev = 41;
        }
        assert_eq!(client.db(), &expected);
        client.settle();
        client.rpc.assert_complete();
        println!(
            "L20-04 first rebuild save primary={primary} mirror={mirror}; reload rev={}; {:?}",
            client.db().rev,
            client.trace
        );
    }
}

#[test]
fn l20_04_missing_parse_and_migration_errors_have_distinct_rebuild_boundaries() {
    for (response, notice, tried_safety) in [
        (Reply::SnapshotRead(None), Notice::NoSnapshot, false),
        (
            Reply::SnapshotRead(Some(Snapshot {
                json: String::new(),
                seq: Some(5),
            })),
            Notice::NoSnapshot,
            false,
        ),
        (
            Reply::SnapshotRead(Some(Snapshot {
                json: "not JSON".into(),
                seq: Some(5),
            })),
            Notice::MalformedSnapshot,
            true,
        ),
        (
            Reply::SnapshotRead(Some(Snapshot {
                json: r#"{"rows":false}"#.into(),
                seq: Some(5),
            })),
            Notice::MigrationFailed,
            true,
        ),
        (
            Reply::Rejected(Op::GetSnapshot, Rejection::Ordinary),
            Notice::RebuildRpcFailed(Rejection::Ordinary),
            false,
        ),
    ] {
        let mut client = Client::new(baseline(), BASE, true);
        client.rebuild(false);
        assert!(client.trace.is_empty());
        client.rpc.load([response]);
        client.rebuild(true);
        assert_eq!(client.rpc.calls, [Request::GetSnapshot]);
        let mut trace = vec![Event::Rpc(Op::GetSnapshot)];
        if tried_safety {
            trace.push(Event::SafetyStarted(40));
        }
        trace.push(Event::Notice(notice));
        assert_eq!(client.trace, trace);
        match &client.live {
            Live::Ready(db) => {
                assert_ne!(notice, Notice::MigrationFailed);
                assert_eq!(db, &baseline());
            }
            Live::Assigned(raw) => {
                assert_eq!(notice, Notice::MigrationFailed);
                assert_eq!(raw, &serde_json::json!({"rows": false}));
            }
        }
        assert_eq!(client.files.progress(), BASE);
        assert_eq!(client.files.copies(), (baseline(), baseline()));
        client.settle();
        assert_eq!(client.files.safety(), tried_safety.then(baseline));
        client.rpc.assert_complete();
        println!("L20-04 invalid snapshot {notice:?}: {:?}", client.trace);
    }
}
