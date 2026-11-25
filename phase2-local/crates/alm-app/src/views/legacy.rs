//! The "Import legacy backup" card (Owner only): files, the dry-run reconciliation, the
//! confirmation, and after the import the manifest and the quarantine.
use std::fmt::Write as _;

use alm_core::legacy::import::{EntryStatus, ImportManifest};
use alm_core::legacy::{LineStatus, Reconciliation};

use crate::gst_print::{esc, indian_digits};

pub enum LegacyStage<'a> {
    DryRun(&'a Reconciliation),
    /// The manifest, its report and its undecided quarantine (position, stored record).
    Imported(
        &'a ImportManifest,
        &'a Reconciliation,
        &'a [(usize, String)],
    ),
}

fn report(
    html: &mut String,
    r: &Reconciliation,
    open: Option<(&ImportManifest, &[(usize, String)])>,
) {
    let _ = write!(
        html,
        r#"<div class="help">Source SHA-256 <code>{}</code> · mapping {} · {} to import · {} already imported · {} quarantined · {} differences explained by quarantine · <b>{} unexplained</b></div>"#,
        esc(&r.digest),
        esc(&r.version),
        r.imported,
        r.already,
        r.quarantined.len(),
        r.explained(),
        r.unexplained()
    );
    for section in &r.sections {
        let _ = write!(
            html,
            r#"<div class="sectit" style="margin-top:10px">{}</div><div class="tbl-wrap" style="max-height:30vh"><table aria-label="{0}"><thead><tr><th>Control</th><th>Legacy file</th><th>Authority</th><th>Result</th></tr></thead><tbody>"#,
            esc(section.name)
        );
        for line in &section.lines {
            let (class, status) = match &line.status {
                LineStatus::Match => ("", "match".to_owned()),
                LineStatus::Explained => ("muted", "explained by quarantine".to_owned()),
                LineStatus::Unexplained(why) => ("neg", format!("UNEXPLAINED: {why}")),
            };
            let _ = write!(
                html,
                r#"<tr><td>{}</td><td>{}</td><td>{}</td><td class="{class}">{}</td></tr>"#,
                esc(&line.key),
                esc(line.legacy.as_deref().unwrap_or("—")),
                esc(line.authority.as_deref().unwrap_or("—")),
                esc(&status)
            );
        }
        html.push_str("</tbody></table></div>");
    }
    html.push_str(r#"<div class="sectit" style="margin-top:10px">Quarantine — unresolved</div><div class="tbl-wrap" style="max-height:40vh"><table aria-label="Quarantine"><thead><tr><th>Kind</th><th>Legacy ID</th><th>Reason</th><th>Owner decision</th></tr></thead><tbody>"#);
    if r.quarantined.is_empty() {
        html.push_str(
            r#"<tr><td colspan="4"><div class="empty">Nothing unresolved</div></td></tr>"#,
        );
    }
    match open {
        None => {
            for q in &r.quarantined {
                let _ = write!(
                    html,
                    "<tr><td>{}</td><td>{}</td><td>{}</td><td class=\"muted\">after the import</td></tr>",
                    esc(&q.kind),
                    esc(&q.legacy_id),
                    esc(&q.reason)
                );
            }
        }
        Some((manifest, open)) => {
            for (entry, record) in open {
                let Some(q) = manifest.entries.get(*entry) else {
                    continue;
                };
                let _ = write!(
                    html,
                    r#"<tr><td>{}</td><td>{}</td><td>{}</td><td><form data-action="legacy_resolve" data-payload="terms"><input type="hidden" name="entry" value="{entry}"><select class="inp" name="decision" aria-label="Decision for {1}"><option value="exclude">Keep excluded</option><option value="import">Import as corrected</option></select><input class="inp" name="reason" required placeholder="Reason" aria-label="Reason for {1}"><textarea class="inp" name="record" rows="4" aria-label="Corrected record for {1}" style="font-family:monospace;font-size:11px">{}</textarea><button class="btn sm" type="submit">Record decision</button></form></td></tr>"#,
                    esc(&q.kind),
                    esc(&q.legacy_id),
                    esc(&q.reason),
                    esc(record)
                );
            }
        }
    }
    html.push_str("</tbody></table></div>");
    html.push_str(r#"<div class="sectit" style="margin-top:10px">Quarantine — resolved</div><div class="tbl-wrap" style="max-height:30vh"><table aria-label="Resolved quarantine"><thead><tr><th>Kind</th><th>Legacy ID</th><th>Decision</th><th>Reason</th><th>Staged record</th></tr></thead><tbody>"#);
    if r.resolved.is_empty() {
        html.push_str(
            r#"<tr><td colspan="5"><div class="empty">No decision recorded</div></td></tr>"#,
        );
    }
    for (q, decided) in &r.resolved {
        let _ = write!(
            html,
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            esc(&q.kind),
            esc(&q.legacy_id),
            match decided.decision {
                alm_core::legacy::resolve::Decision::ImportCorrected => "imported as corrected",
                alm_core::legacy::resolve::Decision::KeepExcluded => "kept excluded",
            },
            esc(&decided.reason),
            esc(decided.authority_id.as_deref().unwrap_or("—"))
        );
    }
    html.push_str("</tbody></table></div>");
}

pub fn legacy_card(files: &[(String, u64)], review: Option<(&str, LegacyStage<'_>)>) -> String {
    let mut html = String::from(
        r#"<div class="card" style="margin-bottom:14px"><div class="sectit">📥 Import legacy backup (rehearsal)</div><div class="help">Development rehearsal with synthetic files only. Choose an HTML backup file (<code>ALM_backup_…json</code>): it is copied into this app's <code>legacy-import</code> folder first (files already there are listed too). A dry run reads and migrates it as the HTML's restore does and shows the reconciliation first; nothing is imported until you confirm. The import is staged: it posts no sale and issues no invoice or GST number.</div><div class="row" style="margin:10px 0"><button class="btn" type="button" data-action="file_pick" data-q="legacy">📂 Choose a legacy file…</button></div><div class="tbl-wrap"><table><thead><tr><th>Legacy file</th><th class="num">Size</th><th class="num">Actions</th></tr></thead><tbody>"#,
    );
    if files.is_empty() {
        html += r#"<tr><td colspan="3"><div class="empty">No legacy files in the legacy-import folder</div></td></tr>"#;
    }
    for (name, size) in files {
        let _ = write!(
            html,
            r#"<tr><td>{0}</td><td class="num">{1} bytes</td><td class="num"><button class="btn sm" type="button" data-action="legacy_dry_run" data-q="{0}" aria-label="Dry run {0}">Dry run</button></td></tr>"#,
            esc(name),
            indian_digits(*size)
        );
    }
    html += "</tbody></table></div>";
    match review {
        None => {}
        Some((name, LegacyStage::DryRun(r))) => {
            let _ = write!(
                html,
                r#"<div class="card" style="margin-top:10px;border-left:3px solid var(--warn)"><b>Dry run: {} (nothing imported)</b>"#,
                esc(name)
            );
            report(&mut html, r, None);
            html.push_str(r#"<div class="row" style="justify-content:flex-end;margin-top:10px;gap:8px"><button class="btn" type="button" data-action="legacy_close">Cancel</button><details style="display:inline-block"><summary class="btn primary">Import into the authority…</summary><button class="btn sm danger" type="button" data-action="legacy_import_confirm">Confirm import of this file</button></details></div></div>"#);
        }
        Some((name, LegacyStage::Imported(manifest, r, open))) => {
            let _ = write!(
                html,
                r#"<div class="card" style="margin-top:10px;border-left:3px solid var(--good)"><b>Imported: {} — manifest {}</b><div class="help">{} imported · {} already imported · {} quarantined</div><div class="sectit" style="margin-top:10px">Manifest</div><div class="tbl-wrap" style="max-height:30vh"><table aria-label="Import manifest"><thead><tr><th>Kind</th><th>Legacy ID</th><th>Authority ID</th><th>Status</th></tr></thead><tbody>"#,
                esc(name),
                manifest.id,
                manifest.count(EntryStatus::Imported),
                manifest.count(EntryStatus::Already),
                manifest.count(EntryStatus::Quarantined)
            );
            for e in &manifest.entries {
                let status = match e.status {
                    EntryStatus::Imported => "imported",
                    EntryStatus::Already => "already imported",
                    EntryStatus::Quarantined => "quarantined",
                };
                let _ = write!(
                    html,
                    "<tr><td>{}</td><td>{}</td><td>{}</td><td>{status}</td></tr>",
                    esc(&e.kind),
                    esc(&e.legacy_id),
                    esc(e.authority_id.as_deref().unwrap_or("—"))
                );
            }
            html.push_str("</tbody></table></div>");
            report(&mut html, r, Some((manifest, open)));
            html.push_str(r#"<div class="row" style="justify-content:flex-end;margin-top:10px"><button class="btn" type="button" data-action="legacy_close">Close</button></div></div>"#);
        }
    }
    html + "</div>"
}
