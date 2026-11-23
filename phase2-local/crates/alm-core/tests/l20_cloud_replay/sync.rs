use super::{Client, MIRROR_INTERVAL, SNAP_INTERVAL};
use crate::rpc::{Rejection, Reply, Request};

impl Client {
    pub(super) fn sync_records(&mut self) -> Result<(), Rejection> {
        let outgoing = self.db().changed(self.files.progress().pushed);
        let mut pushed_max = 0;
        for part in outgoing.chunks(100) {
            let stale = match self.call(Request::Push(part.to_vec())) {
                Reply::Pushed(stale) => stale,
                Reply::Rejected(_, error) => return Err(error),
                _ => unreachable!("script checks reply operation"),
            };
            pushed_max = pushed_max.max(part.last().unwrap().sent_time());
            for record in stale {
                self.db_mut().apply(record, true);
            }
        }
        if pushed_max != 0 {
            let mut state = self.files.progress();
            state.pushed = state.pushed.max(pushed_max);
            self.checkpoint(state);
        }

        let mut applied = false;
        for _ in 0..200 {
            let mut state = self.files.progress();
            let page = match self.call(Request::Pull {
                since: state.seq,
                limit: 500,
            }) {
                Reply::Pulled(page) => page,
                Reply::Rejected(_, error) => return Err(error),
                _ => unreachable!("script checks reply operation"),
            };
            for record in page.rows {
                applied |= self.db_mut().apply(record, false);
            }
            state.seq = state.seq.max(page.max_seq);
            self.checkpoint(state);
            if !page.more {
                break;
            }
        }
        if applied {
            self.save();
            if !self.view.busy() {
                self.redraw(self.view.screen);
            }
        }
        Ok(())
    }

    pub(super) fn refresh_optional(&mut self) {
        let mut progress = self.files.progress();
        if self.now.saturating_sub(progress.mirror_at) > MIRROR_INTERVAL {
            match self.call(Request::Mirror { updated: self.now }) {
                Reply::Mirrored(completed) => {
                    self.now = completed;
                    progress.mirror_at = completed;
                    self.checkpoint(progress);
                }
                Reply::Rejected(..) => {}
                _ => unreachable!("script checks reply operation"),
            }
        }
        let progress = self.files.progress();
        if self.now.saturating_sub(progress.snap_at) > SNAP_INTERVAL
            && self.db().changed(progress.pushed).is_empty()
        {
            self.put_snapshot();
        }
    }

    pub(crate) fn put_snapshot(&mut self) {
        let mut progress = self.files.progress();
        let request = Request::PutSnapshot {
            seq: progress.seq,
            json: serde_json::to_string(self.db()).unwrap(),
        };
        match self.call(request) {
            Reply::SnapshotWritten(completed) => {
                self.now = completed;
                progress.snap_at = completed;
                self.checkpoint(progress);
            }
            Reply::Rejected(..) => {}
            _ => unreachable!("script checks reply operation"),
        }
    }
}
