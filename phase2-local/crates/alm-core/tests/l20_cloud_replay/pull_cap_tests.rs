use crate::client::Screen;
use crate::rpc::{Op, Page, Reply, Request};
use crate::{
    BASE, Client, Completed, Db, Device, Event, ITEM, Key, Notice, Progress, Table, baseline, row,
};

const CUSTOMER: Key = Key(Table::Customers, 7);
const DELETED: Key = Key(Table::Products, 8);
const TOMBSTONE: Key = Key(Table::Deleted, 8);
const ECHO: Key = Key(Table::Products, 9);
const CAP_ONLY: Key = Key(Table::Stock, 20);
const CONTINUED: Key = Key(Table::Products, 21);

const fn pull(since: u64) -> Request {
    Request::Pull { since, limit: 500 }
}

fn reopen_copies(client: &mut Client, expected: &Db, progress: Progress) {
    client.settle();
    let (primary, mirror) = client.files.copies();
    assert_eq!(&primary, expected, "independent primary file readback");
    assert_eq!(&mirror, expected, "independent mirror file readback");
    client.reload();
    assert_eq!(client.db(), expected, "reloaded working database");
    assert_eq!(
        client.trace.last(),
        Some(&Event::Complete(Completed::Primary {
            rev: expected.rev,
            written: true,
        }))
    );
    let stored = client.files.progress();
    assert_eq!(stored, progress, "durable cursor readback");
    let versions = |db: &Db| {
        db.rows
            .iter()
            .map(|r| (r.key, r.stamp.unwrap().version))
            .collect::<Vec<_>>()
    };
    println!(
        "L20-16 {} pulls: {:?} to {:?}; durable seq={}; primary r{} {:?}; mirror r{} {:?}",
        client.rpc.calls.len(),
        client.rpc.calls.first(),
        client.rpc.calls.last(),
        stored.seq,
        primary.rev,
        versions(&primary),
        mirror.rev,
        versions(&mirror),
    );
}

#[test]
fn l20_16_mixed_pages_save_the_capped_tail_and_continue_from_durable_cursor() {
    // Given a clean baseline and a seeded tombstone, with more=true on every capped page.
    let mut tombstone = row(TOMBSTONE, (0, 4, 2), Device::Peer);
    tombstone.deletes = Some(DELETED);
    let mut seed = baseline();
    seed.rows.push(tombstone.clone());
    let mut client = Client::new(seed.clone(), BASE, true);
    client.rpc.load((71..=270).map(|max_seq| {
        let rows = match max_seq {
            71 => vec![
                row(ITEM, (130, 3, 6), Device::Peer),
                row(ECHO, (900, 9, 6), Device::Local),
            ],
            127 => vec![row(CUSTOMER, (710, 1, 7), Device::Peer)],
            191 => vec![
                row(ITEM, (999, 3, 8), Device::Peer),
                row(DELETED, (800, 99, 8), Device::Peer),
            ],
            270 => vec![
                row(CUSTOMER, (720, 2, 9), Device::Peer),
                row(CAP_ONLY, (200, 1, 9), Device::Peer),
                row(ITEM, (999, 2, 9), Device::Peer),
            ],
            _ => vec![],
        };
        Reply::Pulled(Page {
            max_seq,
            more: true,
            rows: rows.into_iter().map(Into::into).collect(),
        })
    }));

    // When the first sync reaches the cap and the pending primary write settles.
    client.sync(false);
    assert_eq!(client.rpc.calls, (70..270).map(pull).collect::<Vec<_>>());
    client.rpc.assert_complete();
    let at_cap = Progress { seq: 270, ..BASE };
    assert_eq!(client.files.progress(), at_cap);
    let expected_at_cap = Db::new(
        41,
        vec![
            row(CAP_ONLY, (200, 1, 9), Device::Peer),
            row(CUSTOMER, (720, 2, 9), Device::Peer),
            row(ITEM, (130, 3, 6), Device::Peer),
            tombstone.clone(),
        ],
    );
    let (paging, finish) = client.trace.split_at(400);
    let (pairs, tail) = paging.as_chunks::<2>();
    assert!(tail.is_empty());
    assert!(pairs.iter().copied().eq((71..=270).map(|seq| [
        Event::Rpc(Op::Pull),
        Event::Progress(Progress { seq, ..BASE }, true),
    ])));
    assert_eq!(
        finish,
        [
            Event::Save {
                rev: 41,
                mirror: true,
            },
            Event::Redraw(Screen::Products),
            Event::Notice(Notice::Synced),
        ]
    );
    assert_eq!(client.files.copies(), (seed, expected_at_cap.clone()));
    reopen_copies(&mut client, &expected_at_cap, at_cap);

    // Then a new invocation starts at the saved cursor and persists only applicable rows.
    client.trace.clear();
    client.rpc.load([
        Reply::Pulled(Page {
            max_seq: 275,
            more: true,
            rows: vec![
                row(CONTINUED, (210, 1, 10), Device::Peer).into(),
                row(CAP_ONLY, (220, 2, 10), Device::Peer).into(),
            ],
        }),
        Reply::Pulled(Page {
            max_seq: 280,
            more: false,
            rows: vec![
                row(CUSTOMER, (999, 2, 10), Device::Peer).into(),
                row(ECHO, (910, 10, 10), Device::Local).into(),
            ],
        }),
    ]);
    client.sync(false);
    assert_eq!(client.rpc.calls, [pull(270), pull(275)]);
    client.rpc.assert_complete();
    let continued = Progress { seq: 280, ..BASE };
    assert_eq!(client.files.progress(), continued);
    assert_eq!(
        client.trace,
        [
            Event::Rpc(Op::Pull),
            Event::Progress(Progress { seq: 275, ..BASE }, true),
            Event::Rpc(Op::Pull),
            Event::Progress(continued, true),
            Event::Save {
                rev: 42,
                mirror: true,
            },
            Event::Redraw(Screen::Products),
            Event::Notice(Notice::Synced),
        ]
    );
    let expected_continued = Db::new(
        42,
        vec![
            row(CONTINUED, (210, 1, 10), Device::Peer),
            row(CAP_ONLY, (220, 2, 10), Device::Peer),
            row(CUSTOMER, (720, 2, 9), Device::Peer),
            row(ITEM, (130, 3, 6), Device::Peer),
            tombstone,
        ],
    );
    assert_eq!(
        client.files.copies(),
        (expected_at_cap, expected_continued.clone())
    );
    reopen_copies(&mut client, &expected_continued, continued);
}
