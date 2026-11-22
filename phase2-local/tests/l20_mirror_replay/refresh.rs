use std::collections::VecDeque;

use serde_json::{Value, json};

use super::database::DEVICE;
use super::fixture::{Fixture, TODAY};
use super::store::{State, Store};
use super::summary;

const TOKEN: &str = "synthetic-mirror-token";
const MIRROR_TTL: u64 = 15 * 60 * 1_000;
const SNAPSHOT_TTL: u64 = 86_400_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Rpc {
    Push,
    Pull,
    Mirror,
    Snapshot,
}

impl Rpc {
    const fn name(self) -> &'static str {
        match self {
            Self::Push => "sync_push",
            Self::Pull => "sync_pull",
            Self::Mirror => "mirror_write",
            Self::Snapshot => "put_snapshot",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Reply {
    Resolved,
    Rejected,
}

#[derive(Debug, PartialEq)]
pub(super) struct Call {
    pub(super) name: &'static str,
    pub(super) body: Value,
    pub(super) dispatched: u64,
    pub(super) completed: u64,
    pub(super) reply: Reply,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Save {
    Push,
    Pull,
    Mirror,
    Snapshot,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct Write {
    pub(super) stage: Save,
    pub(super) attempted: State,
    pub(super) saved: bool,
}

pub(super) struct Round {
    now: u64,
    script: VecDeque<(Rpc, u64, Reply)>,
    pub(super) calls: Vec<Call>,
    pub(super) writes: Vec<Write>,
    pub(super) reject_save: Option<Save>,
    pub(super) pending_after_pull: bool,
    pub(super) toast: Option<&'static str>,
}

impl Round {
    pub(super) fn new(now: u64, script: &[(Rpc, u64, Reply)]) -> Self {
        Self {
            now,
            script: script.iter().copied().collect(),
            calls: Vec::new(),
            writes: Vec::new(),
            reject_save: None,
            pending_after_pull: false,
            toast: None,
        }
    }

    fn rpc(&mut self, rpc: Rpc, body: Value) -> Reply {
        let (expected, latency, reply) = self.script.pop_front().expect("unexpected RPC");
        assert_eq!(rpc, expected);
        let dispatched = self.now;
        self.now += latency;
        self.calls.push(Call {
            name: rpc.name(),
            body,
            dispatched,
            completed: self.now,
            reply,
        });
        reply
    }

    fn save(&mut self, store: &Store, stage: Save, attempted: State) {
        let saved = self.reject_save != Some(stage) && store.write(attempted).is_ok();
        self.writes.push(Write {
            stage,
            attempted,
            saved,
        });
    }

    pub(super) fn run(&mut self, store: &Store, fixture: &Fixture, silent: bool) {
        let mut state = store.read();
        if state.pushed_m < 301 {
            let record = fixture.changed_record();
            let push = self.rpc(
                Rpc::Push,
                json!({
                    "p_token": TOKEN, "p_device": DEVICE, "p_records": [{
                        "collection": "sales", "id": "sale-1", "json": record,
                        "v": 4, "m": 301, "d": DEVICE
                    }]
                }),
            );
            assert_eq!(
                push,
                Reply::Resolved,
                "record sync is stubbed successful in L20-15"
            );
            state.pushed_m = 301;
            self.save(store, Save::Push, state);
        }

        let mut state = store.read();
        let pull = self.rpc(
            Rpc::Pull,
            json!({
                "p_token": TOKEN, "p_device": DEVICE, "p_since": state.seq, "p_limit": 500
            }),
        );
        assert_eq!(pull, Reply::Resolved, "single successful pull page");
        state.seq = state.seq.max(42);
        self.save(store, Save::Pull, state);

        let mut mirror = store.read();
        if self.now.saturating_sub(mirror.mirror_at) > MIRROR_TTL {
            let body = json!({"p_token": TOKEN, "p_tabs": {
                "tabs": summary::tabs(fixture, TODAY), "updated": self.now
            }});
            if self.rpc(Rpc::Mirror, body) == Reply::Resolved {
                mirror.mirror_at = self.now;
                self.save(store, Save::Mirror, mirror);
            }
        }

        let mut snapshot = store.read();
        if self.now.saturating_sub(snapshot.snap_at) > SNAPSHOT_TTL && !self.pending_after_pull {
            let body =
                json!({"p_token": TOKEN, "p_json": fixture.snapshot(), "p_seq": snapshot.seq});
            if self.rpc(Rpc::Snapshot, body) == Reply::Resolved {
                snapshot.snap_at = self.now;
                self.save(store, Save::Snapshot, snapshot);
            }
        }

        self.toast = (!silent).then_some("Cloud sync ✓");
        assert!(self.script.is_empty(), "unconsumed RPCs: {:?}", self.script);
    }
}
