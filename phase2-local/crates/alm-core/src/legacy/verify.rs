//! Re-checks an import against its stored source file: the file is re-parsed and re-mapped
//! with the core ([`plan_file`]) and the rebuilt plan is compared record by record with the
//! authority's manifest and the records it holds. It catches a plan the device altered while
//! sending the true file (the authority checks only the file's digest and the plan's shape).
use std::fmt;

use super::import::{EntryStatus, ImportManifest, LegacyImports, plan_file};

/// One disagreement between the stored file's mapping and the manifest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Finding {
    /// Position in the manifest (`None` for file-level findings).
    pub entry: Option<usize>,
    pub kind: String,
    pub legacy_id: String,
    pub problem: String,
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.entry {
            Some(n) => write!(
                f,
                "entry {n} {} {}: {}",
                self.kind, self.legacy_id, self.problem
            ),
            None => write!(f, "{}", self.problem),
        }
    }
}

enum Expected<'a> {
    Quarantined(&'a str),
    Record(&'a serde_json::Value),
}

/// The findings for `manifest` against `source`, the file the authority stored for it, and the
/// records `imports` holds. Empty when the manifest is what the core maps from the file.
pub fn verify(source: &[u8], manifest: &ImportManifest, imports: &LegacyImports) -> Vec<Finding> {
    let file = |problem: String| Finding {
        entry: None,
        kind: String::new(),
        legacy_id: String::new(),
        problem,
    };
    let digest = super::sha256_hex(source);
    if digest != manifest.digest {
        return vec![file(format!(
            "the stored file's SHA-256 {digest} is not the manifest's {}",
            manifest.digest
        ))];
    }
    let plan = match plan_file(source) {
        Ok((_, plan)) => plan,
        Err(error) => return vec![file(format!("the stored file does not parse: {error:?}"))],
    };
    let by_id: std::collections::BTreeMap<&str, &super::import::ImportedRecord> = imports
        .records
        .iter()
        .map(|r| (r.authority_id.as_str(), r))
        .collect();
    let mut findings = Vec::new();
    if plan.version != manifest.version {
        findings.push(file(format!(
            "mapping {} rebuilt, manifest says {}",
            plan.version, manifest.version
        )));
        return findings;
    }
    // `import::apply` lists the plan's quarantine first, then each record, in order.
    let expected: Vec<(&str, &str, &str, Expected<'_>)> = plan
        .quarantine
        .iter()
        .map(|q| {
            (
                q.kind.as_str(),
                q.legacy_id.as_str(),
                q.source.as_str(),
                Expected::Quarantined(&q.reason),
            )
        })
        .chain(plan.records.iter().map(|r| {
            (
                r.kind.as_str(),
                r.legacy_id.as_str(),
                r.source.as_str(),
                Expected::Record(&r.body),
            )
        }))
        .collect();
    for (n, (kind, legacy_id, place, want)) in expected.iter().enumerate() {
        let mut add = |problem: String| {
            findings.push(Finding {
                entry: Some(n),
                kind: (*kind).to_owned(),
                legacy_id: (*legacy_id).to_owned(),
                problem,
            });
        };
        let Some(entry) = manifest.entries.get(n) else {
            add("in the file but missing from the manifest".into());
            continue;
        };
        if entry.kind != *kind || entry.legacy_id != *legacy_id {
            add(format!(
                "the manifest has {} {} here",
                entry.kind, entry.legacy_id
            ));
            continue;
        }
        if entry.source != *place {
            add(format!(
                "placed at {:?} in the manifest, at {place:?} in the file",
                entry.source
            ));
        }
        match (want, entry.status) {
            (Expected::Quarantined(reason), EntryStatus::Quarantined) => {
                if entry.reason != *reason {
                    add(format!(
                        "quarantined for {:?}, the file gives {reason:?}",
                        entry.reason
                    ));
                }
            }
            (Expected::Quarantined(reason), _) => {
                add(format!(
                    "the file quarantines it ({reason}) but the manifest placed it"
                ));
            }
            (Expected::Record(body), EntryStatus::Imported | EntryStatus::Already) => {
                let stored = entry
                    .authority_id
                    .as_deref()
                    .and_then(|id| by_id.get(id).copied());
                match stored {
                    None => add("the manifest names no stored record".into()),
                    Some(record) if &&record.body != body => add(format!(
                        "stored record {} differs from the file's mapping",
                        record.authority_id
                    )),
                    Some(_) => {}
                }
            }
            (Expected::Record(body), EntryStatus::Quarantined) => {
                let differs = entry
                    .reason
                    .strip_prefix("differs from the imported record ")
                    .map(|id| by_id.get(id).is_none_or(|r| &&r.body != body));
                let unresolved = entry.reason.starts_with("unresolved reference ");
                if differs != Some(true) && !unresolved {
                    add(format!(
                        "quarantined for {:?}, which the file's mapping does not explain",
                        entry.reason
                    ));
                }
            }
        }
    }
    for (n, entry) in manifest.entries.iter().enumerate().skip(expected.len()) {
        findings.push(Finding {
            entry: Some(n),
            kind: entry.kind.clone(),
            legacy_id: entry.legacy_id.clone(),
            problem: "in the manifest but not in the file".into(),
        });
    }
    findings
}
