use std::fs;
use std::path::{Path, PathBuf};
mod scratch_retry {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../alm-store/tests/common/scratch_retry.rs"
    ));
}

use serde::{Deserialize, Serialize};

struct Sandbox(PathBuf);
impl Sandbox {
    fn new() -> Self {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
        fs::create_dir_all(&root).unwrap();
        let path = scratch_retry::create(&root, "persist-replay-").unwrap();
        Self(path)
    }
    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
    fn write(&self, name: &str, value: &Snapshot) {
        fs::write(self.path(name), serde_json::to_vec(value).unwrap()).unwrap();
    }
    fn read(&self, name: &str) -> Snapshot {
        serde_json::from_slice(&fs::read(self.path(name)).unwrap()).unwrap()
    }
}
impl Drop for Sandbox {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct Snapshot {
    revision: u64,
    record: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Outcome {
    Success,
    Failure,
}

#[derive(Debug, Eq, PartialEq)]
enum Event {
    Revision(u64),
    MirrorAttempt(u64),
    MirrorResult(u64, Outcome),
    IdbInvoke(u64),
    FolderSchedule(u64),
    CloudSchedule(u64),
    SheetsCancel(Option<u64>),
    SheetsReplace(u64, u64),
    Stats(u64),
    IdbCapture(u64, u64),
    IdbResult(u64, Outcome),
    Warning(u64, &'static str, &'static str, u64),
}

struct Pending {
    call: u64,
    mirror: Outcome,
    captured: Option<Snapshot>,
}
struct Replay<'a> {
    dir: &'a Sandbox,
    db: Snapshot,
    timer: Option<u64>,
    pending: Vec<Pending>,
    trace: Vec<Event>,
}
impl<'a> Replay<'a> {
    fn new(dir: &'a Sandbox, seed: &Snapshot) -> Self {
        dir.write("mirror.json", seed);
        dir.write("primary.json", seed);
        Self {
            dir,
            db: seed.clone(),
            timer: None,
            pending: vec![],
            trace: vec![],
        }
    }
    fn persist(&mut self, record: &str, mirror: Outcome) {
        self.db.record = record.into();
        self.db.revision += 1;
        let call = self.db.revision;
        self.trace.push(Event::Revision(call));
        self.trace.push(Event::MirrorAttempt(call));
        if mirror == Outcome::Success {
            self.dir.write("mirror.json", &self.db);
        }
        self.trace.push(Event::MirrorResult(call, mirror));
        self.trace.push(Event::IdbInvoke(call));
        self.pending.push(Pending {
            call,
            mirror,
            captured: None,
        });
        self.trace.push(Event::FolderSchedule(call));
        self.trace.push(Event::CloudSchedule(call));
        self.trace.push(Event::SheetsCancel(self.timer));
        self.timer = Some(call);
        self.trace.push(Event::SheetsReplace(call, 2500));
        self.trace.push(Event::Stats(call));
    }
    // idbSet awaits opening the database before put; capture is controlled separately
    // from invocation and completion. This is a replay choice, not browser timing evidence.
    fn capture(&mut self, index: usize) {
        let pending = &mut self.pending[index];
        assert!(pending.captured.is_none());
        pending.captured = Some(self.db.clone());
        self.trace
            .push(Event::IdbCapture(pending.call, self.db.revision));
    }
    fn release(&mut self, index: usize, result: Outcome) {
        let pending = self.pending.remove(index);
        let snapshot = pending.captured.unwrap();
        if result == Outcome::Success {
            self.dir.write("primary.json", &snapshot);
        }
        self.trace.push(Event::IdbResult(pending.call, result));
        if pending.mirror == Outcome::Failure && result == Outcome::Failure {
            self.trace.push(Event::Warning(
                pending.call,
                "Could not save — use folder/cloud backup!",
                "bad",
                6000,
            ));
        }
    }
}

fn expected_call(call: u64, mirror: Outcome, previous: Option<u64>) -> Vec<Event> {
    vec![
        Event::Revision(call),
        Event::MirrorAttempt(call),
        Event::MirrorResult(call, mirror),
        Event::IdbInvoke(call),
        Event::FolderSchedule(call),
        Event::CloudSchedule(call),
        Event::SheetsCancel(previous),
        Event::SheetsReplace(call, 2500),
        Event::Stats(call),
    ]
}

#[test]
fn single_and_burst_saves_keep_fanout_ahead_of_delayed_completions() {
    for count in [1, 2] {
        for outcome in [Outcome::Success, Outcome::Failure] {
            // Given: independent, decodable seeded local files and controlled completions.
            let dir = Sandbox::new();
            let seed = Snapshot {
                revision: 10,
                record: "seed".into(),
            };
            let seed_bytes = serde_json::to_vec(&seed).unwrap();
            let mut replay = Replay::new(&dir, &seed);
            let mut expected = vec![];
            // When: all calls and captures occur before any completion is released.
            for n in 0..count {
                let revision = 11 + n;
                let record = if n == 0 { "first" } else { "second" };
                replay.persist(record, outcome);
                expected.extend(expected_call(
                    revision,
                    outcome,
                    if n == 0 { None } else { Some(revision - 1) },
                ));
                replay.capture(usize::try_from(n).unwrap());
                expected.push(Event::IdbCapture(revision, revision));
            }
            // Then: fan-out is complete, no warning has fired, and primary is still seeded.
            assert_eq!(replay.trace, expected);
            assert_eq!(replay.timer, Some(10 + count));
            assert_eq!(dir.read("primary.json"), seed);
            let latest = Snapshot {
                revision: 10 + count,
                record: if count == 1 { "first" } else { "second" }.into(),
            };
            assert_eq!(
                dir.read("mirror.json"),
                if outcome == Outcome::Success {
                    latest
                } else {
                    seed.clone()
                }
            );
            if outcome == Outcome::Failure {
                assert_eq!(fs::read(dir.path("mirror.json")).unwrap(), seed_bytes);
            }
            for n in 0..count {
                replay.release(0, outcome);
                expected.push(Event::IdbResult(11 + n, outcome));
                if outcome == Outcome::Failure {
                    expected.push(Event::Warning(
                        11 + n,
                        "Could not save — use folder/cloud backup!",
                        "bad",
                        6000,
                    ));
                }
                assert_eq!(replay.trace, expected);
                let written = Snapshot {
                    revision: 11 + n,
                    record: if n == 0 { "first" } else { "second" }.into(),
                };
                assert_eq!(
                    dir.read("primary.json"),
                    if outcome == Outcome::Success {
                        written
                    } else {
                        seed.clone()
                    }
                );
                if outcome == Outcome::Failure {
                    assert_eq!(fs::read(dir.path("primary.json")).unwrap(), seed_bytes);
                }
            }
            println!("count={count} outcome={outcome:?}: {:?}", replay.trace);
        }
    }
    println!(
        "Synthetic file replay only; IDB capture/release order controlled; browser timing and scheduler callbacks not run."
    );
}

#[test]
fn warning_belongs_to_the_call_whose_both_writes_fail() {
    // Given: a failed mirror followed by a successful mirror, with both IDB writes held.
    let dir = Sandbox::new();
    let seed = Snapshot {
        revision: 10,
        record: "seed".into(),
    };
    let mut replay = Replay::new(&dir, &seed);
    replay.persist("first", Outcome::Failure);
    replay.capture(0);
    replay.persist("second", Outcome::Success);
    replay.capture(1);
    // When: the later failed primary completes first, then the earlier failed primary.
    replay.release(1, Outcome::Failure);
    assert!(
        !replay
            .trace
            .iter()
            .any(|event| matches!(event, Event::Warning(..)))
    );
    replay.release(0, Outcome::Failure);
    // Then: only the earlier call warns, despite the later call completing first.
    let warnings: Vec<_> = replay
        .trace
        .iter()
        .filter(|event| matches!(event, Event::Warning(..)))
        .collect();
    assert_eq!(
        warnings,
        vec![&Event::Warning(
            11,
            "Could not save — use folder/cloud backup!",
            "bad",
            6000,
        )]
    );
    assert_eq!(dir.read("primary.json"), seed);
    assert_eq!(
        dir.read("mirror.json"),
        Snapshot {
            revision: 12,
            record: "second".into()
        }
    );
    println!("mixed outcomes: {:?}", replay.trace);
}
