use super::rpc::{Page, Reply, Request};
use super::{
    BASE, Client, Db, Device, Event, ITEM, Key, NOW, Notice, Progress, Table, baseline, row,
};

#[test]
fn l20_02_versioned_meta_and_stock_merge_preserves_save_revision_and_schema() {
    let meta = Key(Table::Meta, 0);
    let stock = Key(Table::Stock, 1);
    let missing_stock = Key(Table::Stock, 2);
    let before = [
        row(meta, (10, 5, 1), Device::Local),
        row(stock, (15, 5, 1), Device::Local),
    ];
    let mut seed = baseline();
    seed.rows.extend(before.clone());
    let mut client = Client::new(seed.clone(), BASE, true);
    let changed = [
        row(meta, (42, 6, 2), Device::Peer),
        row(stock, (7, 6, 2), Device::Peer),
        row(missing_stock, (8, 1, 2), Device::Peer),
    ];
    let stale = [
        row(meta, (90, 4, 9), Device::Peer),
        row(stock, (90, 5, 9), Device::Peer),
    ];
    client.rpc.load([Reply::Pulled(Page {
        max_seq: 71,
        more: false,
        rows: stale.into_iter().map(Into::into).collect(),
    })]);
    client.sync(false);
    assert_eq!(client.db(), &seed);
    assert_eq!(client.files.copies(), (seed.clone(), seed.clone()));
    client.rpc.load([Reply::Pulled(Page {
        max_seq: 72,
        more: false,
        rows: changed.clone().into_iter().map(Into::into).collect(),
    })]);
    client.sync(false);
    client.settle();
    let expected = Db::new(
        41,
        vec![
            changed[2].clone(),
            row(ITEM, (100, 2, 5), Device::Local),
            changed[0].clone(),
            changed[1].clone(),
        ],
    );
    assert_eq!(client.files.copies(), (expected.clone(), expected));
    assert_eq!(client.files.progress(), Progress { seq: 72, ..BASE });
    client.rpc.assert_complete();
    println!(
        "L20-02 metadata/stock versions and local revision: {:?}",
        client.trace
    );
}

#[test]
fn l20_03_optional_mirror_ack_is_throttled_and_checkpoints_completion_time() {
    for due in [false, true] {
        let initial = Progress {
            mirror_at: NOW - 900_000 - u64::from(due),
            ..BASE
        };
        let mut client = Client::new(baseline(), initial, true);
        let mut replies = vec![Reply::Pulled(Page::empty(71, false))];
        if due {
            replies.push(Reply::Mirrored(NOW + 30));
        }
        client.rpc.load(replies);
        client.sync(false);
        let mut calls = vec![Request::Pull {
            since: 70,
            limit: 500,
        }];
        if due {
            calls.push(Request::Mirror { updated: NOW });
        }
        assert_eq!(client.rpc.calls, calls);
        assert_eq!(
            client.files.progress(),
            Progress {
                seq: 71,
                mirror_at: if due { NOW + 30 } else { initial.mirror_at },
                ..initial
            }
        );
        assert_eq!(client.files.copies(), (baseline(), baseline()));
        assert_eq!(client.trace.last(), Some(&Event::Notice(Notice::Synced)));
        client.rpc.assert_complete();
        println!("L20-03 optional mirror due={due}: {:?}", client.trace);
    }
}
