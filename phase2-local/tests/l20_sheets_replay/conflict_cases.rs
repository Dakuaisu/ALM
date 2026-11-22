use super::schedule::START;
use super::script::{Read, Request};
use super::state::{Activity, Sales, Stamp, WriteIntent, WriteMode};
use super::{DEVICE, Notice, OTHER, Replay, Status, URL, remote};

fn conflict(silent: bool) -> Replay {
    let mut app = Replay::new(URL);
    app.disk.set_seen(&Stamp::new("previous-device", 800));
    app.disk.intents.clear();
    app.push(silent);
    app.reply(0, Ok(Some(remote(OTHER, 900))));
    assert_eq!(app.blocked, Some(Stamp::new(OTHER, 900)));
    assert_eq!(app.notices, [Notice::Conflict]);
    assert_eq!(app.dialog, !silent);
    assert_eq!(app.status, Status::Paused);
    assert!(app.conflict_click && !app.status.ok_dot());
    assert_eq!(app.script.requests.len(), 1);
    app
}

#[test]
fn l20_06_later_preserves_conflict_and_force_bypasses_precheck_only_on_confirmation() {
    let mut app = conflict(false);
    let before = app.disk.reload_selection();
    app.later();
    app.save_product("still local");
    app.advance(START + 2_500);
    assert_eq!(app.script.requests.len(), 1);
    assert_eq!(app.notices, [Notice::Conflict]);
    assert!(!app.dialog);
    assert_eq!(app.blocked, Some(Stamp::new(OTHER, 900)));
    app.push(false);
    app.force(false);
    assert!(app.dialog);
    assert_eq!(app.script.requests.len(), 1);
    app.force(true);
    let body = app.script.body(1);
    assert_eq!(body.db.log, [Activity::Force(OTHER.into())]);
    assert_eq!(body.db.meta.push, Stamp::new(DEVICE, START + 2_500));
    assert_eq!(body.db.products, ["original product", "still local"]);
    assert!(app.blocked.is_none() && !app.dialog);
    let mut locally_saved = before;
    locally_saved.meta.rev = 41;
    locally_saved.products.push("still local".into());
    app.assert_copies(&locally_saved);
    app.settle_post(1, false);
    assert_eq!(app.notices, [Notice::Conflict, Notice::PostFailed]);
    app.assert_copies(&locally_saved);
    assert_eq!(app.disk.seen(), Stamp::new("previous-device", 800));
}

#[test]
fn l20_06_preflight_allows_old_equal_same_device_and_unreadable_results() {
    for reply in [
        Err(()),
        Ok(None),
        Ok(Some(remote(OTHER, 799))),
        Ok(Some(remote(OTHER, 800))),
        Ok(Some(remote(DEVICE, 900))),
    ] {
        let expected_check = if reply.is_ok() { START } else { 0 };
        let mut app = Replay::new(URL);
        app.disk.set_seen(&Stamp::new("previous-device", 800));
        let before = app.disk.copies();
        app.push(false);
        app.reply(0, reply);
        assert_eq!(app.script.body(1).db.meta.push, Stamp::new(DEVICE, START));
        assert_eq!(app.last_remote, expected_check);
        assert!(app.blocked.is_none());
        assert_eq!(app.disk.copies(), before);
        assert!(app.notices.is_empty());
    }
}

