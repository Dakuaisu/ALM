//! Settings cards that need hosted services (Supabase sync, Google Sheets, folder auto-save,
//! Clear All Data), shown with the HTML's fields and buttons, disabled; and the activity log,
//! read from the authority's rows.
use crate::gst_print::esc;

const INERT: &str = "Needs the hosted service; not available in this development build.";

fn disabled_input(label: &str, placeholder: &str) -> String {
    format!(
        r#"<div><label>{label} <input class="inp" disabled placeholder="{placeholder}"></label></div>"#
    )
}

fn disabled_button(class: &str, label: &str) -> String {
    format!(
        r#"<button class="btn {class}" type="button" disabled title="{INERT}">{label}</button>"#
    )
}

pub fn cloud_sync_card() -> String {
    format!(
        r#"<div class="card" style="margin-bottom:14px" aria-disabled="true"><div class="sectit">☁ Cloud Sync — Supabase (primary)</div><p class="help" style="margin-bottom:10px">Record-by-record sync to the shop's Supabase project. {INERT}</p><div class="field">{}</div><div class="f2">{}{}</div><label style="display:flex;align-items:center;gap:8px;margin-bottom:10px"><input type="checkbox" disabled> Enable cloud sync on this device</label><div class="row">{}{}{}{}</div><div class="help" style="margin-top:8px">Status: not enabled — posting stays a local simulation.</div></div>"#,
        disabled_input("Project URL", "https://xxxx.supabase.co"),
        disabled_input("Anon key", "anon public key (Project Settings → API)"),
        disabled_input("Secret token", "secret token (the one inside the SQL)"),
        disabled_button("primary", "Save"),
        disabled_button("", "⬆ First-time full upload"),
        disabled_button("", "🔄 Sync now"),
        disabled_button("danger", "⬇ Rebuild from cloud"),
    )
}

pub fn sheets_card() -> String {
    format!(
        r#"<div class="card" style="margin-bottom:14px" aria-disabled="true"><div class="sectit">☁️ Google Sheets Sync</div><p class="help" style="margin-bottom:10px">Paste the Web-App URL from your Google Apps Script. {INERT}</p><div class="field">{}</div><div class="row">{}{}{}</div><div class="help" style="margin-top:8px">Not synced yet.</div></div>"#,
        disabled_input(
            "Apps Script Web App URL",
            "https://script.google.com/macros/s/..../exec"
        ),
        disabled_button("", "Save URL"),
        disabled_button("primary", "⬆ Sync Now (push)"),
        disabled_button("", "⬇ Pull from Sheet"),
    )
}

pub fn data_safety_card() -> String {
    let dot = |ok: bool| {
        format!(
            r#"<span style="display:inline-block;width:9px;height:9px;border-radius:50%;margin-right:9px;vertical-align:middle;background:var(--{})"></span>"#,
            if ok { "good" } else { "mut2" }
        )
    };
    format!(
        r#"<div class="card" style="margin-bottom:14px"><div class="sectit">🛟 Data Safety — your data is saved in multiple places</div><div style="display:grid;gap:8px;margin-bottom:10px;font-size:13px"><div>{}This device — app database (SQLite drafts, outbox and projection) <b class="pos">✓ active</b></div><div>{}Full backups in this app's data folder <b class="pos">✓ available</b></div><div>{}Auto-save to a folder on disk — <b class="muted">not available</b></div><div>{}Google Sheet (online, any device) — <b class="muted">not set</b></div></div><div class="row">{}</div></div>"#,
        dot(true),
        dot(true),
        dot(false),
        dot(false),
        disabled_button("primary", "📁 Connect Backup Folder"),
    )
}

/// The authority's action log (`logAct`), newest first, the last 200 (Owner only, OPEN-004).
pub fn activity_card(
    role: alm_core::Role,
    rows: &[alm_core::projection::book::ActivityRow],
    utc_offset_minutes: i32,
) -> String {
    use std::fmt::Write as _;
    if role != alm_core::Role::Owner {
        return r#"<div class="card" style="margin-bottom:14px"><div class="sectit">🕒 Activity Log</div><div class="help">Owner only.</div></div>"#.into();
    }
    let shown: Vec<_> = rows.iter().rev().take(200).collect();
    let mut html = format!(
        r#"<div class="card" style="margin-bottom:14px"><div class="sectit">🕒 Activity Log</div><div class="help" style="margin-bottom:8px">Last {} recorded actions (bills, payments, goods received, adjustments, sync…). Newest first — this log is permanent; the authority records each action with the device and account it came from.</div><div class="tbl-wrap" style="max-height:40vh"><table aria-label="Activity log"><thead><tr><th>When</th><th>Device</th><th>Action</th><th>Details</th></tr></thead><tbody>"#,
        shown.len()
    );
    if shown.is_empty() {
        html += r#"<tr><td colspan="4"><div class="empty">No actions logged yet</div></td></tr>"#;
    }
    for row in shown {
        let when = alm_core::projection::activity_time(row.at_ms, utc_offset_minutes);
        let _ = write!(
            html,
            r#"<tr><td class="muted">{}</td><td class="muted" title="account {}">{}</td><td>{}</td><td>{}</td></tr>"#,
            esc(&when),
            esc(&row.account),
            esc(&row.device),
            esc(&row.kind),
            esc(&row.message())
        );
    }
    html + "</tbody></table></div></div>"
}

pub fn danger_card() -> String {
    format!(
        r#"<div class="card" style="margin-bottom:14px;border-color:#f0cdc8;background:var(--bad-bg)"><div class="sectit" style="color:var(--bad)">⚠️ Danger Zone</div><div class="help">Posted records are kept by the hosted authority; this device cannot erase them.</div><div class="row">{}</div></div>"#,
        disabled_button("danger", "🗑 Clear All Data")
    )
}
