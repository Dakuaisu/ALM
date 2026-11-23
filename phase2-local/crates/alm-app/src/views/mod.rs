//! Rust-rendered screen fragments. HTML source: ALM_Business_Manager.html L329–352, L1994–2017.
use std::fmt::Write as _;

use crate::gst_print::esc;

mod drafts;
mod gst;

pub use drafts::{DraftView, ReviewView, draft_screen, review_screen};
pub use gst::gst_screen;

pub fn tabs(active: &str) -> String {
    let mut html =
        String::from("<div class=\"row no-print\" style=\"gap:8px;margin-bottom:12px\">");
    for (tab, label) in [
        ("draft", "Manager draft"),
        ("review", "Owner review"),
        ("gst", "GST invoice"),
    ] {
        let class = if tab == active { "btn primary" } else { "btn" };
        let _ = write!(
            html,
            "<button class=\"{class}\" data-action=\"screen\" data-tab=\"{tab}\">{label}</button>"
        );
    }
    html + "</div>"
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
