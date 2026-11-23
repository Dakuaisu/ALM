//! Rust-rendered screen fragments. HTML source: ALM_Business_Manager.html L329–352, L1994–2017.
use std::fmt::Write as _;

use crate::gst_print::{esc, indian_digits};
use alm_core::Money;

mod bill;
mod counter;
mod drafts;
mod gst;
mod icons;
mod receipt;
mod sales;

pub use bill::{BillView, bill_screen};
pub use counter::counter_screen;
pub use drafts::{ReviewView, customer, posting_review, review_screen};
pub use gst::gst_screen;
pub use receipt::receipt_screen;
pub use sales::{Filter, SalesFilter, sales_screen};

/// Sidebar sections follow the HTML NAV (L1896–1975); only ported screens are listed.
const NAV: &[(&str, &[(&str, &str)])] = &[(
    "Daily",
    &[
        ("counter", "Counter"),
        ("bill", "New Bill"),
        ("review", "Owner review"),
        ("sales", "Sales"),
        ("receipt", "Receive Payment"),
        ("gst", "GST invoice"),
    ],
)];

pub fn tabs(active: &str) -> String {
    let (title, subtitle) = match active {
        "counter" => ("Counter", "What to do next · counts only"),
        "review" => (
            "Owner review",
            "Review a saved Manager draft before local simulation",
        ),
        "sales" => ("Sales", "All cash memos & sale records"),
        "receipt" => (
            "Receive Payment",
            "Record a separately dated customer receipt",
        ),
        "gst" => ("GST invoice", "Standalone synthetic invoice"),
        _ => ("New Bill", "Provisional only; not posted"),
    };
    let mut html = format!(
        r#"<template id="screen-nav" data-title="{title}" data-subtitle="{}">"#,
        esc(subtitle)
    );
    for (section, items) in NAV {
        let _ = write!(html, r#"<div class="navsec">{section}</div>"#);
        for (tab, label) in *items {
            let (class, current) = if *tab == active {
                ("navitem on", " aria-current=\"page\"")
            } else {
                ("navitem", "")
            };
            let _ = write!(
                html,
                "<button type=\"button\" class=\"{class}\"{current} data-action=\"screen\" data-tab=\"{tab}\"><span class=\"ic\">{}</span><span>{label}</span></button>",
                icons::nav(tab)
            );
        }
    }
    html += "</template>";
    if active == "bill" {
        html += r#"<template id="screen-actions"><button type="button" class="btn ghost" data-action="bill_fresh">＋ Fresh Bill</button></template>"#;
    }
    html
}

pub fn denied(screen: &str) -> String {
    format!(
        r#"<div class="card" role="alert"><h3>Access denied</h3><p>Only the Owner can open {}.</p></div>"#,
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
