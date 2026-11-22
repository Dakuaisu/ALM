use super::client::{Client, Event, Live, Notice, Screen};
use super::records::Db;
use super::rpc::{Reply, Request};

impl Client {
    pub fn first_upload(&mut self) {
        if !self.enabled() {
            self.note(Notice::EnableFirst);
            return;
        }
        let mut progress = self.files.progress();
        progress.pushed = 0;
        self.checkpoint(progress);
        self.db_mut().stamp_legacy();
        self.note(Notice::Uploading);
        self.sync(true);
        self.put_snapshot();
        self.audit(1);
        self.persist();
        self.note(self.err.map_or(Notice::Uploaded, Notice::UploadIssue));
    }

    pub fn rebuild(&mut self, confirmed: bool) {
        if !self.enabled() {
            self.note(Notice::EnableFirst);
            return;
        }
        if !confirmed {
            return;
        }
        let snapshot = match self.call(Request::GetSnapshot) {
            Reply::SnapshotRead(Some(snapshot)) if !snapshot.json.is_empty() => snapshot,
            Reply::SnapshotRead(_) => {
                self.note(Notice::NoSnapshot);
                return;
            }
            Reply::Rejected(_, error) => {
                self.note(Notice::RebuildRpcFailed(error));
                return;
            }
            _ => unreachable!("script checks reply operation"),
        };
        let old = self.db().clone();
        self.files.begin_safety(&old);
        self.trace.push(Event::SafetyStarted(old.rev));
        let Ok(parsed) = serde_json::from_str(&snapshot.json) else {
            self.note(Notice::MalformedSnapshot);
            return;
        };
        self.live = Live::Assigned(parsed);
        let Live::Assigned(parsed) = &self.live else {
            unreachable!()
        };
        let Ok(db) = serde_json::from_value::<Db>(parsed.clone()) else {
            self.note(Notice::MigrationFailed);
            return;
        };
        self.live = Live::Ready(db);
        let mut progress = self.files.progress();
        progress.seq = snapshot.seq.unwrap_or(0);
        progress.pushed = self.now;
        self.checkpoint(progress);
        self.save();
        self.audit(2);
        self.sync(true);
        self.redraw(Screen::Dashboard);
        self.note(Notice::Rebuilt);
    }
}
