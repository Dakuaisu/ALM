use std::collections::VecDeque;

use super::records::Envelope;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Op {
    Push,
    Pull,
    Mirror,
    PutSnapshot,
    GetSnapshot,
}

#[derive(Debug, Eq, PartialEq)]
pub enum Request {
    Push(Vec<Envelope>),
    Pull { since: u64, limit: u16 },
    Mirror { updated: u64 },
    PutSnapshot { seq: u64, json: String },
    GetSnapshot,
}

impl Request {
    pub const fn op(&self) -> Op {
        match self {
            Self::Push(_) => Op::Push,
            Self::Pull { .. } => Op::Pull,
            Self::Mirror { .. } => Op::Mirror,
            Self::PutSnapshot { .. } => Op::PutSnapshot,
            Self::GetSnapshot => Op::GetSnapshot,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Rejection {
    Revoked,
    Http401,
    Http403,
    Ordinary,
}

pub struct Page {
    pub max_seq: u64,
    pub more: bool,
    pub rows: Vec<Envelope>,
}

impl Page {
    pub const fn empty(max_seq: u64, more: bool) -> Self {
        Self {
            max_seq,
            more,
            rows: Vec::new(),
        }
    }
}

pub struct Snapshot {
    pub json: String,
    pub seq: Option<u64>,
}

pub enum Reply {
    Pushed(Vec<Envelope>),
    Pulled(Page),
    Mirrored(u64),
    SnapshotWritten(u64),
    SnapshotRead(Option<Snapshot>),
    Rejected(Op, Rejection),
}

impl Reply {
    const fn op(&self) -> Op {
        match self {
            Self::Pushed(_) => Op::Push,
            Self::Pulled(_) => Op::Pull,
            Self::Mirrored(_) => Op::Mirror,
            Self::SnapshotWritten(_) => Op::PutSnapshot,
            Self::SnapshotRead(_) => Op::GetSnapshot,
            Self::Rejected(op, _) => *op,
        }
    }
}

#[derive(Default)]
pub struct Script {
    replies: VecDeque<Reply>,
    pub calls: Vec<Request>,
}

impl Script {
    pub fn load(&mut self, replies: impl IntoIterator<Item = Reply>) {
        assert!(
            self.replies.is_empty(),
            "unconsumed reply before next script"
        );
        self.replies.extend(replies);
        self.calls.clear();
    }

    pub fn call(&mut self, request: Request) -> Reply {
        let op = request.op();
        self.calls.push(request);
        let reply = self
            .replies
            .pop_front()
            .unwrap_or_else(|| panic!("unexpected {op:?}"));
        assert_eq!(op, reply.op());
        reply
    }

    pub fn assert_complete(&self) {
        assert!(self.replies.is_empty(), "script still has pending replies");
    }
}
