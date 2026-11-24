//! Settings cards that need hosted services (Supabase sync, Google Sheets, folder auto-save,
//! the activity log, Clear All Data). Shown with the HTML's fields and buttons, disabled.
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

pub fn activity_card() -> String {
    format!(
        r#"<div class="card" style="margin-bottom:14px"><div class="sectit">🕒 Activity Log</div><div class="help" style="margin-bottom:8px">The permanent action log is kept by the hosted service. {INERT}</div><div class="empty">No actions logged on this device</div></div>"#
    )
}

pub fn danger_card() -> String {
    format!(
        r#"<div class="card" style="margin-bottom:14px;border-color:#f0cdc8;background:var(--bad-bg)"><div class="sectit" style="color:var(--bad)">⚠️ Danger Zone</div><div class="help">Posted records are kept by the hosted authority; this device cannot erase them.</div><div class="row">{}</div></div>"#,
        disabled_button("danger", "🗑 Clear All Data")
    )
}
