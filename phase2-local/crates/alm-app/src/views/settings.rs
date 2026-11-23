//! Settings: ALM_Business_Manager.html L8981–9120 (Appearance, Business Profile). Device-local
//! preferences come from `alm_core::preferences`; the shared profile and default reorder level
//! live in the shop archive (`synthetic_read_model`), as `SettingScope` marks them.
use alm_core::Role;
use alm_core::confirmation::BalanceMode;
use alm_core::preferences::Theme;
use alm_core::synthetic_read_model::{BusinessProfile, SettingScope};

use super::denied;
use crate::gst_print::esc;

pub struct SettingsView<'a> {
    pub role: Role,
    pub theme: Theme,
    pub balance: BalanceMode,
    pub privacy: bool,
    pub profile: &'a BusinessProfile,
    pub reorder_default: u32,
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

pub fn settings_screen(view: &SettingsView<'_>) -> String {
    if view.role != Role::Owner {
        return denied("Settings");
    }
    let local = scope_note(SettingScope::DeviceLocal);
    let shared = scope_note(SettingScope::Shared);
    let mut html = format!(
        r#"<div class="card" style="margin-bottom:14px"><div class="sectit">🎨 Appearance</div>{}<div class="help">{local}</div></div>"#,
        seg(
            "settings_theme",
            "Theme",
            &[
                ("light", "Light", view.theme == Theme::Light),
                ("dark", "Dark", view.theme == Theme::Dark)
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
    let (fssai_on, gstin_on) = view.profile.shows();
    let check = |on: bool| if on { " checked" } else { "" };
    html += &format!(
        r#"<div class="card" style="margin-bottom:14px"><div class="sectit">🏢 Business Profile</div><form data-action="settings_profile" data-payload="terms"><div class="f2"><div><label>GSTIN <input class="inp" name="gstin" value="{}"></label></div><div><label>FSSAI License No. <input class="inp" name="fssai" value="{}"></label></div></div><div class="row" style="gap:18px;margin-bottom:14px"><label style="display:flex;align-items:center;gap:7px;margin:0"><input type="checkbox" name="show_fssai" value="on"{}> Show FSSAI on bill</label><label style="display:flex;align-items:center;gap:7px;margin:0"><input type="checkbox" name="show_gstin" value="on"{}> Show GSTIN on bill</label></div><div class="f2"><div><label>Default Reorder Level <input class="inp num" name="reorder" inputmode="numeric" value="{}"></label></div><div class="help">Profile version {} · saved by {}</div></div><button class="btn primary" type="submit">Save Profile</button><div class="help" style="margin-top:8px">{shared}</div></form></div>"#,
        esc(&view.profile.gstin),
        esc(&view.profile.fssai),
        check(fssai_on),
        check(gstin_on),
        view.reorder_default,
        view.profile.version,
        esc(if view.profile.device.is_empty() {
            "—"
        } else {
            &view.profile.device
        })
    );
    html
}
