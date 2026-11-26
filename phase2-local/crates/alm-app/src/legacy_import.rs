//! Settings > Backup & Restore > "Import legacy backup" (R-8 rehearsal, synthetic files only,
//! Owner only). A file from the app's `legacy-import` folder is read and migrated by the core
//! (`restoreJSON` + `migrate()`), mapped, and reconciled as a dry run against the authority's
//! current rows; only an explicit confirmation sends the import. Afterwards the manifest, the
//! quarantine and the reconciliation from the authority's rows are shown. Nothing is posted.
use std::fs;
use std::path::PathBuf;

use crate::ipc::State;
use alm_core::legacy::import::{EntryStatus, ImportManifest, ImportPlan, plan_file};
use alm_core::legacy::resolve::{Decision, QuarantineDecision, map_corrected_text, source_record};
use alm_core::legacy::{
    LegacyBackup, LegacyError, Reconciliation, dry_run_plan, live_numbers, reconcile,
};
use alm_sync::{Intent, Outcome};

use crate::{AppState, Trial, text, with};

pub(crate) enum Stage {
    DryRun(Reconciliation),
    Imported {
        manifest: ImportManifest,
        report: Reconciliation,
        /// Quarantined entries without a decision: position, and the record as the stored file
        /// holds it (for the Owner to correct).
        open: Vec<(usize, String)>,
    },
}

pub(crate) struct Review {
    pub(crate) name: String,
    backup: LegacyBackup,
    plan: ImportPlan,
    pub(crate) stage: Stage,
}

fn valid_name(name: &str) -> bool {
    name.ends_with(".json")
        && !name.starts_with('.')
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
}

impl Trial {
    pub(crate) fn legacy_dir(&self) -> PathBuf {
        self.device_root.join("legacy-import")
    }

