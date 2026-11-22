use super::Replay;
use super::schedule::{Cloud, CloudStep, START};
use super::scheduled_cases::COMPLETE;
use super::script::{CloudTrigger, Request};
use super::state::Db;

#[derive(Clone, Copy, Debug)]
enum Entry {
    Ready,
    Disabled,
    MissingToken,
    Busy,
    MissingDb,
}

impl Entry {
    fn set(self, app: &mut Replay) {
        match self {
            Self::Ready => (),
            Self::Disabled => app.cloud.on = false,
            Self::MissingToken => app.cloud.token = "",
            Self::Busy => app.cloud_busy = true,
            Self::MissingDb => app.db = None,
        }
    }
}

const ENTRIES: [Entry; 5] = [
    Entry::Ready,
    Entry::Disabled,
    Entry::MissingToken,
    Entry::Busy,
    Entry::MissingDb,
];

fn assert_rpc(app: &Replay, expected: Option<(CloudTrigger, u64)>) {
    if let Some((trigger, at)) = expected {
        assert_eq!(app.script.requests, [Request::CloudPull { trigger, at }]);
    } else {
        assert_eq!(app.script.requests, []);
    }
}

const fn armed(at: u64, due: u64, previous: Option<u64>) -> (u64, CloudStep) {
    (at, CloudStep::Armed { due, previous })
}

#[test]
fn l20_09_save_kick_checks_enable_flag_url_key_and_token_before_arming() {
    for cloud in [
        Cloud {
            on: false,
            ..COMPLETE
        },
        Cloud {
            url: "",
            ..COMPLETE
        },
        Cloud {
            key: "",
            ..COMPLETE
        },
        Cloud {
            token: "",
            ..COMPLETE
        },
    ] {
        let mut app = Replay::new("");
        app.cloud = cloud;
        app.save_product("local only");
        app.advance(START + 3_000);
        assert_eq!(app.timers.cloud, None);
        assert_eq!(app.timers.cloud_trace, [(START, CloudStep::Kick)]);
        assert_rpc(&app, None);
    }
    println!(
        "L20-09 save: off, missing URL, missing key and missing token each call syncKick without arming or dispatching"
    );
}

#[test]
fn l20_09_replaced_save_timeout_fires_and_rechecks_entry_without_self_retry() {
    const FIRST: u64 = START + 3_000;
    const SECOND: u64 = START + 3_500;
    for entry in ENTRIES {
        let mut app = Replay::new("");
        app.cloud = COMPLETE;
        app.save_product("first");
        app.advance(START + 500);
        app.save_product("second");
        app.advance(FIRST);
        entry.set(&mut app);
        if matches!(entry, Entry::Disabled) {
            app.kick();
        }
        let mut expected = vec![
            (START, CloudStep::Kick),
            armed(START, FIRST, None),
            (START + 500, CloudStep::Kick),
            armed(START + 500, SECOND, Some(FIRST)),
        ];
        if matches!(entry, Entry::Disabled) {
            expected.push((FIRST, CloudStep::Kick));
        }
        app.advance(SECOND - 1);
        assert_eq!(app.timers.cloud, Some(SECOND), "{entry:?}");
        assert_eq!(app.timers.cloud_trace, expected, "{entry:?}");
        assert_rpc(&app, None);

        app.advance(SECOND);
        expected.extend([
            (SECOND, CloudStep::Timeout),
            (SECOND, CloudStep::SyncEntry(CloudTrigger::Kick)),
        ]);
        let dispatch = matches!(entry, Entry::Ready).then_some((CloudTrigger::Kick, SECOND));
        assert_eq!(app.timers.cloud_trace, expected, "{entry:?}");
        assert_rpc(&app, dispatch);

        app.cloud = COMPLETE;
        app.cloud_busy = false;
        app.db = Some(app.disk.reload_selection());
        app.advance(START + 8_000);
        assert_eq!(app.timers.cloud, None);
        assert_eq!(app.timers.cloud_trace, expected, "{entry:?}");
        assert_rpc(&app, dispatch);
        let mut saved = Db::fixture("", "original product", 42);
        saved.products.extend(["first".into(), "second".into()]);
        app.assert_copies(&saved);
        assert_eq!(app.disk.intents.len(), 2);
        println!(
            "L20-09 save {entry:?}: replacement timeout and entry fired; sync_pull={}",
            dispatch.is_some()
        );
    }
}

