use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::state::Db;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Read {
    Precheck(bool),
    Confirm { silent: bool, at: u64 },
    Pull,
    Probe,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CloudTrigger {
    Manual,
    Kick,
    Interval,
}

#[derive(Debug, Eq, PartialEq)]
pub(super) enum Request {
    Post {
        url: String,
        method: &'static str,
        mode: &'static str,
        content_type: &'static str,
        body: Vec<u8>,
    },
    Jsonp {
        read: Read,
        src: String,
        callback: String,
    },
    CloudPull {
        trigger: CloudTrigger,
        at: u64,
    },
    Open {
        url: String,
        target: &'static str,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Lifecycle {
    Register(usize),
    Append(usize),
    Loaded(usize),
    Ok(usize),
    Delete(usize),
    Remove(usize),
    Err(usize),
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct PushBody {
    pub action: String,
    pub db: Db,
}

#[derive(Default)]
pub(super) struct Script {
    pub requests: Vec<Request>,
    pub lifecycle: Vec<Lifecycle>,
    pub callbacks: BTreeSet<usize>,
    pub nodes: BTreeSet<usize>,
    pub probe_payloads: Vec<Option<Db>>,
    posts: BTreeMap<usize, bool>,
    reads: BTreeMap<usize, Read>,
}

impl Script {
    pub fn sync_pull(&mut self, trigger: CloudTrigger, at: u64) {
        self.requests.push(Request::CloudPull { trigger, at });
    }

    pub fn post(&mut self, db: Db, silent: bool) {
        self.posts.insert(self.requests.len(), silent);
        self.requests.push(Request::Post {
            url: db.meta.sync_url.clone(),
            method: "POST",
            mode: "no-cors",
            content_type: "text/plain;charset=utf-8",
            body: serde_json::to_vec(&PushBody {
                action: "push".into(),
                db,
            })
            .unwrap(),
        });
    }

    pub fn take_post(&mut self, id: usize) -> bool {
        self.posts
            .remove(&id)
            .expect("no pending POST at this capture index")
    }

    pub fn jsonp(&mut self, url: &str, now: u64, read: Read) -> usize {
        let id = self.requests.len();
        let callback = format!("__almJP_{now}{}", id % 10_000);
        let separator = if url.contains('?') { '&' } else { '?' };
        let src = format!("{url}{separator}action=pull&callback={callback}&t={now}");
        self.callbacks.insert(id);
        self.lifecycle.push(Lifecycle::Register(id));
        self.nodes.insert(id);
        self.lifecycle.push(Lifecycle::Append(id));
        self.requests.push(Request::Jsonp {
            read,
            src,
            callback,
        });
        self.reads.insert(id, read);
        id
    }

    pub fn begin_reply(&mut self, id: usize, loaded: bool) -> Read {
        let read = self
            .reads
            .remove(&id)
            .expect("no pending JSONP at this capture index");
        if loaded {
            self.lifecycle.push(Lifecycle::Ok(id));
        } else {
            self.cleanup(id);
            self.lifecycle.push(Lifecycle::Err(id));
        }
        read
    }

    pub fn loaded_without_callback(&mut self, id: usize) {
        assert!(self.nodes.contains(&id));
        self.lifecycle.push(Lifecycle::Loaded(id));
    }

    pub fn cleanup(&mut self, id: usize) {
        assert!(self.callbacks.remove(&id));
        self.lifecycle.push(Lifecycle::Delete(id));
        assert!(self.nodes.remove(&id));
        self.lifecycle.push(Lifecycle::Remove(id));
    }

    pub fn body(&self, id: usize) -> PushBody {
        let Request::Post { body, .. } = &self.requests[id] else {
            panic!("expected captured POST at {id}");
        };
        serde_json::from_slice(body).unwrap()
    }
}
