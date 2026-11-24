//! The reconciliation report: each control line of the legacy file against the same line
//! computed from the records the authority holds for one import manifest. A difference is
//! explained only when the legacy line without the quarantined records equals the authority's;
//! anything else is reported as unexplained. The numbering lines compare the legacy series'
//! next numbers with the authority's live series (shown, never issued).
use super::LegacyBackup;
use super::controls::{ControlSection, controls};
use super::import::{EntryStatus, LegacyImports, ManifestEntry, rebuilt, without_quarantined};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LineStatus {
    Match,
    /// The legacy value without the quarantined records equals the authority's.
    Explained,
    Unexplained(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReconciledLine {
    pub key: String,
    pub legacy: Option<String>,
    pub authority: Option<String>,
    pub status: LineStatus,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReconciledSection {
    pub name: &'static str,
    pub lines: Vec<ReconciledLine>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Reconciliation {
    pub digest: String,
    pub version: String,
    pub manifest: u64,
    pub imported: usize,
    pub already: usize,
    pub quarantined: Vec<ManifestEntry>,
    pub sections: Vec<ReconciledSection>,
}

impl Reconciliation {
    pub fn unexplained(&self) -> usize {
        self.sections
            .iter()
            .flat_map(|s| &s.lines)
            .filter(|l| matches!(l.status, LineStatus::Unexplained(_)))
            .count()
    }

    pub fn explained(&self) -> usize {
        self.sections
            .iter()
            .flat_map(|s| &s.lines)
            .filter(|l| l.status == LineStatus::Explained)
            .count()
    }
}

fn value<'a>(section: Option<&'a ControlSection>, key: &str) -> Option<&'a str> {
    section?
        .lines
        .iter()
        .find(|l| l.key == key)
        .map(|l| l.value.as_str())
}

/// The report for manifest `id`; `live` is the authority's live next invoice and next GST
/// number. `None` when the manifest is not in `imports`.
pub fn reconcile(
    backup: &LegacyBackup,
    imports: &LegacyImports,
    id: u64,
    live: (u64, u64),
) -> Option<Reconciliation> {
    let manifest = imports.manifests.iter().find(|m| m.id == id)?;
    let legacy = controls(backup);
    let cleaned = controls(&without_quarantined(backup, manifest));
    let authority = controls(&rebuilt(imports, id)?);
    let mut sections = Vec::new();
    for (index, section) in legacy.iter().enumerate() {
        let theirs = authority.get(index);
        let cleaned = cleaned.get(index);
        let mut keys: Vec<&str> = section.lines.iter().map(|l| l.key.as_str()).collect();
        for line in theirs.into_iter().flat_map(|s| &s.lines) {
            if !keys.contains(&line.key.as_str()) {
                keys.push(&line.key);
            }
        }
        let lines = keys
            .into_iter()
            .map(|key| {
                let l = value(Some(section), key);
                let a = value(theirs, key);
                let status = if l == a {
                    LineStatus::Match
                } else if value(cleaned, key) == a {
                    LineStatus::Explained
                } else {
                    LineStatus::Unexplained(
                        "differs from the legacy value without quarantined records".into(),
                    )
                };
                ReconciledLine {
                    key: key.to_owned(),
                    legacy: l.map(str::to_owned),
                    authority: a.map(str::to_owned),
                    status,
                }
            })
            .collect();
        sections.push(ReconciledSection {
            name: section.name,
            lines,
        });
    }
    let next = |section: &str, key: &str| {
        legacy
            .iter()
            .find(|s| s.name == section)
            .and_then(|s| s.lines.iter().find(|l| l.key == key))
            .map(|l| l.value.trim_start_matches('#').parse::<u64>().unwrap_or(0))
            .unwrap_or(0)
    };
    let mut numbering = Vec::new();
    for (label, legacy_next, live_next) in [
        (
            "invoice: legacy next vs authority live series",
            next("Invoices", "next invoice (not issued)"),
            live.0,
        ),
        (
            "GST number: legacy next vs authority live series",
            next("GST bills", "next GST number (not issued)"),
            live.1,
        ),
    ] {
        numbering.push(ReconciledLine {
            key: label.to_owned(),
            legacy: Some(legacy_next.to_string()),
            authority: Some(live_next.to_string()),
            status: if live_next >= legacy_next {
                LineStatus::Match
            } else {
                LineStatus::Unexplained(
                    "the authority's live series would reissue legacy numbers (series and \
                     continuity need an owner decision, OPEN-003)"
                        .into(),
                )
            },
        });
    }
    sections.push(ReconciledSection {
        name: "Numbering (shown, not issued)",
        lines: numbering,
    });
    Some(Reconciliation {
        digest: manifest.digest.clone(),
        version: manifest.version.clone(),
        manifest: id,
        imported: manifest.count(EntryStatus::Imported),
        already: manifest.count(EntryStatus::Already),
        quarantined: manifest
            .entries
            .iter()
            .filter(|e| e.status == EntryStatus::Quarantined)
            .cloned()
            .collect(),
        sections,
    })
}

/// The authority's live next invoice and next GST bill number, from its Owner book (shown,
/// never issued).
pub fn live_numbers(book: &crate::projection::book::OwnerBook) -> (u64, u64) {
    (
        book.settings.next_invoice,
        book.gst_bills.iter().map(|b| b.number.0).max().unwrap_or(0) + 1,
    )
}

/// A dry run: the report as it would be if `plan` were imported into `imports` now; nothing
/// is changed.
pub fn dry_run(backup: &LegacyBackup, imports: &LegacyImports, live: (u64, u64)) -> Reconciliation {
    let mut scratch = imports.clone();
    let id = super::import::apply(&mut scratch, &super::import::plan(backup));
    reconcile(backup, &scratch, id, live).unwrap_or_else(|| Reconciliation {
        digest: backup.digest.clone(),
        version: super::import::MAPPING_VERSION.into(),
        manifest: id,
        imported: 0,
        already: 0,
        quarantined: Vec::new(),
        sections: Vec::new(),
    })
}