    /// Legacy backup files offered for a dry run, by name, with their sizes.
    pub(crate) fn legacy_files(&self) -> Vec<(String, u64)> {
        let mut list: Vec<_> = fs::read_dir(self.legacy_dir())
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|entry| {
                let name = entry.file_name().into_string().ok()?;
                let size = entry.metadata().ok()?.len();
                valid_name(&name).then_some((name, size))
            })
            .collect();
        list.sort();
        list
    }

    fn owner_book(&mut self) -> Result<alm_core::projection::book::OwnerBook, String> {
        self.projected()?
            .snapshot
            .owner
            .map(|owner| owner.book)
            .ok_or_else(|| "Posted data is not loaded yet.".to_owned())
    }

    /// Reads, migrates and maps a legacy file and shows the reconciliation as it would be after
    /// importing it now. Nothing is sent.
    pub(crate) fn legacy_dry_run(&mut self, name: &str) -> Result<(), String> {
        self.require_owner()?;
        if !valid_name(name) {
            return Err("Unknown legacy backup.".into());
        }
        let bytes = fs::read(self.legacy_dir().join(name)).map_err(text)?;
        let (backup, plan) = plan_file(&bytes).map_err(|error| match error {
            LegacyError::NotJson => "Not a legacy backup: the file is not JSON.".to_owned(),
            LegacyError::NoProducts => {
                "Not a legacy backup: it has no products (restoreJSON's check).".to_owned()
            }
        })?;
        let book = self.owner_book()?;
        let report = dry_run_plan(&backup, &plan, &book.legacy, live_numbers(&book));
        self.legacy = Some(Review {
            name: name.to_owned(),
            plan,
            backup,
            stage: Stage::DryRun(report),
        });
        self.message = format!("Dry run of {name}: nothing imported yet.");
        Ok(())
    }

    /// Sends the reviewed import (explicit confirmation), then reconciles it against the rows
    /// the authority holds.
    pub(crate) fn legacy_import_confirm(&mut self) -> Result<(), String> {
        self.require_owner()?;
        let review = self.legacy.take().ok_or("Run a dry run first.")?;
        if !matches!(review.stage, Stage::DryRun(_)) {
            self.legacy = Some(review);
            return Err("This file was imported already; run a new dry run.".into());
        }
        let outcome = self.send(&Intent::ImportLegacy(Box::new(review.plan.clone())));
        let id = match outcome {
            Ok(Outcome::Created(id)) => id,
            Ok(_) => {
                self.legacy = Some(review);
                return Err("Unexpected outcome.".into());
            }
            Err(error) => {
                self.legacy = Some(review);
                return Err(error);
            }
        };
        let stage = self.imported_stage(&review, id)?;
        if let Stage::Imported {
            manifest, report, ..
        } = &stage
        {
            self.message = format!(
                "Imported {} {}: manifest {id}, {} records, {} quarantined.",
                review.name,
                self.authority_name(),
                manifest.entries.len() - report.quarantined.len(),
                report.quarantined.len()
            );
        }
        self.legacy = Some(Review { stage, ..review });
        Ok(())
    }

    /// The manifest, its reconciliation and its undecided quarantine, from the authority's rows.
    fn imported_stage(&mut self, review: &Review, id: u64) -> Result<Stage, String> {
        let book = self.owner_book()?;
        let manifest = book
            .legacy
            .manifests
            .iter()
            .find(|m| m.id == id)
            .cloned()
            .ok_or("The authority holds no such manifest.")?;
        let report = reconcile(&review.backup, &book.legacy, id, live_numbers(&book))
            .ok_or("The authority holds no such manifest.")?;
        let open = manifest
            .entries
            .iter()
            .enumerate()
            .filter(|(n, e)| {
                e.status == EntryStatus::Quarantined
                    && !book
                        .legacy
                        .resolutions
                        .iter()
                        .any(|r| r.manifest == id && usize::try_from(r.entry) == Ok(*n))
            })
            .map(|(n, e)| {
                (
                    n,
                    source_record(&review.plan.source, &e.source).unwrap_or_default(),
                )
            })
            .collect();
        Ok(Stage::Imported {
            manifest,
            report,
            open,
        })
    }

    /// The Owner's decision on one quarantined entry of the imported file: keep it excluded,
    /// or import the corrected record (mapped by the core), with a reason. Never automatic.
    pub(crate) fn legacy_resolve(&mut self, terms: &ResolveTerms) -> Result<(), String> {
        self.require_owner()?;
        let review = self.legacy.take().ok_or("Import a legacy file first.")?;
        let result = self.resolve_in(&review, terms);
        let stage = match &result {
            Ok(id) => self.imported_stage(&review, *id),
            Err(_) => Err(String::new()),
        };
        self.legacy = Some(match stage {
            Ok(stage) => Review { stage, ..review },
            Err(_) => review,
        });
        result.map(|_| ())
    }

    fn resolve_in(&mut self, review: &Review, terms: &ResolveTerms) -> Result<u64, String> {
        let Stage::Imported { manifest, .. } = &review.stage else {
            return Err("Import the file before deciding its quarantine.".into());
        };
        let entry: usize = terms.entry.trim().parse().map_err(|_| "Unknown entry.")?;
        let quarantined = manifest
            .entries
            .get(entry)
            .filter(|e| e.status == EntryStatus::Quarantined)
            .ok_or("Not a quarantined entry.")?;
        let (decision, body) = match terms.decision.as_str() {
            "exclude" => (Decision::KeepExcluded, None),
            "import" => (
                Decision::ImportCorrected,
                Some(map_corrected_text(&quarantined.kind, &terms.record)?),
            ),
            _ => return Err("Choose a decision.".into()),
        };
        if terms.reason.trim().is_empty() {
            return Err("Give a reason for the decision.".into());
        }
        self.send(&Intent::ResolveLegacyQuarantine(Box::new(
            QuarantineDecision {
                manifest: manifest.id,
                entry: entry as u64,
                kind: quarantined.kind.clone(),
                legacy_id: quarantined.legacy_id.clone(),
                decision,
                body,
                reason: terms.reason.trim().to_owned(),
            },
        )))?;
        self.message = format!(
            "Recorded: {} {} {}.",
            quarantined.kind,
            quarantined.legacy_id,
            match decision {
                Decision::KeepExcluded => "kept excluded",
                Decision::ImportCorrected => "imported as corrected",
            }
        );
        Ok(manifest.id)
    }

    pub(crate) fn legacy_close(&mut self) {
        self.legacy = None;
    }
}

#[tauri::command(async)]
pub fn legacy_dry_run(state: State<'_, AppState>, q: String) -> Result<String, String> {
    with(&state, "settings", |trial| trial.legacy_dry_run(&q))
}

#[tauri::command(async)]
pub fn legacy_import_confirm(state: State<'_, AppState>) -> Result<String, String> {
    with(&state, "settings", Trial::legacy_import_confirm)
}

#[derive(serde::Deserialize)]
pub struct ResolveTerms {
    pub(crate) entry: String,
    pub(crate) decision: String,
    #[serde(default)]
    pub(crate) reason: String,
    #[serde(default)]
    pub(crate) record: String,
}

#[tauri::command(async)]
pub fn legacy_resolve(state: State<'_, AppState>, terms: ResolveTerms) -> Result<String, String> {
    with(&state, "settings", |trial| trial.legacy_resolve(&terms))
}

#[tauri::command(async)]
pub fn legacy_close(state: State<'_, AppState>) -> Result<String, String> {
    with(&state, "settings", |trial| {
        trial.legacy_close();
        Ok(())
    })
}