#[test]
fn l20_09_interval_rechecks_visibility_enablement_and_strict_age_before_entry() {
    const TICK: u64 = START + 15_000;
    for (entry, hidden, age, enters, dispatches) in [
        (Entry::Ready, false, 54_999, false, false),
        (Entry::Ready, false, 55_000, false, false),
        (Entry::Ready, false, 55_001, true, true),
        (Entry::Disabled, false, 55_001, false, false),
        (Entry::MissingToken, false, 55_001, false, false),
        (Entry::Ready, true, 55_001, false, false),
        (Entry::Busy, false, 55_001, true, false),
        (Entry::MissingDb, false, 55_001, true, false),
    ] {
        let mut app = Replay::new("");
        let before = app.disk.copies();
        app.cloud = COMPLETE;
        app.cloud_last = TICK - age;
        app.advance(TICK - 1);
        assert!(app.timers.cloud_trace.is_empty());
        assert_rpc(&app, None);
        app.hidden = hidden;
        entry.set(&mut app);
        app.advance(TICK);
        let mut expected = vec![(TICK, CloudStep::Interval)];
        if enters {
            expected.push((TICK, CloudStep::SyncEntry(CloudTrigger::Interval)));
        }
        assert_eq!(
            app.timers.cloud_trace, expected,
            "{entry:?}, hidden={hidden}, age={age}"
        );
        assert_rpc(&app, dispatches.then_some((CloudTrigger::Interval, TICK)));
        assert_eq!((app.timers.cloud, app.timers.sheets), (None, None));
        assert_eq!(app.cloud_last, TICK - age);
        if dispatches {
            app.cloud_busy = false;
            let next = TICK + 15_000;
            app.advance(next - 1);
            assert_rpc(&app, Some((CloudTrigger::Interval, TICK)));
            app.advance(next);
            expected.extend([
                (next, CloudStep::Interval),
                (next, CloudStep::SyncEntry(CloudTrigger::Interval)),
            ]);
            assert_eq!(app.timers.cloud_trace, expected);
            assert_eq!(
                app.script.requests,
                [TICK, next].map(|at| Request::CloudPull {
                    trigger: CloudTrigger::Interval,
                    at,
                })
            );
        }
        assert!(app.disk.intents.is_empty());
        assert_eq!(app.disk.copies(), before);
        println!(
            "L20-09 interval {entry:?} hidden={hidden} age={age}: tick=true entry={enters} sync_pull={dispatches}"
        );
    }
}

#[test]
fn l20_09_visibility_kick_ignores_age_replaces_timer_and_rechecks_entry() {
    const VISIBLE: u64 = START + 500;
    const DUE: u64 = VISIBLE + 3_000;
    let mut disabled = Replay::new("");
    disabled.visibility(true);
    disabled.visibility(false);
    disabled.advance(DUE);
    assert_eq!(
        disabled.timers.cloud_trace,
        [
            (START, CloudStep::Visibility(true)),
            (START, CloudStep::Visibility(false)),
            (START, CloudStep::Kick),
        ]
    );
    assert_eq!(disabled.timers.cloud, None);
    assert_rpc(&disabled, None);

    for entry in ENTRIES {
        let mut app = Replay::new("");
        let before = app.disk.copies();
        app.cloud = COMPLETE;
        app.cloud_last = START;
        app.visibility(true);
        assert_eq!(app.timers.cloud, None);
        app.visibility(false);
        app.advance(VISIBLE);
        app.visibility(true);
        app.visibility(false);
        app.advance(DUE - 1);
        let first = START + 3_000;
        let expected = [
            (START, CloudStep::Visibility(true)),
            (START, CloudStep::Visibility(false)),
            (START, CloudStep::Kick),
            armed(START, first, None),
            (VISIBLE, CloudStep::Visibility(true)),
            (VISIBLE, CloudStep::Visibility(false)),
            (VISIBLE, CloudStep::Kick),
            armed(VISIBLE, DUE, Some(first)),
        ];
        assert_eq!(app.timers.cloud_trace, expected);
        assert_eq!(app.timers.cloud, Some(DUE));
        assert_rpc(&app, None);

        app.visibility(true);
        entry.set(&mut app);
        app.advance(DUE);
        assert_eq!(
            app.timers.cloud_trace[expected.len()..],
            [
                (DUE - 1, CloudStep::Visibility(true)),
                (DUE, CloudStep::Timeout),
                (DUE, CloudStep::SyncEntry(CloudTrigger::Kick)),
            ]
        );
        let dispatch = matches!(entry, Entry::Ready).then_some((CloudTrigger::Kick, DUE));
        assert_rpc(&app, dispatch);
        assert_eq!((app.timers.cloud, app.timers.sheets), (None, None));
        assert_eq!(app.cloud_last, START);
        assert!(app.disk.intents.is_empty());
        assert_eq!(app.disk.copies(), before);
        println!(
            "L20-09 visibility {entry:?}: visible re-armed without an age check; hidden before timeout; entry=true sync_pull={}",
            dispatch.is_some()
        );
    }
}