#[test]
fn l20_06_missing_failed_and_malformed_pull_preserve_durable_copy() {
    let mut malformed = remote(OTHER, 901);
    malformed.meta.schema = 1;
    malformed.sales = Sales::Malformed("not an array".into());
    for (result, notice) in [
        (Ok(None), Notice::NoData),
        (Err(()), Notice::PullLoadFailed),
        (Ok(Some(malformed.clone())), Notice::PullError),
    ] {
        let mut app = conflict(true);
        let before = app.disk.reload_selection();
        app.click_status();
        assert!(app.dialog);
        app.pull_latest();
        assert!(matches!(
            app.script.requests[1],
            Request::Jsonp {
                read: Read::Pull,
                ..
            }
        ));
        app.reply(1, result);
        assert_eq!(app.notices, [Notice::Conflict, Notice::Pulling, notice]);
        assert!(!app.dialog);
        assert_eq!(app.blocked, Some(Stamp::new(OTHER, 900)));
        assert_eq!(app.disk.seen(), Stamp::new("previous-device", 800));
        assert_eq!((app.last_remote, app.pushed_m), (0, 812));
        assert!(app.redraws.is_empty());
        app.assert_copies(&before);
        if notice == Notice::PullError {
            assert_eq!(app.db(), &malformed);
            assert_eq!(
                app.disk.intents,
                [WriteIntent::MigrationBackup(malformed.clone())]
            );
        } else {
            assert_eq!(app.db(), &before);
            assert!(app.disk.intents.is_empty());
        }
    }
}

#[test]
fn l20_06_pull_success_does_not_imply_stamp_or_primary_durability() {
    for (seen_writable, mirror_writable, primary, rev) in [
        (true, true, WriteMode::Now, 3),
        (true, true, WriteMode::Now, -5),
        (false, true, WriteMode::Hold, 3),
        (true, false, WriteMode::Hold, 3),
        (true, true, WriteMode::Reject, 3),
    ] {
        let mut app = conflict(false);
        let before = app.disk.reload_selection();
        app.disk.seen_writable = seen_writable;
        app.disk.mirror_writable = mirror_writable;
        app.disk.primary = primary;
        let mut incoming = remote(OTHER, 901);
        incoming.meta.rev = rev;
        app.pull_latest();
        app.advance(START + 80);
        app.reply(1, Ok(Some(incoming.clone())));
        let mut expected = incoming;
        expected.meta.rev = if rev == 3 { 4 } else { 1 };
        expected.log.push(Activity::Pull(OTHER.into()));
        assert_eq!(
            app.disk.intents,
            [
                WriteIntent::Seen(Stamp::new(OTHER, 901)),
                WriteIntent::Save(expected.clone())
            ]
        );
        assert_eq!(
            app.notices,
            [Notice::Conflict, Notice::Pulling, Notice::Pulled]
        );
        assert_eq!(app.db(), &expected);
        assert_eq!(app.redraws, ["stock"]);
        assert_eq!(
            (app.blocked.as_ref(), app.last_remote, app.pushed_m),
            (None, START + 80, 812)
        );
        assert_eq!(app.script.requests.len(), 2, "no direct catch-up");
        let expected_seen = if seen_writable {
            Stamp::new(OTHER, 901)
        } else {
            Stamp::new("previous-device", 800)
        };
        assert_eq!(app.disk.seen(), expected_seen);
        let mirror = if mirror_writable { &expected } else { &before };
        let primary_before = if primary == WriteMode::Now {
            &expected
        } else {
            &before
        };
        assert_eq!(app.disk.copies(), [mirror.clone(), primary_before.clone()]);
        assert_eq!(app.disk.reload_selection(), *primary_before);
        if primary == WriteMode::Hold {
            let saved = mirror_writable;
            app.complete_primary(saved);
            let last = if saved {
                Notice::Pulled
            } else {
                Notice::SaveFailed
            };
            assert_eq!(app.notices.last(), Some(&last));
            let selected = if saved { expected } else { before };
            app.assert_copies(&selected);
            assert_eq!(app.disk.reload_selection(), selected);
        }
    }
    println!(
        "L20-06: Pull success can precede or outlive a failed modeled write; reload selection checked independently"
    );
}

#[test]
fn l20_06_manual_pull_waits_for_replacement_confirmation() {
    let mut app = Replay::new(URL);
    app.pull(false);
    assert_eq!(app.notices, [Notice::ConfirmReplace]);
    assert!(app.script.requests.is_empty());
    app.pull(true);
    assert!(matches!(
        app.script.requests.as_slice(),
        [Request::Jsonp {
            read: Read::Pull,
            ..
        }]
    ));
}
