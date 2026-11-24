//! Settings: ALM_Business_Manager.html L8981–9400 (Appearance, Business Profile, Cloud Sync,
//! Google Sheets, Data Safety, Backup, Activity Log, Danger Zone). Device-local preferences come
//! from `alm_core::preferences`; the shared profile and default reorder level live in the shop
//! archive (`synthetic_read_model`), as `SettingScope` marks them. Hosted-service cards are inert.
use alm_core::Role;
use alm_core::confirmation::BalanceMode;
use alm_core::preferences::Theme;
use alm_core::synthetic_read_model::{BusinessProfile, SettingScope};

use super::denied;
use crate::gst_print::esc;

mod hosted;

pub struct SettingsView<'a> {
    pub role: Role,
    pub theme: Theme,
    pub balance: BalanceMode,
    pub privacy: bool,
    pub profile: &'a BusinessProfile,
    pub reorder_default: u32,
    /// The Backup & Restore card, placed where the HTML has it.
    pub backup: &'a str,
}

fn scope_note(scope: SettingScope) -> &'static str {
    match scope {
        SettingScope::DeviceLocal => "This device only — not saved in backups or the shop archive.",
        SettingScope::Shared => "Saved in the shop archive: included in backups and restores.",
    }
}

fn seg(action: &str, field: &str, options: &[(&str, &str, bool)]) -> String {
    let mut html = String::from(r#"<div class="seg" style="margin:6px 0">"#);
    for (value, label, on) in options {
        let class = if *on { "on" } else { "" };
        html += &format!(
            r#"<button class="{class}" type="button" data-action="{action}" data-q="{value}" aria-label="{field} {label}">{label}</button>"#
        );
    }
    html + "</div>"
}

fn field(label: &str, name: &str, value: &str, extra: &str) -> String {
    format!(
        r#"<div><label>{label} <input class="inp{}" name="{name}"{extra} value="{}"></label></div>"#,
        if extra.contains("inputmode") {
            " num"
        } else {
            ""
        },
        esc(value)
    )
}

fn profile_card(view: &SettingsView<'_>) -> String {
    let p = view.profile;
    let (fssai_on, gstin_on) = p.shows();
    let check = |on: bool| if on { " checked" } else { "" };
    let loading = if p.loading_charge().is_zero() {
        "0".to_owned()
    } else {
        p.loading_charge().to_rupee_text()
    };
    format!(
        r#"<div class="card" style="margin-bottom:14px"><div class="sectit">🏢 Business Profile</div><form data-action="settings_profile" data-payload="terms"><div class="f2">{}{}</div><div class="f2">{}{}</div><div class="field"><label>Address <textarea class="inp" name="address" rows="2">{}</textarea></label></div><div class="f2">{}{}</div><div class="row" style="gap:18px;margin-bottom:14px"><label style="display:flex;align-items:center;gap:7px;margin:0"><input type="checkbox" name="show_fssai" value="on"{}> Show FSSAI on bill</label><label style="display:flex;align-items:center;gap:7px;margin:0"><input type="checkbox" name="show_gstin" value="on"{}> Show GSTIN on bill</label></div><div class="f2">{}{}</div><div class="help">Profile version {} · saved by {}</div><button class="btn primary" type="submit">Save Profile</button><div class="help" style="margin-top:8px">{}</div></form></div>"#,
        field("Business Name", "name", &p.name, ""),
        field("Tagline", "tagline", &p.tagline, ""),
        field("Owner", "owner", &p.owner, ""),
        field("Mobile", "mobile", &p.mobile, ""),
        esc(&p.address),
        field("GSTIN", "gstin", &p.gstin, ""),
        field(
            "FSSAI License No.",
            "fssai",
            &p.fssai,
            r#" placeholder="e.g. 10012345678901""#
        ),
        check(fssai_on),
        check(gstin_on),
        field(
            "Default Loading ₹/pkt",
            "loading",
            &loading,
            r#" inputmode="decimal""#
        ),
        field(
            "Default Reorder Level",
            "reorder",
            &view.reorder_default.to_string(),
            r#" inputmode="numeric""#
        ),
        p.version,
        esc(if p.device.is_empty() {
            "—"
        } else {
            &p.device
        }),
        scope_note(SettingScope::Shared)
    )
}

pub fn settings_screen(view: &SettingsView<'_>) -> String {
    if view.role != Role::Owner {
        return denied("Settings");
    }
    let local = scope_note(SettingScope::DeviceLocal);
    let system = if view.theme == Theme::System {
        " Following this device's light or dark setting."
    } else {
        ""
    };
    let mut html = format!(
        r#"<div class="card" style="margin-bottom:14px"><div class="sectit">🎨 Appearance</div>{}<div class="help">This device only — appearance is a screen preference, so it is not synced to your other device. Printed bills and PDFs always come out on white paper regardless.{system} {local}</div></div>"#,
        seg(
            "settings_theme",
            "Theme",
            &[
                ("light", "Light", view.theme == Theme::Light),
                ("dark", "Dark", view.theme == Theme::Dark),
                ("system", "Match device", view.theme == Theme::System),
            ]
        )
    );
    html += &format!(
        r#"<div class="card" style="margin-bottom:14px"><div class="sectit">🧾 Estimate balance</div>{}<div class="help">Which balance bills and bill images show. {local}</div><div class="sectit" style="margin-top:10px">🔒 Privacy</div>{}<div class="help">{local}</div></div>"#,
        seg(
            "settings_balance",
            "Balance",
            &[
                (
                    "bill-date",
                    "As on bill date",
                    view.balance == BalanceMode::AsOnBillDate
                ),
                ("latest", "Latest", view.balance == BalanceMode::Latest),
            ]
        ),
        seg(
            "settings_privacy",
            "Privacy",
            &[
                ("off", "Show amounts", !view.privacy),
                ("on", "Hide amounts", view.privacy)
            ]
        )
    );
    html += &profile_card(view);
    html += &hosted::cloud_sync_card();
    html += &hosted::sheets_card();
    html += &hosted::data_safety_card();
    html += view.backup;
    html += &hosted::activity_card();
    html += &hosted::danger_card();
    html
}
