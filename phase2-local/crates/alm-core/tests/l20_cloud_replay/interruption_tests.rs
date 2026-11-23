use super::rpc::{Op, Page, Rejection, Reply, Request};
use super::{BASE, Client, Db, Device, Event, ITEM, Key, Notice, Progress, Table, baseline, row};

const NEW: Key = Key(Table::Products, 202);

fn outgoing() -> Db {
    Db::new(
        40,
        (1..=101)
            .rev()
            .map(|id| row(Key(Table::Products, id), (100, 2, id + 10), Device::Local))
            .collect(),
    )
}

fn winner() -> super::records::Row {
    row(ITEM, (700, 9, 777), Device::Local)
}

#[test]
fn l20_02_success_persists_pull_merge_and_counts_only_sent_timestamps() {
    let mut client = Client::new(outgoing(), BASE, true);
    let fresh = row(NEW, (303, 1, 120), Device::Peer);
    client.rpc.load([
        Reply::Pushed(vec![winner().into()]),
        Reply::Pushed(vec![]),
        Reply::Pulled(Page {
            max_seq: 80,
            more: true,
            rows: vec![
                row(ITEM, (999, 20, 140), Device::Local).into(),
                fresh.clone().into(),
            ],
        }),
        Reply::Pulled(Page {
            max_seq: 83,
            more: false,
            rows: vec![
                row(ITEM, (706, 9, 150), Device::Peer).into(),
                row(ITEM, (707, 8, 151), Device::Peer).into(),
            ],
        }),
    ]);
    client.sync(false);
    let [
        Request::Push(first),
        Request::Push(second),
        Request::Pull {
            since: 70,
            limit: 500,
        },
        Request::Pull {
            since: 80,
            limit: 500,
        },
    ] = client.rpc.calls.as_slice()
    else {
        panic!("unexpected request sequence")
    };
    assert!(
        first
            .iter()
            .map(|r| (r.key.1, r.sent_time()))
            .eq((1..=100).map(|id| (id, id + 10)))
    );
    assert!(
        second
            .iter()
            .map(|r| (r.key.1, r.sent_time()))
            .eq([(101, 111)])
    );
    let expected = Progress {
        pushed: 111,
        seq: 83,
        ..BASE
    };
    assert_eq!(client.files.progress(), expected);
    assert_eq!(client.db().row(ITEM), Some(&winner()));
    assert_eq!(client.db().row(NEW), Some(&fresh));
    let (primary, mirror) = client.files.copies();
    assert_eq!(primary, outgoing());
    assert_eq!(
        (mirror.rev, mirror.row(ITEM), mirror.row(NEW)),
        (41, Some(&winner()), Some(&fresh))
    );
    client.settle();
    assert_eq!(client.files.copies(), (mirror.clone(), mirror));
    assert!(client.trace.contains(&Event::Notice(Notice::Synced)));
    client.rpc.assert_complete();
    println!(
        "L20-02 100+1, stale winner, echo and older/equal ignored: {:?}",
        client.trace
    );
}

#[test]
fn l20_02_later_push_rejection_keeps_progress_and_both_files_at_baseline() {
    let before = outgoing();
    let mut client = Client::new(before.clone(), BASE, true);
    client.rpc.load([
        Reply::Pushed(vec![winner().into()]),
        Reply::Rejected(Op::Push, Rejection::Ordinary),
    ]);
    client.sync(false);
    client.settle();
    assert_eq!(client.db().row(ITEM), Some(&winner()));
    assert_eq!(client.files.progress(), BASE);
    assert_eq!(client.files.copies(), (before.clone(), before));
    assert_eq!(
        client.trace,
        [
            Event::Rpc(Op::Push),
            Event::Rpc(Op::Push),
            Event::Notice(Notice::Retrying(Rejection::Ordinary))
        ]
    );
    println!(
        "L20-02 push interruption: {:?}; progress={:?}; RAM winner only",
        client.trace,
        client.files.progress()
    );

    client.trace.clear();
    client.rpc.load([
        Reply::Pushed(vec![]),
        Reply::Pushed(vec![]),
        Reply::Pulled(Page::empty(71, false)),
    ]);
    client.sync(false);
    let [
        Request::Push(retried),
        Request::Push(tail),
        Request::Pull {
            since: 70,
            limit: 500,
        },
    ] = client.rpc.calls.as_slice()
    else {
        panic!("unexpected retry")
    };
    assert!(retried.iter().map(|entry| entry.key.1).eq(2..=101));
    assert!(
        tail.iter()
            .map(|entry| (entry.key, entry.sent_time()))
            .eq([(ITEM, 777)])
    );
    assert_eq!(
        client.files.progress(),
        Progress {
            pushed: 777,
            seq: 71,
            ..BASE
        }
    );
    assert_eq!(client.files.copies(), (outgoing(), outgoing()));
    assert_eq!(client.db().row(ITEM), Some(&winner()));
    assert!(
        !client
            .trace
            .iter()
            .any(|event| matches!(event, Event::Save { .. }))
    );
    client.rpc.assert_complete();
    println!(
        "L20-02 retry reattempts prior batch peers; no pull application means no save: {:?}",
        client.trace
    );
}

#[test]
fn l20_02_later_pull_rejection_keeps_cursor_but_not_database_across_reload() {
    let fresh = row(NEW, (303, 1, 6), Device::Peer);
    for interrupted in [true, false] {
        let mut client = Client::new(baseline(), BASE, true);
        let last = if interrupted {
            Reply::Rejected(Op::Pull, Rejection::Ordinary)
        } else {
            Reply::Pulled(Page::empty(80, false))
        };
        client.rpc.load([
            Reply::Pulled(Page {
                max_seq: 80,
                more: true,
                rows: vec![fresh.clone().into()],
            }),
            last,
        ]);
        client.sync(false);
        assert_eq!(
            client.rpc.calls,
            [
                Request::Pull {
                    since: 70,
                    limit: 500
                },
                Request::Pull {
                    since: 80,
                    limit: 500
                }
            ]
        );
        assert_eq!(client.db().row(NEW), Some(&fresh));
        let progress = Progress { seq: 80, ..BASE };
        assert_eq!(client.files.progress(), progress);
        client.settle();
        let expected = if interrupted {
            baseline()
        } else {
            Db::new(41, vec![fresh.clone(), baseline().rows.remove(0)])
        };
        assert_eq!(client.files.copies(), (expected.clone(), expected.clone()));
        client.reload();
        assert_eq!(client.db(), &expected);
        assert_eq!(client.files.progress(), progress);
        println!(
            "L20-02 pull interrupted={interrupted}: {:?}; reload revision={}",
            client.trace,
            client.db().rev
        );
        client.rpc.load([Reply::Pulled(Page::empty(81, false))]);
        client.sync(false);
        assert_eq!(
            client.rpc.calls,
            [Request::Pull {
                since: 80,
                limit: 500
            }]
        );
        assert_eq!(client.files.progress(), Progress { seq: 81, ..BASE });
        client.rpc.assert_complete();
    }
}
