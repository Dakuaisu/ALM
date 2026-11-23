//! Rust-rendered screen fragments. HTML source: ALM_Business_Manager.html L329–352, L1994–2017.
use std::fmt::Write as _;

use crate::gst_print::{esc, indian_digits};
use alm_core::Money;

mod backup;
mod bill;
mod cashbook;
mod counter;
mod crates;
mod customers;
mod dashboard;
mod drafts;
mod godown;
mod gst;
mod gst_sales;
mod icons;
mod order;
mod products;
mod purchases;
mod ratelist;
mod receipt;
mod reorder;
mod reports;
mod sale_edit;
mod sales;
mod settings;
mod stock;
mod suppliers;

pub use backup::{RestoredSummary, backup_section};
pub use bill::{BillView, bill_body, bill_screen};
pub use cashbook::{CashBookView, cash_system_card, cashbook_screen};
pub use counter::counter_screen;
pub use crates::{CrateForm, crates_screen};
pub use customers::{CustomersState, customers_screen};
pub use dashboard::dashboard_screen;
pub use drafts::{ReviewView, customer, posting_review, review_screen};
pub use godown::godown_screen;
pub use gst::gst_screen;
pub use gst_sales::gst_sales_screen;
pub use order::{OrderState, order_screen};
pub use products::{ProductsState, product_form, products_screen};
pub use purchases::purchases_screen;
pub use ratelist::ratelist_screen;
pub use receipt::{receipt_customer_options, receipt_screen};
pub use reorder::reorder_screen;
pub use reports::reports_screen;
pub use sale_edit::sale_edit_form;
pub use sales::{Filter, SalesFilter, sales_screen};
pub use settings::{SettingsView, settings_screen};
pub use stock::{StockState, stock_screen};
pub use suppliers::suppliers_screen;

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
        let _ = write!(
            html,
            r#"<div role="option" data-fill="{}"{submit}><span>{}</span>{sub}</div>"#,
            esc(&fill),
            esc(option.label)
        );
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
