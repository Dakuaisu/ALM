use super::schedule::{Cloud, START};
use super::script::{CloudTrigger, Read, Request};
use super::state::{Db, WriteIntent};
use super::{Notice, Replay, Status, URL};

pub(super) const COMPLETE: Cloud = Cloud {
    on: true,
    url: "https://cloud.invalid/",
    key: "synthetic-key",
    token: "synthetic-token",
};

#[test]
fn l20_01_guarded_save_commits_with_both_integrations_off() {
    let mut app = Replay::new("");
    app.save_product("offline product");
    assert!(app.script.requests.is_empty());
    let mut expected = Db::fixture("", "original product", 41);
    expected.products.push("offline product".into());
    assert_eq!(app.disk.intents, [WriteIntent::Save(expected.clone())]);
    app.assert_copies(&expected);
    assert_eq!(app.disk.reload_selection(), expected);
    assert_eq!(
        (app.timers.cloud, app.timers.sheets),
        (None, Some(START + 2_500))
    );
    app.cloud_sync(CloudTrigger::Manual);
    app.push(false);
    app.pull(false);
    app.advance(START + 75_000);
    assert_eq!(app.notices, [Notice::MissingUrl, Notice::MissingUrl]);
    assert_eq!(app.status, Status::Local);
    assert!(!app.status.ok_dot() && !app.conflict_click);
    assert!(
        app.script.requests.is_empty(),
        "off integrations must not dispatch"
    );
    assert_eq!(app.disk.intents.len(), 1);
    app.assert_copies(&expected);
    println!("L20-01: both reopened copies contain the guarded save; no external effects captured");
}

#[test]
fn l20_09_l20_10_both_configured_save_fires_sheet_before_cloud() {
    let mut app = Replay::new(URL);
    app.cloud = COMPLETE;
    app.last_remote = START;
    app.save_product("shared scheduled input");
    assert_eq!(
        (app.timers.sheets, app.timers.cloud),
        (Some(START + 2_500), Some(START + 3_000))
    );
    let mut saved = Db::fixture(URL, "original product", 41);
    saved.products.push("shared scheduled input".into());
    app.assert_copies(&saved);
    app.advance(START + 2_499);
    assert!(app.script.requests.is_empty());
    app.advance(START + 2_500);
    assert!(matches!(
        app.script.requests.as_slice(),
        [Request::Post { .. }]
    ));
    let mut posted = saved.clone();
    posted.meta.push = super::state::Stamp::new(super::DEVICE, START + 2_500);
    assert_eq!(app.script.body(0).db, posted);
    app.advance(START + 3_000);
    assert_eq!(
        app.script.requests[1],
        Request::CloudPull {
            trigger: CloudTrigger::Kick,
            at: START + 3_000
        }
    );
    assert_eq!(app.script.requests.len(), 2);
    app.assert_copies(&saved);
    assert_eq!(app.disk.intents, [WriteIntent::Save(saved)]);
}

#[test]
fn l20_10_sheets_callback_uses_current_url_and_replaces_pending_timer() {
    for (saved, at_fire, recent) in [
        ("", "", false),
        (URL, "", false),
        ("", URL, false),
        (URL, URL, true),
        (URL, URL, false),
    ] {
        let mut app = Replay::new(saved);
        app.save_product("first");
        app.advance(START + 400);
        app.save_product("second");
        app.advance(START + 2_500);
        assert!(app.script.requests.is_empty());
        assert_eq!(app.timers.sheets, Some(START + 2_900));
        app.db_mut().meta.sync_url = at_fire.into();
        if recent {
            app.last_remote = START + 2_900 - 600_000;
        }
        app.advance(START + 2_900);
        match (at_fire.is_empty(), recent, app.script.requests.as_slice()) {
            (true, _, []) => (),
            (false, true, [Request::Post { url, .. }]) => assert_eq!(url, at_fire),
            (
                false,
                false,
                [
                    Request::Jsonp {
                        read: Read::Precheck(true),
                        src,
                        ..
                    },
                ],
            ) => assert!(src.starts_with(at_fire)),
            (_, _, unexpected) => panic!("wrong scheduled route: {unexpected:?}"),
        }
        if recent {
            let before = app.disk.copies();
            app.settle_post(0, false);
            assert_eq!(app.script.requests.len(), 1);
            assert!(app.notices.is_empty());
            assert_eq!(app.disk.copies(), before);
        }
        assert_eq!(app.disk.reload_selection().meta.sync_url, saved);
    }
    println!(
        "L20-10: URL added/removed at fire time; recent and stale checks take different routes"
    );
}
