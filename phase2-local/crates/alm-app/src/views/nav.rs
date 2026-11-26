//! Sidebar navigation and per-screen header actions (HTML NAV, L1896–1975).
use std::fmt::Write as _;

use super::icons;
use crate::gst_print::esc;

/// Sidebar sections follow the HTML NAV (L1896–1975); only ported screens are listed.
const NAV: &[(&str, &[(&str, &str)])] = &[
    (
        "Daily",
        &[
            ("counter", "Counter"),
            ("bill", "New Bill"),
            ("review", "Owner review"),
            ("sales", "Sales"),
            ("receipt", "Receive Payment"),
            ("purchases", "Purchase Orders"),
            ("gstsales", "GST Sales"),
            ("expenses", "Cash Book"),
        ],
    ),
    (
        "Manage",
        &[
            ("customers", "Customers"),
            ("products", "Products"),
            ("crates", "Crates"),
            ("suppliers", "Suppliers"),
            ("stock", "Stock"),
            ("godown", "Godown"),
            ("reorder", "Reorder"),
            ("order", "Order Planner"),
        ],
    ),
    (
        "More",
        &[
            ("ratelist", "Rate List"),
            ("dashboard", "Dashboard"),
            ("reports", "Reports"),
            ("settings", "Settings"),
        ],
    ),
];

pub fn tabs(active: &str) -> String {
    let (title, subtitle) = match active {
        "counter" => ("Counter", "What to do next · counts only"),
        "customers" => ("Customers", "Contacts & credit ledger (udhaar)"),
        "expenses" => ("Cash Book", "Daily income & expenses"),
        "dashboard" => ("Dashboard", "Sales, collections and udhaar"),
        "reports" => ("Reports", "Business performance"),
        "settings" => ("Settings", "Business profile · Backup"),
        "reorder" => ("Reorder Planner", "Items running low + cheapest supplier"),
        "stock" => ("Stock", "Live inventory (billing stock)"),
        "products" => ("Products", "Price list, margins & supplier comparison"),
        "suppliers" => ("Suppliers", "Where you buy stock from"),
        "crates" => ("Crates", "Reusable mixed-crate templates for quick billing"),
        "ratelist" => (
            "Rate List",
            "Tick the brands and items you want, then print or send",
        ),
        "purchases" => (
            "Purchase Orders",
            "Order stock — inventory updates only when goods are received",
        ),
        "order" => (
            "Order Planner",
            "Pick items into the cart, then create the purchase",
        ),
        "godown" => ("Godown", "Simple list of what's stored in the godown"),
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
        "rateprint" => ("Rate List", "Print or save as PDF"),
        "gstsales" => (
            "GST Sales",
            "Standalone GST tax invoices — does NOT change stock or customer balance",
        ),
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
    if active == "gstsales" {
        html += r#"<template id="screen-actions"><button type="button" class="btn primary" data-action="gst_new">＋ New GST Invoice</button></template>"#;
    }
    if active == "gst" {
        html += r#"<template id="screen-actions"><button type="button" class="btn ghost" data-action="screen" data-tab="gstsales">← Back to list</button></template>"#;
    }
    if active == "expenses" {
        html += r#"<template id="screen-actions"><button type="button" class="btn primary" data-action="cash_new">＋ Add Day Entry</button></template>"#;
    }
    if active == "crates" {
        html += r#"<template id="screen-actions"><button type="button" class="btn primary" data-action="crate_new">＋ New Crate</button></template>"#;
    }
    if active == "products" {
        html += r#"<template id="screen-actions"><button type="button" class="btn primary" data-action="product_new">＋ Add Product</button></template>"#;
    }
    if active == "reports" {
        html += r#"<template id="screen-actions"><button type="button" class="btn" data-action="share_file" data-command="export_file" data-q="csv">⬇ Sales CSV</button></template>"#;
    }
    if active == "godown" {
        html += r#"<template id="screen-actions"><button type="button" class="btn" data-copy="godown_copy_text">📋 Copy List</button></template>"#;
    }
    if active == "ratelist" {
        html += r#"<template id="screen-actions"><button type="button" class="btn" data-copy="rate_copy_text">📋 Copy</button><button type="button" class="btn" data-action="screen" data-tab="rateprint">🖨️ Print</button><button type="button" class="btn" data-action="screen" data-tab="rateprint">📄 Save PDF</button><button type="button" class="btn wa" data-action="open_url" data-command="rate_whatsapp_url">💬 WhatsApp</button></template>"#;
    }
    if active == "stock" {
        html += r#"<template id="screen-actions"><button type="button" class="btn" data-action="screen" data-tab="reorder">🔁 Reorder List</button></template>"#;
    }
    if active == "reorder" {
        html += r#"<template id="screen-actions"><button type="button" class="btn" data-copy="reorder_copy_text">📋 Copy List</button></template>"#;
    }
    if active == "bill" {
        html += r#"<template id="screen-actions"><button type="button" class="btn ghost" data-action="bill_fresh">＋ Fresh Bill</button></template>"#;
    }
    html
}

/// The sidebar's account line and authority line, from the session: the signed-in account and
/// its role (or "Not signed in"), and the REST URL's host or the built-in simulation.
pub fn session_header(signed_in: Option<(&str, alm_core::Role)>, rest: Option<&str>) -> String {
    let account = signed_in.map_or_else(
        || "Not signed in".to_owned(),
        |(account, role)| {
            let role = match role {
                alm_core::Role::Owner => "Owner",
                alm_core::Role::Manager => "Manager",
            };
            format!("{account} · {role}")
        },
    );
    let (authority, dot) = rest.map_or_else(
        || ("Built-in simulation".to_owned(), "syncdot off"),
        |url| (format!("REST · {}", url_host(url)), "syncdot ok"),
    );
    format!(
        r#"<template id="screen-session" data-account="{}" data-authority="{}" data-dot="{dot}"></template>"#,
        esc(&account),
        esc(&authority)
    )
}

/// The host (and port) of `url`: no scheme, user, path or query.
fn url_host(url: &str) -> &str {
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    let authority = rest.split(['/', '?', '#']).next().unwrap_or(rest);
    authority
        .rsplit_once('@')
        .map_or(authority, |(_, host)| host)
}
