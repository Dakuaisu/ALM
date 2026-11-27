//! Shared markup pieces: KPI cards, modals, search options, notices, rupee amounts.
use std::fmt::Write as _;

use crate::gst_print::{esc, indian_digits};
use alm_core::Money;

/// KPI card markup from the HTML's kpi() (L2986–3001); ₹ values get the privacy class.
pub fn kpi(class: &str, label: &str, value: &str, sub: &str, icon: &str) -> String {
    let private = if value.starts_with('₹') {
        " privcard"
    } else {
        ""
    };
    format!(
        r#"<div class="kpi {class}{private}"><div class="ic">{icon}</div><div class="label">{label}</div><div class="val">{value}</div><div class="sub">{}</div></div>"#,
        esc(sub)
    )
}

/// A modal as the HTML's modal() builds it (L1519–1560), delivered as `<template id="screen-modal">`
/// for the glue to show; the × button carries the Rust command that closes it.
pub fn modal(icon: &str, title: &str, close: &str, large: bool, body: &str) -> String {
    let size = if large { " lg" } else { "" };
    let title = esc(title);
    format!(
        r#"<template id="screen-modal"><div class="modal{size}" role="dialog" aria-modal="true" aria-label="{title}"><h3>{icon} {title}<button class="x" type="button" aria-label="Close" data-action="{close}" style="margin-left:auto">×</button></h3><div class="body">{body}</div></div></template>"#
    )
}

/// One autocomplete option (`.autolist` row): picking it fills the named form fields and, with
/// `submit`, submits the form with that intent.
pub struct SearchOption<'a> {
    pub fill: &'a [(&'a str, &'a str)],
    pub label: &'a str,
    pub sub: &'a str,
    pub submit: Option<&'a str>,
}

fn query_escape(text: &str) -> String {
    let mut out = String::new();
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.~".contains(&byte) {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

pub fn search_options(options: &[SearchOption<'_>]) -> String {
    let mut html = String::new();
    for option in options {
        let fill = option
            .fill
            .iter()
            .map(|(name, value)| format!("{}={}", query_escape(name), query_escape(value)))
            .collect::<Vec<_>>()
            .join("&");
        let submit = option.submit.map_or_else(String::new, |intent| {
            format!(r#" data-submit="{}""#, esc(intent))
        });
        let sub = if option.sub.is_empty() {
            String::new()
        } else {
            format!("<small>{}</small>", esc(option.sub))
        };
        let name = if option.sub.is_empty() {
            option.label.to_owned()
        } else {
            format!("{}, {}", option.label, option.sub)
        };
        let _ = write!(
            html,
            r#"<div role="option" aria-selected="false" tabindex="-1" aria-label="{}" data-fill="{}"{submit}><span>{}</span>{sub}</div>"#,
            esc(&name),
            esc(&fill),
            esc(option.label)
        );
    }
    html
}

/// Typed text that must stay exactly as entered (URLs, emails, passwords, codes): mobile
/// keyboards must not capitalise, correct or spell-check it.
macro_rules! verbatim {
    ($kind:literal) => {
        concat!(
            $kind,
            r#" autocapitalize="off" autocorrect="off" spellcheck="false""#
        )
    };
}
pub const URL_INPUT: &str = verbatim!(r#" type="url" inputmode="url""#);
pub const EMAIL_INPUT: &str = verbatim!(r#" type="email" inputmode="email""#);
pub const PASSWORD_INPUT: &str = verbatim!(r#" type="password""#);
pub const CODE_INPUT: &str = verbatim!(r#" type="text""#);
pub const DIGITS_CODE_INPUT: &str = verbatim!(r#" type="text" inputmode="numeric""#);

/// A screen the role may not open (DEC-034 wording).
pub fn denied(screen: &str) -> String {
    format!(
        r#"<div class="card" role="alert"><h3>{}</h3><p>{} is the owner's.</p></div>"#,
        crate::refusals::OWNER_ONLY,
        esc(screen)
    )
}

pub fn money_r(money: Money) -> String {
    let rounded = money.rounded_rupees();
    let sign = if rounded < 0 { "-" } else { "" };
    format!("₹{sign}{}", indian_digits(rounded.unsigned_abs()))
}

pub fn notice(message: &str) -> String {
    if message.is_empty() {
        String::new()
    } else {
        format!(
            "<div class=\"help no-print\" style=\"margin-bottom:10px\">{}</div>",
            esc(message)
        )
    }
}
