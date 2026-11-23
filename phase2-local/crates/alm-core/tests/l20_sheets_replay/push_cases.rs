use super::schedule::START;
use super::script::{Read, Request};
use super::state::{Db, Stamp, WriteIntent};
use super::{DEVICE, Notice, OTHER, Replay, Status, URL, remote};

#[test]
fn l20_05_opaque_post_only_confirms_matching_readback() {
    for (script, notice) in [
        (Ok(Some(remote(DEVICE, START))), Notice::Confirmed),
        (Ok(Some(remote(DEVICE, START - 1))), Notice::Mismatch),
        (Ok(Some(remote(OTHER, START))), Notice::Mismatch),
        (Ok(None), Notice::Mismatch),
        (
            Ok(Some(Db::fixture(URL, "missing stamp", 3))),
            Notice::Mismatch,
        ),
        (Err(()), Notice::ReadbackFailed),
    ] {
        let mut app = Replay::new(URL);
        let original = Db::fixture(URL, "original product", 40);
        app.push(false);
        assert!(matches!(
            app.script.requests.as_slice(),
            [Request::Jsonp {
                read: Read::Precheck(false),
                ..
            }]
        ));
        app.reply(0, Ok(None));
        let mut stamped = original.clone();
        stamped.meta.push = Stamp::new(DEVICE, START);
        let Request::Post {
            url,
            method,
            mode,
            content_type,
            ..
        } = &app.script.requests[1]
        else {
            panic!("no POST")
        };
        assert_eq!(
            (url.as_str(), *method, *mode, *content_type),
            (URL, "POST", "no-cors", "text/plain;charset=utf-8")
        );
        let body = app.script.body(1);
        assert_eq!((body.action.as_str(), body.db), ("push", stamped.clone()));
        app.settle_post(1, true);
        assert!(matches!(
            app.script.requests[2],
            Request::Jsonp {
                read: Read::Confirm {
                    at: START,
                    silent: false
                },
                ..
            }
        ));
        app.assert_copies(&original);
        assert!(app.disk.intents.is_empty());
        app.advance(START + 25);
        app.reply(2, script);
        assert_eq!(app.notices, [notice]);
        if notice == Notice::Confirmed {
            stamped.meta.rev = 41;
            stamped.meta.last_sync = START + 25;
            assert_eq!(app.status, Status::SheetsSynced(START + 25));
            assert_eq!(app.disk.seen(), Stamp::new(DEVICE, START));
            assert_eq!(
                app.disk.intents,
                [
                    WriteIntent::Seen(stamped.meta.push.clone()),
                    WriteIntent::Save(stamped.clone())
                ]
            );
            app.assert_copies(&stamped);
        } else {
            assert_eq!(app.status, Status::SheetsSynced(77));
            assert_eq!(app.disk.seen(), Stamp::default());
            assert!(app.disk.intents.is_empty());
            app.assert_copies(&original);
        }
    }
    println!(
        "L20-05: opaque POST never commits; exact matching device and time confirm at callback time"
    );
}

#[test]
fn l20_05_late_post_uses_latest_stamp_but_keeps_immutable_body() {
    for returned in [START, START + 10] {
        let mut app = Replay::new(URL);
        app.last_remote = START;
        app.push(false);
        app.advance(START + 10);
        app.push(false);
        assert_eq!(app.script.body(0).db.meta.push, Stamp::new(DEVICE, START));
        assert_eq!(
            app.script.body(1).db.meta.push,
            Stamp::new(DEVICE, START + 10)
        );
        app.settle_post(0, true);
        assert!(
            matches!(app.script.requests[2], Request::Jsonp { read: Read::Confirm { at, silent: false }, .. } if at == START + 10)
        );
        app.advance(START + 20);
        app.reply(2, Ok(Some(remote(DEVICE, returned))));
        let durable = app.disk.reload_selection();
        if returned == START {
            assert_eq!(
                (
                    durable.meta.last_sync,
                    app.disk.seen(),
                    app.notices.as_slice()
                ),
                (77, Stamp::default(), &[Notice::Mismatch][..])
            );
        } else {
            assert_eq!(
                (
                    durable.meta.last_sync,
                    app.disk.seen(),
                    app.notices.as_slice()
                ),
                (
                    START + 20,
                    Stamp::new(DEVICE, START + 10),
                    &[Notice::Confirmed][..]
                )
            );
        }
    }
}

#[test]
fn l20_05_throttled_trailing_check_captures_stamp_at_fire_even_if_post_pending() {
    for resolve_b_before_timer in [false, true] {
        let mut app = Replay::new(URL);
        app.last_remote = START;
        app.checked_at = START;
        app.advance(START + 100);
        app.push(true);
        app.settle_post(0, true);
        assert_eq!(app.timers.confirm, Some(START + 600_000));
        app.advance(START + 200);
        app.push(true);
        if resolve_b_before_timer {
            app.settle_post(1, true);
        }
        app.advance(START + 599_999);
        assert_eq!(app.script.requests.len(), 2);
        assert_eq!(app.disk.reload_selection().meta.last_sync, 77);
        assert!(app.disk.intents.is_empty());
        app.advance(START + 600_000);
        assert!(
            matches!(app.script.requests[2], Request::Jsonp { read: Read::Confirm { silent: true, at }, .. } if at == START + 200)
        );
        assert_eq!(app.script.requests.len(), 3);
        assert_eq!(app.timers.confirm, None);
        app.reply(2, Ok(Some(remote(DEVICE, START + 200))));
        let durable = app.disk.reload_selection();
        assert_eq!(durable.meta.last_sync, START + 600_000);
        assert_eq!(app.disk.seen(), Stamp::new(DEVICE, START + 200));
        app.assert_copies(&durable);
        assert!(app.notices.is_empty());
    }
}

#[test]
fn l20_05_in_flight_confirmations_compare_their_own_stamps_out_of_order() {
    let mut app = Replay::new(URL);
    app.last_remote = START;
    app.push(false);
    app.settle_post(0, true);
    app.advance(START + 20);
    app.push(false);
    app.settle_post(2, true);
    app.advance(START + 40);
    app.reply(3, Ok(Some(remote(DEVICE, START + 20))));
    assert_eq!(app.disk.seen(), Stamp::new(DEVICE, START + 20));
    app.advance(START + 60);
    app.reply(1, Ok(Some(remote(DEVICE, START))));
    let durable = app.disk.reload_selection();
    assert_eq!(
        (durable.meta.rev, durable.meta.last_sync, durable.meta.push),
        (42, START + 60, Stamp::new(DEVICE, START + 20))
    );
    assert_eq!(app.disk.seen(), Stamp::new(DEVICE, START));
    assert_eq!(app.notices, [Notice::Confirmed, Notice::Confirmed]);
}
