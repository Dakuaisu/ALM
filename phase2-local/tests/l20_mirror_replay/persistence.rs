use super::fixture::{NOON_MS, seed};
use super::refresh::{
    Reply::{Rejected, Resolved},
    Round,
    Rpc::{Mirror, Pull, Snapshot},
    Save, Write,
};
use super::store::{State, Store};

#[test]
fn l20_15_resolved_mirror_and_failed_local_save_leave_persisted_timestamp_stale() {
    let fixture = seed();
    let initial = State {
        seq: 42,
        pushed_m: 301,
        mirror_at: NOON_MS - 900_001,
        snap_at: NOON_MS - 86_400_001,
    };
    let store = Store::seed(initial);
    let mut round = Round::new(
        NOON_MS,
        &[
            (Pull, 0, Resolved),
            (Mirror, 25, Resolved),
            (Snapshot, 5, Resolved),
        ],
    );
    round.reject_save = Some(Save::Mirror);
    round.run(&store, &fixture, false);
    assert_eq!(round.calls[1].reply, Resolved);
    assert_eq!(round.calls[1].body["p_tabs"]["updated"], NOON_MS);
    assert_eq!(round.calls[1].completed, NOON_MS + 25);
    assert_eq!(
        round.writes[1],
        Write {
            stage: Save::Mirror,
            attempted: State {
                mirror_at: NOON_MS + 25,
                ..initial
            },
            saved: false,
        }
    );
    let after_snapshot = State {
        snap_at: NOON_MS + 30,
        ..initial
    };
    assert_eq!(
        round.writes[2],
        Write {
            stage: Save::Snapshot,
            attempted: after_snapshot,
            saved: true
        }
    );
    assert_eq!(store.read(), after_snapshot);
    assert_eq!(round.toast, Some("Cloud sync ✓"));

    let mut retry = Round::new(NOON_MS + 31, &[(Pull, 0, Resolved), (Mirror, 2, Resolved)]);
    retry.run(&store, &fixture, true);
    assert_eq!(
        store.read(),
        State {
            mirror_at: NOON_MS + 33,
            ..after_snapshot
        }
    );
}

#[test]
fn l20_15_snapshot_has_separate_strict_daily_idle_gate_and_retries() {
    let fixture = seed();
    let initial = State {
        seq: 42,
        pushed_m: 301,
        mirror_at: NOON_MS,
        snap_at: NOON_MS - 86_400_000,
    };
    let store = Store::seed(initial);
    for (now, pending) in [(NOON_MS - 1, false), (NOON_MS, false), (NOON_MS + 1, true)] {
        let mut round = Round::new(now, &[(Pull, 0, Resolved)]);
        round.pending_after_pull = pending;
        round.run(&store, &fixture, true);
        assert_eq!(round.writes.len(), 1);
        assert_eq!(store.read(), initial);
    }
    let mut failure = Round::new(NOON_MS + 1, &[(Pull, 0, Resolved), (Snapshot, 8, Rejected)]);
    failure.run(&store, &fixture, true);
    assert_eq!(failure.writes.len(), 1);
    assert_eq!(store.read(), initial);
    let mut retry = Round::new(
        NOON_MS + 10,
        &[(Pull, 0, Resolved), (Snapshot, 7, Resolved)],
    );
    retry.run(&store, &fixture, true);
    assert_eq!(
        store.read(),
        State {
            snap_at: NOON_MS + 17,
            ..initial
        }
    );
    assert_eq!(retry.calls[1].body["p_seq"], 42);
}
