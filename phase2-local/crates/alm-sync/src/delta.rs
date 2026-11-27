//! Incremental pulls over the projection rows both authorities send. Every array of rows at
//! the top of the rows object, of `owner`, of `owner.book` and of `manager.book` is a tracked
//! list; its rows are named by a hash of their content. An authority remembers, per role, when
//! each (list, row) was first seen and when it went, and when each list last changed, for a
//! retained window of revisions. A delta for a device's checkpoint carries the rows object with
//! each tracked list replaced by `{"$same": true}` (unchanged since the checkpoint) or
//! `{"$rows": [hash, ...]}` (its rows in order), the rows first seen after the checkpoint
//! (`added`, by hash) and the rows that went (`removed`, tombstones by list). Older
//! checkpoints, a first sync or another role get the full rows.
use std::collections::{BTreeMap, BTreeSet};

use alm_core::projection::StoredRows;
use serde_json::{Map, Value, json};

/// Revisions of change history an authority keeps by default.
pub const DEFAULT_KEEP: u64 = 10_000;

/// A row's name in its list: 64 bits of the SHA-256 of its JSON text (lists hold at most
/// some 10^5 rows, so a collision within one list is about one in 10^9).
pub fn row_hash(row: &Value) -> String {
    let mut hex = alm_core::legacy::sha256_hex(row.to_string().as_bytes());
    hex.truncate(16);
    hex
}

/// The tracked lists of `rows` with their rows in order: `(path, rows)`.
pub fn lists(rows: &Value) -> Vec<(String, &Vec<Value>)> {
    fn level<'a>(out: &mut Vec<(String, &'a Vec<Value>)>, object: &'a Value, prefix: &str) {
        if let Value::Object(map) = object {
            for (key, value) in map {
                if let Value::Array(items) = value {
                    out.push((format!("{prefix}{key}"), items));
                }
            }
        }
    }
    let mut out = Vec::new();
    level(&mut out, rows, "");
    level(&mut out, &rows["owner"], "owner.");
    level(&mut out, &rows["owner"]["book"], "owner.book.");
    level(&mut out, &rows["manager"]["book"], "manager.book.");
    out
}

fn place<'a>(rows: &'a mut Value, path: &str) -> Option<&'a mut Value> {
    let mut parts: Vec<&str> = path.split('.').collect();
    let last = parts.pop()?;
    let mut node = rows;
    for part in parts {
        node = node.get_mut(part)?;
    }
    node.get_mut(last)
}

fn list_hash(hashes: &[String]) -> String {
    alm_core::legacy::sha256_hex(hashes.join(",").as_bytes())
}

#[derive(Clone, Debug, Default)]
struct RoleLog {
    floor: u64,
    synced: Option<u64>,
    rows: BTreeMap<(String, String), (u64, Option<u64>)>,
    lists: BTreeMap<String, (String, u64)>,
}

/// An authority's change history, per role (`"owner"`, `"manager"`).
#[derive(Clone, Debug)]
pub struct Tracker {
    keep: u64,
    roles: BTreeMap<String, RoleLog>,
}

impl Default for Tracker {
    fn default() -> Self {
        Self::new(DEFAULT_KEEP)
    }
}

impl Tracker {
    pub fn new(keep: u64) -> Self {
        Self {
            keep,
            roles: BTreeMap::new(),
        }
    }

    /// Records `rows` as `role`'s projection at `revision`.
    pub fn sync(&mut self, role: &str, revision: u64, rows: &Value) {
        let keep = self.keep;
        let log = self
            .roles
            .entry(role.to_owned())
            .or_insert_with(|| RoleLog {
                floor: revision,
                ..RoleLog::default()
            });
        if log.synced == Some(revision) {
            return;
        }
        let mut present = BTreeSet::new();
        let mut paths = BTreeSet::new();
        for (path, items) in lists(rows) {
            let hashes: Vec<String> = items.iter().map(row_hash).collect();
            let whole = list_hash(&hashes);
            match log.lists.get_mut(&path) {
                Some((stored, _)) if *stored == whole => {}
                Some(entry) => *entry = (whole, revision),
                None => {
                    log.lists.insert(path.clone(), (whole, revision));
                }
            }
            for hash in hashes {
                present.insert((path.clone(), hash));
            }
            paths.insert(path);
        }
        log.lists.retain(|path, _| paths.contains(path));
        for (key, seen) in &mut log.rows {
            if seen.1.is_none() && !present.contains(key) {
                seen.1 = Some(revision);
            }
        }
        for key in present {
            let seen = log.rows.entry(key).or_insert((revision, None));
            if seen.1.is_some() {
                *seen = (revision, None);
            }
        }
        log.floor = log.floor.max(revision.saturating_sub(keep));
        let floor = log.floor;
        log.rows
            .retain(|_, (_, gone)| gone.is_none_or(|gone| gone >= floor));
        log.synced = Some(revision);
    }

