use super::client::{Focus, Screen, View};
use super::rpc::{Op, Page, Rejection, Reply, Request};
use super::storage::Completed;
use super::{BASE, Client, Db, Device, Event, Key, NOW, Notice, Progress, Table, baseline, row};

#[path = "pull_cap_tests.rs"]
mod pull_cap_tests;

const FRESH: Key = Key(Table::Products, 222);

#[test]
fn l20_16_terminal_page_and_cap_both_checkpoint_without_saving_db() {
    for capped in [false, true] {
        let pages = if capped { 200 } else { 2 };
        let mut client = Client::new(baseline(), BASE, true);
        client
            .rpc
            .load((1..=pages).map(|i| Reply::Pulled(Page::empty(70 + i, capped || i < pages))));
        client.sync(false);
        assert!(
            client.rpc.calls.iter().eq((70..70 + pages)
                .map(|since| Request::Pull { since, limit: 500 })
                .collect::<Vec<_>>()
                .iter())
        );
        let expected = Progress {
            seq: 70 + pages,
            ..BASE
        };
        assert_eq!(client.files.progress(), expected);
        assert_eq!(client.files.copies(), (baseline(), baseline()));
        let (checkpoints, tail) = client.trace[..client.trace.len() - 1].as_chunks::<2>();
        assert!(tail.is_empty());
        for (i, pair) in checkpoints.iter().enumerate() {
            let seq = 71 + u64::try_from(i).unwrap();
            assert_eq!(
                *pair,
                [
                    Event::Rpc(Op::Pull),
                    Event::Progress(Progress { seq, ..BASE }, true)
                ]
            );
        }
        assert_eq!(client.trace.len(), usize::try_from(2 * pages + 1).unwrap());
        assert_eq!(client.trace.last(), Some(&Event::Notice(Notice::Synced)));
        client.rpc.assert_complete();
        println!(
            "L20-16 capped={capped}; calls={pages}; progress={expected:?}; both file revisions=40; normal sync notice"
        );

        client.rpc.load([Reply::Pulled(Page::empty(1, false))]);
        client.sync(false);
        assert_eq!(
            client.rpc.calls,
            [Request::Pull {
                since: expected.seq,
                limit: 500
            }]
        );
        assert_eq!(client.files.progress(), expected);
    }
}

#[test]
fn l20_16_failed_progress_write_reuses_the_last_durable_cursor() {
    let mut client = Client::new(baseline(), BASE, true);
    client.files.progress_policy.push_back(false);
    client.rpc.load([
        Reply::Pulled(Page::empty(90, true)),
        Reply::Pulled(Page::empty(80, false)),
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
                since: 70,
                limit: 500
            }
        ]
    );
    assert_eq!(client.files.progress(), Progress { seq: 80, ..BASE });
    assert_eq!(client.files.copies(), (baseline(), baseline()));
    assert!(
        client
            .trace
            .contains(&Event::Progress(Progress { seq: 90, ..BASE }, false))
    );
    println!("L20-16 suppressed state-write failure: {:?}", client.trace);
}

#[test]
fn l20_17_view_guards_suppress_redraw_but_not_save() {
    let idle = View::default();
    let bill = View {
        screen: Screen::Bill,
        draft: true,
        ..idle
    };
    let gst = View {
        screen: Screen::GstSales,
        gst_draft: true,
        ..idle
    };
    let fresh = row(FRESH, (42, 1, 6), Device::Peer);
    let expected = Db::new(41, vec![fresh.clone(), baseline().rows.remove(0)]);
    for (name, view, redraw) in [
        ("idle", idle, true),
        (
            "modal",
            View {
                modal: true,
                ..idle
            },
            false,
        ),
        ("bill draft", bill, false),
        ("GST draft", gst, false),
        (
            "INPUT",
            View {
                focus: Focus::Input,
                ..idle
            },
            false,
        ),
        (
            "SELECT",
            View {
                focus: Focus::Select,
                ..idle
            },
            false,
        ),
        (
            "TEXTAREA",
            View {
                focus: Focus::Textarea,
                ..idle
            },
            false,
        ),
        (
            "bill draft off screen",
            View {
                screen: Screen::Products,
                ..bill
            },
            true,
        ),
        (
            "GST draft off screen",
            View {
                screen: Screen::Products,
                ..gst
            },
            true,
        ),
    ] {
        let mut client = Client::new(baseline(), BASE, true);
        client.view = view;
        client.rpc.load([Reply::Pulled(Page {
            max_seq: 91,
            more: false,
            rows: vec![fresh.clone().into()],
        })]);
        client.sync(false);
        assert_eq!(client.db(), &expected);
        assert_eq!(client.files.progress(), Progress { seq: 91, ..BASE });
        assert_eq!(client.files.copies(), (baseline(), expected.clone()));
        assert_eq!(client.painted.row(FRESH), redraw.then_some(&fresh));
        let mut trace = vec![
            Event::Rpc(Op::Pull),
            Event::Progress(Progress { seq: 91, ..BASE }, true),
            Event::Save {
                rev: 41,
                mirror: true,
            },
        ];
        if redraw {
            trace.push(Event::Redraw(view.screen));
        }
        trace.push(Event::Notice(Notice::Synced));
        client.settle();
        trace.push(Event::Complete(Completed::Primary {
            rev: 41,
            written: true,
        }));
        assert_eq!(client.trace, trace);
        assert_eq!(client.files.copies(), (expected.clone(), expected.clone()));
        println!("L20-17 {name}; redraw={redraw}; {:?}", client.trace);
    }
}

#[test]
fn l20_03_optional_rpc_rejections_do_not_abort_completed_record_sync() {
    let initial = Progress {
        snap_at: 0,
        mirror_at: 0,
        ..BASE
    };
    let fresh = row(FRESH, (42, 1, 6), Device::Peer);
    let expected = Db::new(41, vec![fresh.clone(), baseline().rows.remove(0)]);
    let mut client = Client::new(baseline(), initial, true);
    client.rpc.load([
        Reply::Pulled(Page {
            max_seq: 92,
            more: false,
            rows: vec![fresh.into()],
        }),
        Reply::Rejected(Op::Mirror, Rejection::Http403),
        Reply::Rejected(Op::PutSnapshot, Rejection::Ordinary),
    ]);
    client.sync(false);
    assert_eq!(
        client.rpc.calls,
        [
            Request::Pull {
                since: 70,
                limit: 500
            },
            Request::Mirror { updated: NOW },
            Request::PutSnapshot {
                seq: 92,
                json: serde_json::to_string(&expected).unwrap()
            }
        ]
    );
    assert_eq!(client.files.progress(), Progress { seq: 92, ..initial });
    assert_eq!((client.err, client.files.cloud_on()), (None, true));
    assert_eq!(client.trace.last(), Some(&Event::Notice(Notice::Synced)));
    client.settle();
    assert_eq!(client.files.copies(), (expected.clone(), expected));
    println!(
        "L20-03 optional failures after merged record save: {:?}",
        client.trace
    );
}
