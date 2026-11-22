use super::rpc::{Op, Page, Rejection, Reply};
use super::{BASE, Client, Event, Key, Notice, Table, baseline};

#[test]
fn l20_03_rejection_distinguishes_authorization_from_third_silent_warning() {
    for error in [
        Rejection::Revoked,
        Rejection::Http401,
        Rejection::Http403,
        Rejection::Ordinary,
    ] {
        let ordinary = error == Rejection::Ordinary;
        let mut client = Client::new(baseline(), BASE, true);
        for attempt in 1..=if ordinary { 3 } else { 1 } {
            client.rpc.load([Reply::Rejected(Op::Pull, error)]);
            client.sync(true);
            assert_eq!(
                (client.err, client.failures, client.busy),
                (Some(error), attempt, false)
            );
            if ordinary && attempt < 3 {
                assert_eq!(client.trace.last(), Some(&Event::Rpc(Op::Pull)));
            }
        }
        assert_eq!(client.files.cloud_on(), ordinary);
        assert_eq!(client.files.progress(), BASE);
        assert_eq!(client.files.copies(), (baseline(), baseline()));
        let notices: Vec<_> = client
            .trace
            .iter()
            .filter_map(|event| match event {
                Event::Notice(notice) => Some(*notice),
                _ => None,
            })
            .collect();
        assert_eq!(
            notices,
            [if ordinary {
                Notice::Retrying(error)
            } else {
                Notice::Stopped(error)
            }]
        );
        if ordinary {
            client.rpc.load([Reply::Pulled(Page::empty(71, false))]);
            client.sync(false);
            assert_eq!((client.err, client.failures), (None, 0));
        } else {
            assert_eq!(client.db().row(Key(Table::Log, 1)).unwrap().value, -1);
            client.rpc.load([]);
            client.sync(false);
            assert!(client.rpc.calls.is_empty());
        }
        client.rpc.assert_complete();
        println!("L20-03 {error:?}: {:?}", client.trace);
    }
}
