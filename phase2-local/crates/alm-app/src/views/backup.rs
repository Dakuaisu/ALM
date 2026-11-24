//! Backup & Restore section of Settings: ALM_Business_Manager.html L9273–9330.
use std::fmt::Write as _;

use crate::gst_print::{esc, indian_digits};

/// Counts read back from a restored archive (no amounts).
pub struct RestoredSummary {
    pub products: usize,
    pub customers: usize,
    pub sales: usize,
    pub receipts: usize,
    pub gst_bills: usize,
    pub reorder_default: u32,
    pub profile_version: u64,
    pub gstin: String,
}

pub fn backup_section(
    backups: &[(String, u64)],
    restored: Option<(&str, &RestoredSummary)>,
) -> String {
    let mut html = String::from(
        r#"<div class="card" style="margin-bottom:14px"><div class="sectit">💾 Backup &amp; Restore</div><div class="help">A full backup holds every record, standalone GST bill and shared setting. Backups are saved in this app's data folder. Restoring checks the backup at a fresh place first, so a bad file cannot damage your data; a good one replaces the shop settings and the posted data is reloaded from the authority.</div><div class="row" style="margin:10px 0"><button class="btn primary" type="button" data-action="backup_create">⬇ Full Backup (.json)</button></div><div class="tbl-wrap"><table><thead><tr><th>Backup</th><th class="num">Size</th><th class="num">Actions</th></tr></thead><tbody>"#,
    );
    if backups.is_empty() {
        html += r#"<tr><td colspan="3"><div class="empty">No backups yet</div></td></tr>"#;
    }
    for (name, size) in backups {
        let _ = write!(
            html,
            r#"<tr><td>{0}</td><td class="num">{1} bytes</td><td class="num"><details style="display:inline-block"><summary class="btn sm">⬆ Restore</summary><button class="btn sm danger" type="button" data-action="backup_restore" data-q="{0}" aria-label="Restore {0}">Restore this backup</button></details></td></tr>"#,
            esc(name),
            indian_digits(*size)
        );
    }
    html += "</tbody></table></div>";
    if let Some((name, s)) = restored {
        let _ = write!(
            html,
            r#"<div class="card" style="margin-top:10px;border-left:3px solid var(--good)"><b>Restored archive: {}</b><div class="help">{} products · {} customers · {} sales · {} receipts · {} GST bills · default reorder {} · profile v{} {}</div><div class="help">Settings replaced from this backup; posted data reloaded from the authority.</div></div>"#,
            esc(name),
            s.products,
            s.customers,
            s.sales,
            s.receipts,
            s.gst_bills,
            s.reorder_default,
            s.profile_version,
            esc(&s.gstin)
        );
    }
    html + r#"<div class="row" style="margin-top:12px;gap:8px"><button class="btn" type="button" data-action="share_file" data-command="export_file" data-q="xls">📊 Export Excel (.xls)</button><button class="btn" type="button" data-action="share_file" data-command="export_file" data-q="csv">⬇ Sales CSV</button></div></div>"#
}