    /// The delta from `base` to `rows` (already synced), or `None` when `base` is older than the
    /// retained history.
    pub fn delta(&self, role: &str, base: u64, rows: &Value) -> Option<Value> {
        let log = self.roles.get(role)?;
        if base < log.floor || log.synced.is_none_or(|synced| base > synced) {
            return None;
        }
        let mut skeleton = rows.clone();
        let mut added = Map::new();
        for (path, items) in lists(rows) {
            let changed = log.lists.get(&path).is_none_or(|(_, at)| *at > base);
            let replacement = if changed {
                let mut hashes = Vec::with_capacity(items.len());
                for item in items {
                    let hash = row_hash(item);
                    let first = log.rows.get(&(path.clone(), hash.clone())).map(|s| s.0);
                    if first.is_none_or(|first| first > base) {
                        added.entry(hash.clone()).or_insert_with(|| item.clone());
                    }
                    hashes.push(Value::String(hash));
                }
                json!({"$rows": hashes})
            } else {
                json!({"$same": true})
            };
            if let Some(slot) = place(&mut skeleton, &path) {
                *slot = replacement;
            }
        }
        let mut removed: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for ((path, hash), (first, gone)) in &log.rows {
            if gone.is_some_and(|gone| gone > base) && *first <= base {
                removed.entry(path.clone()).or_default().push(hash.clone());
            }
        }
        Some(json!({"base": base, "rows": skeleton, "added": added, "removed": removed}))
    }
}

/// Each list's row names in order, as an authority sends them with full rows.
pub fn hashes(rows: &Value) -> Value {
    Value::Object(
        lists(rows)
            .into_iter()
            .map(|(path, items)| {
                let names = items.iter().map(|i| Value::String(row_hash(i))).collect();
                (path, Value::Array(names))
            })
            .collect(),
    )
}

/// [`split`] under the authority's row names (`hashes`: list path → names in order; an empty
/// list may be absent).
///
/// # Errors
/// [`DeltaMismatch`] when the names do not match the rows.
pub fn split_named(rows: &Value, hashes: &Value) -> Result<StoredRows, DeltaMismatch> {
    let mut skeleton = rows.clone();
    let mut stored = BTreeMap::new();
    for (path, items) in lists(rows) {
        let names: Vec<Value> = match hashes.get(&path) {
            Some(Value::Array(names)) => names.clone(),
            None if items.is_empty() => Vec::new(),
            _ => return Err(DeltaMismatch),
        };
        if names.len() != items.len() {
            return Err(DeltaMismatch);
        }
        let pairs = names
            .iter()
            .zip(items)
            .map(|(name, item)| {
                Ok((
                    name.as_str().ok_or(DeltaMismatch)?.to_owned(),
                    item.to_string(),
                ))
            })
            .collect::<Result<Vec<_>, DeltaMismatch>>()?;
        if let Some(slot) = place(&mut skeleton, &path) {
            *slot = json!({"$rows": names});
        }
        stored.insert(path, pairs);
    }
    Ok(StoredRows {
        skeleton: skeleton.to_string(),
        lists: stored,
    })
}

/// Splits full `rows` into what a device stores, naming rows with [`row_hash`].
pub fn split(rows: &Value) -> StoredRows {
    let mut skeleton = rows.clone();
    let mut stored = BTreeMap::new();
    for (path, items) in lists(rows) {
        let pairs: Vec<(String, String)> = items
            .iter()
            .map(|item| (row_hash(item), item.to_string()))
            .collect();
        let hashes: Vec<Value> = pairs
            .iter()
            .map(|(h, _)| Value::String(h.clone()))
            .collect();
        if let Some(slot) = place(&mut skeleton, &path) {
            *slot = json!({"$rows": hashes});
        }
        stored.insert(path, pairs);
    }
    StoredRows {
        skeleton: skeleton.to_string(),
        lists: stored,
    }
}

/// A delta the device cannot apply to what it holds (a row it never received): it rebuilds.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeltaMismatch;

