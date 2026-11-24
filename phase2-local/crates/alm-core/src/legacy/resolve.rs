//! Quarantine resolution (OPEN-003 / OPEN-004 rehearsal): the Owner records a decision for one
//! quarantined manifest entry, with a reason: import it as corrected (the corrected legacy
//! record is mapped by the core as any other and staged as a new authority record) or keep it
//! excluded. Nothing is resolved automatically and a decision is never replaced. The
//! reconciliation lists resolved and unresolved entries separately; its control lines still
//! compare the records the manifest placed.
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::import::{EntryStatus, ImportedRecord, KINDS, LegacyImports, encode, record_refs};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    ImportCorrected,
    KeepExcluded,
}

/// What the Owner sends: which entry (manifest, position, kind and legacy ID as reviewed), the
/// decision, the corrected record as the core mapped it (import only) and why.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct QuarantineDecision {
    pub manifest: u64,
    pub entry: u64,
    pub kind: String,
    pub legacy_id: String,
    pub decision: Decision,
    #[serde(default)]
    pub body: Option<Value>,
    pub reason: String,
}

/// A recorded decision; `authority_id` is the staged record of a corrected import.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Resolution {
    pub manifest: u64,
    pub entry: u64,
    pub kind: String,
    pub legacy_id: String,
    pub decision: Decision,
    pub reason: String,
    pub authority_id: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResolveError {
    /// No quarantined entry with that manifest, position, kind and legacy ID.
    NotQuarantined,
    AlreadyResolved,
    /// A decision needs a reason.
    NoReason,
    /// An import needs a corrected record; keeping it excluded takes none.
    Body,
    /// A record with this kind and legacy ID is already imported.
    AlreadyImported(String),
    /// The corrected record refers to a record no import placed.
    Unresolved(String, String),
}

/// Maps a corrected legacy record (the HTML's shape) of `kind` as the import mapping does.
///
/// # Errors
/// The mapping's issues (unknown kind, amounts below a paisa, fractional quantities, …).
pub fn map_corrected(kind: &str, record: &Value) -> Result<Value, String> {
    let (_, _, fields) = KINDS
        .iter()
        .find(|(k, ..)| *k == kind)
        .ok_or_else(|| format!("unknown kind {kind}"))?;
    if !record.is_object() {
        return Err("the corrected record must be a JSON object".into());
    }
    let mut issues = Vec::new();
    let body = encode(record, fields, &mut issues);
    if issues.is_empty() {
        Ok(body)
    } else {
        Err(issues.join("; "))
    }
}

/// The authority's rule for one decision. Returns the staged record's ID for a corrected
/// import.
///
/// # Errors
/// [`ResolveError`]; nothing changes.
pub fn resolve(
    imports: &mut LegacyImports,
    decision: &QuarantineDecision,
) -> Result<Option<String>, ResolveError> {
    let quarantined = imports
        .manifests
        .iter()
        .find(|m| m.id == decision.manifest)
        .and_then(|m| m.entries.get(usize::try_from(decision.entry).ok()?))
        .is_some_and(|e| {
            e.status == EntryStatus::Quarantined
                && e.kind == decision.kind
                && e.legacy_id == decision.legacy_id
        });
    if !quarantined {
        return Err(ResolveError::NotQuarantined);
    }
    if imports
        .resolutions
        .iter()
        .any(|r| r.manifest == decision.manifest && r.entry == decision.entry)
    {
        return Err(ResolveError::AlreadyResolved);
    }
    if decision.reason.trim().is_empty() {
        return Err(ResolveError::NoReason);
    }
    let authority_id = match (decision.decision, &decision.body) {
        (Decision::KeepExcluded, None) => None,
        (Decision::ImportCorrected, Some(body @ Value::Object(_))) => {
            if let Some(existing) = imports
                .records
                .iter()
                .find(|r| r.kind == decision.kind && r.legacy_id == decision.legacy_id)
            {
                return Err(ResolveError::AlreadyImported(existing.authority_id.clone()));
            }
            if let Some((kind, legacy)) = record_refs(&decision.kind, &decision.legacy_id, body)
                .into_iter()
                .find(|(kind, legacy)| {
                    !imports
                        .records
                        .iter()
                        .any(|r| &r.kind == kind && &r.legacy_id == legacy)
                })
            {
                return Err(ResolveError::Unresolved(kind, legacy));
            }
            let n = imports
                .records
                .iter()
                .filter(|r| r.kind == decision.kind)
                .count()
                + 1;
            let id = format!("{}-{n}", decision.kind);
            imports.records.push(ImportedRecord {
                authority_id: id.clone(),
                kind: decision.kind.clone(),
                legacy_id: decision.legacy_id.clone(),
                body: body.clone(),
                manifest: decision.manifest,
            });
            Some(id)
        }
        _ => return Err(ResolveError::Body),
    };
    imports.resolutions.push(Resolution {
        manifest: decision.manifest,
        entry: decision.entry,
        kind: decision.kind.clone(),
        legacy_id: decision.legacy_id.clone(),
        decision: decision.decision,
        reason: decision.reason.trim().to_owned(),
        authority_id: authority_id.clone(),
    });
    Ok(authority_id)
}

/// The record at `pointer` in a stored source file, as indented JSON for the Owner to correct;
/// `None` when the pointer is empty or names nothing.
pub fn source_record(source: &str, pointer: &str) -> Option<String> {
    if pointer.is_empty() {
        return None;
    }
    let raw: Value = serde_json::from_str(source).ok()?;
    serde_json::to_string_pretty(raw.pointer(pointer)?).ok()
}

/// [`map_corrected`] over the JSON text the Owner typed.
///
/// # Errors
/// Not JSON, or the mapping's issues.
pub fn map_corrected_text(kind: &str, text: &str) -> Result<Value, String> {
    let record: Value =
        serde_json::from_str(text).map_err(|e| format!("the corrected record is not JSON: {e}"))?;
    map_corrected(kind, &record)
}
