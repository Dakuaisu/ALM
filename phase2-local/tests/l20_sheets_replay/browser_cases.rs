use super::schedule::{Cloud, START};
use super::script::{Lifecycle, Read, Request};
use super::state::Stamp;
use super::{OTHER, Replay, Status, URL, remote};

#[test]
fn l20_14_open_sheet_is_conditional_and_only_captures_navigation_arguments() {
    let mut app = Replay::new("");
    assert!(!app.open_sheet());
    assert!(app.script.requests.is_empty());
    app.save_url(&format!("  {URL}  "));
    assert!(app.open_sheet());
    assert_eq!(
        app.script.requests,
        [Request::Open {
            url: format!("{URL}?action=open"),
            target: "_blank"
        }]
    );
    assert_eq!(app.disk.reload_selection().meta.sync_url, URL);
    println!("L20-14: Open Sheet captured an inert URL and _blank; no navigation performed");
}

#[test]
fn l20_18_jsonp_callback_error_and_no_callback_lifecycles() {
    for (url, separator) in [
        (URL, '?'),
        ("https://sheets.invalid/exec?synthetic=yes", '&'),
    ] {
        for outcome in [Some(Ok(Some(remote(OTHER, 123)))), Some(Err(())), None] {
            let mut app = Replay::new(url);
            let id = app.jsonp(Read::Probe);
            assert_eq!(
                app.script.requests,
                [Request::Jsonp {
                    read: Read::Probe,
                    src: format!("{url}{separator}action=pull&callback=__almJP_10000000&t=1000000"),
                    callback: "__almJP_10000000".into()
                }]
            );
            assert_eq!(
                app.script.lifecycle,
                [Lifecycle::Register(id), Lifecycle::Append(id)]
            );
            assert!(app.script.callbacks.contains(&id) && app.script.nodes.contains(&id));
            if let Some(outcome) = outcome {
                let expected_payloads: Vec<_> = outcome.clone().into_iter().collect();
                let expected = if outcome.is_ok() {
                    [
                        Lifecycle::Ok(id),
                        Lifecycle::Delete(id),
                        Lifecycle::Remove(id),
                    ]
                } else {
                    [
                        Lifecycle::Delete(id),
                        Lifecycle::Remove(id),
                        Lifecycle::Err(id),
                    ]
                };
                app.reply(id, outcome);
                assert_eq!(app.script.lifecycle[2..], expected);
                assert_eq!(app.script.probe_payloads, expected_payloads);
                assert!(app.script.callbacks.is_empty() && app.script.nodes.is_empty());
            } else {
                app.script.loaded_without_callback(id);
                app.advance(START + 900_000);
                assert_eq!(app.script.lifecycle[2..], [Lifecycle::Loaded(id)]);
                assert!(app.script.probe_payloads.is_empty());
                assert!(app.script.callbacks.contains(&id) && app.script.nodes.contains(&id));
            }
            assert!(app.disk.intents.is_empty());
            assert_eq!(app.script.requests.len(), 1);
        }
    }
    println!(
        "L20-18: JSONP success, load error, and no-callback cleanup order captured without a DOM"
    );
}

#[test]
fn l20_06_sidebar_precedence_and_stale_local_only_click_are_distinct() {
    let mut app = Replay::new(URL);
    app.blocked = Some(Stamp::new(OTHER, 900));
    let complete = Cloud {
        on: true,
        url: "https://cloud.invalid",
        key: "key",
        token: "token",
    };
    app.cloud = complete;
    for (last, error, expected, ok) in [
        (0, false, Status::CloudReady, true),
        (55, false, Status::CloudSynced(55), true),
        (55, true, Status::CloudRetry, false),
    ] {
        app.cloud_last = last;
        app.cloud_error = error;
        app.conflict_click = true;
        app.status_pointer = true;
        app.update_status();
        assert_eq!(
            (app.status, app.status.ok_dot(), app.conflict_click),
            (expected, ok, false)
        );
        assert!(!app.status_pointer);
    }
    app.cloud.token = "";
    app.update_status();
    assert_eq!(app.status, Status::Paused);
    assert!(app.conflict_click && !app.status.ok_dot());
    assert!(app.status_pointer);
    app.save_url(" ");
    assert_eq!(app.status, Status::Local);
    assert!(app.conflict_click && !app.status.ok_dot());
    assert!(app.status_pointer);
    app.click_status();
    assert!(app.dialog);
    app.later();
    app.save_url(URL);
    app.blocked = None;
    app.update_status();
    assert_eq!(app.status, Status::SheetsSynced(77));
    assert!(!app.conflict_click && app.status.ok_dot());
    assert!(!app.status_pointer);
    app.db_mut().meta.last_sync = 0;
    app.update_status();
    assert_eq!(app.status, Status::SheetsReady);
    assert!(!app.conflict_click && app.status.ok_dot());
    assert!(
        app.script.requests.is_empty(),
        "display precedence does not issue requests"
    );
}