/// Applies `delta` to `stored`: the full rows, and what to store now (the new skeleton and
/// only the lists that changed).
///
/// # Errors
/// [`DeltaMismatch`] when a listed row is neither added nor held, or the delta is malformed.
pub fn merge(stored: &StoredRows, delta: &Value) -> Result<(Value, StoredRows), DeltaMismatch> {
    let mut rows = delta["rows"].clone();
    let mut skeleton = rows.clone();
    let held_skeleton: Value = serde_json::from_str(&stored.skeleton).map_err(|_| DeltaMismatch)?;
    let added = delta["added"].as_object().ok_or(DeltaMismatch)?;
    let mut changed = BTreeMap::new();
    let paths: Vec<String> = {
        let mut found = Vec::new();
        let mut level = |object: &Value, prefix: &str| {
            if let Value::Object(map) = object {
                for (key, value) in map {
                    if value.get("$rows").is_some() || value.get("$same").is_some() {
                        found.push(format!("{prefix}{key}"));
                    }
                }
            }
        };
        level(&rows, "");
        level(&rows["owner"], "owner.");
        level(&rows["owner"]["book"], "owner.book.");
        level(&rows["manager"]["book"], "manager.book.");
        found
    };
    for path in paths {
        let slot = place(&mut rows, &path).ok_or(DeltaMismatch)?;
        let list: Vec<(String, String)> = if slot.get("$same").is_some() {
            let expected = path
                .split('.')
                .try_fold(&held_skeleton, |node, part| node.get(part))
                .and_then(|list| list["$rows"].as_array().map(Vec::len))
                .ok_or(DeltaMismatch)?;
            let list = stored.lists.get(&path).cloned().unwrap_or_default();
            if list.len() != expected {
                return Err(DeltaMismatch);
            }
            list
        } else {
            let held: BTreeMap<&str, &str> = stored
                .lists
                .get(&path)
                .map(|l| l.iter().map(|(h, b)| (h.as_str(), b.as_str())).collect())
                .unwrap_or_default();
            let hashes = slot["$rows"].as_array().ok_or(DeltaMismatch)?;
            let mut list = Vec::with_capacity(hashes.len());
            for hash in hashes {
                let hash = hash.as_str().ok_or(DeltaMismatch)?;
                let body = match added.get(hash) {
                    Some(row) => row.to_string(),
                    None => (*held.get(hash).ok_or(DeltaMismatch)?).to_owned(),
                };
                list.push((hash.to_owned(), body));
            }
            changed.insert(path.clone(), list.clone());
            list
        };
        let hashes: Vec<Value> = list.iter().map(|(h, _)| Value::String(h.clone())).collect();
        if let Some(entry) = place(&mut skeleton, &path) {
            *entry = json!({"$rows": hashes});
        }
        let items = list
            .iter()
            .map(|(_, body)| serde_json::from_str(body).map_err(|_| DeltaMismatch))
            .collect::<Result<Vec<Value>, _>>()?;
        *slot = Value::Array(items);
    }
    Ok((
        rows,
        StoredRows {
            skeleton: skeleton.to_string(),
            lists: changed,
        },
    ))
}

/// What a device holds after `batch`: the snapshot, the rows to store, and whether they replace
/// everything (`None` when nothing changed).
///
/// # Errors
/// [`DeltaMismatch`] for changes that do not fit `held` (the device rebuilds from a full pull).
pub fn device_view(
    held: Option<&StoredRows>,
    batch: &alm_core::projection::ProjectionBatch,
) -> Result<Option<(alm_core::projection::ProjectionSnapshot, StoredRows, bool)>, DeltaMismatch> {
    use alm_core::projection::PulledRows;
    match (&batch.rows, &batch.snapshot) {
        (Some(PulledRows::Full { rows, hashes }), Some(snapshot)) => {
            Ok(Some((snapshot.clone(), split_named(rows, hashes)?, true)))
        }
        (Some(PulledRows::Changes(delta)), None) => {
            let (rows, keep) = merge(held.ok_or(DeltaMismatch)?, delta)?;
            let snapshot =
                crate::wire::decode_rows(&rows, batch.role).map_err(|_| DeltaMismatch)?;
            Ok(Some((snapshot, keep, false)))
        }
        (None, None) => Ok(None),
        _ => Err(DeltaMismatch),
    }
}

#[cfg(test)]
mod tests {
    use super::{Tracker, merge, split};
    use serde_json::json;

    #[test]
    fn deltas_rebuild_the_full_rows_and_name_what_went() {
        let mut t = Tracker::new(5);
        let v1 = json!({"sales": [{"id": 1}, {"id": 2}], "catalog": [{"id": 9}], "note": null,
            "owner": {"stock_value": 3, "book": {"sales": [{"d": 1}]}}});
        t.sync("owner", 1, &v1);
        let held = split(&v1);
        let v2 = json!({"sales": [{"id": 3}, {"id": 1}], "catalog": [{"id": 9}], "note": 7,
            "owner": {"stock_value": 4, "book": {"sales": [{"d": 1}, {"d": 2}]}}});
        t.sync("owner", 2, &v2);
        let delta = t.delta("owner", 1, &v2).unwrap();
        assert_eq!(delta["rows"]["catalog"], json!({"$same": true}));
        assert_eq!(delta["added"].as_object().unwrap().len(), 2);
        assert_eq!(delta["removed"]["sales"].as_array().unwrap().len(), 1);
        let (rows, changed) = merge(&held, &delta).unwrap();
        assert_eq!(rows, v2);
        assert!(changed.lists.contains_key("sales") && !changed.lists.contains_key("catalog"));
        assert_eq!(changed.skeleton, split(&v2).skeleton);
        // Unknown history, a checkpoint past the window, or a row the device lacks.
        assert!(t.delta("manager", 1, &v2).is_none());
        for rev in 3..10 {
            t.sync("owner", rev, &json!({"sales": [{"id": rev}]}));
        }
        assert!(
            t.delta("owner", 2, &json!({"sales": [{"id": 9}]}))
                .is_none()
        );
        assert!(merge(&split(&json!({"sales": []})), &delta).is_err());
    }
}
