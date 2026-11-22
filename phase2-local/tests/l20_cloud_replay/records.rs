use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum Table {
    Products,
    Customers,
    Log,
    Deleted,
    Stock,
    Meta,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Device {
    Local,
    Peer,
    Legacy,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Key(pub Table, pub u64);

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Stamp {
    pub version: u64,
    pub time: u64,
    pub device: Device,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Row {
    pub key: Key,
    pub value: i64,
    pub stamp: Option<Stamp>,
    pub deletes: Option<Key>,
}

impl Row {
    pub const fn new(key: Key, value: i64, stamp: Option<Stamp>) -> Self {
        Self {
            key,
            value,
            stamp,
            deletes: None,
        }
    }

    fn version(&self) -> u64 {
        self.stamp.map_or(0, |s| s.version)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Envelope {
    pub key: Key,
    pub device: Device,
    pub row: Row,
}

impl From<Row> for Envelope {
    fn from(row: Row) -> Self {
        Self {
            key: row.key,
            device: row.stamp.map_or(Device::Local, |s| s.device),
            row,
        }
    }
}

impl Envelope {
    pub fn sent_time(&self) -> u64 {
        self.row.stamp.map_or(0, |s| s.time)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Db {
    #[serde(default)]
    pub rev: u64,
    pub schema: u32,
    pub rows: Vec<Row>,
}

impl Db {
    pub const fn new(rev: u64, rows: Vec<Row>) -> Self {
        Self {
            rev,
            schema: 2,
            rows,
        }
    }

    pub fn row(&self, key: Key) -> Option<&Row> {
        self.rows.iter().find(|row| row.key == key)
    }

    pub fn changed(&self, pushed: u64) -> Vec<Envelope> {
        let mut rows: Vec<_> = self
            .rows
            .iter()
            .filter(|row| row.stamp.is_some_and(|s| s.time > pushed))
            .cloned()
            .map(Envelope::from)
            .collect();
        rows.sort_by_key(|entry| (entry.sent_time(), entry.key.0));
        rows
    }

    pub fn stamp_legacy(&mut self) {
        for row in &mut self.rows {
            row.stamp.get_or_insert(Stamp {
                version: 1,
                time: 1,
                device: Device::Legacy,
            });
        }
    }

    pub fn apply(&mut self, entry: Envelope, from_stale: bool) -> bool {
        if entry.device == Device::Local && !from_stale {
            return false;
        }
        let existing = self.rows.iter().position(|row| row.key == entry.key);
        let newer = existing.is_none_or(|i| entry.row.version() > self.rows[i].version());
        let key = entry.key;
        let applies = match key.0 {
            Table::Meta => existing.is_some() && newer,
            Table::Products | Table::Customers | Table::Log => {
                newer
                    && !self
                        .rows
                        .iter()
                        .any(|row| row.key.0 == Table::Deleted && row.deletes == Some(key))
            }
            Table::Stock | Table::Deleted => newer,
        };
        let deletion = match key.0 {
            Table::Deleted => entry.row.deletes,
            Table::Products | Table::Customers | Table::Stock | Table::Meta | Table::Log => None,
        };
        if applies {
            match existing {
                Some(i) => self.rows[i] = entry.row,
                None => self.rows.insert(0, entry.row),
            }
        }
        let before = self.rows.len();
        if let Some(target) = deletion {
            self.rows.retain(|row| row.key != target);
        }
        applies || self.rows.len() != before
    }
}
