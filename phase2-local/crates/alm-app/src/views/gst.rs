//! Standalone GST fragment. HTML source: ALM_Business_Manager.html L4989–5134.
use crate::gst_print::esc;

pub fn gst_screen(invoice_html: &str, print_title: &str) -> String {
    let title = esc(print_title);
    format!(
        "<div class=\"help no-print\" style=\"margin-bottom:10px\">Standalone GST tax invoice — not linked to stock or dues.</div>\
         <div class=\"row no-print\" style=\"gap:8px;margin-bottom:10px\"><button class=\"btn\" data-action=\"print\" data-title=\"{title}\">🖨️ Print</button><button class=\"btn\" data-action=\"print\" data-title=\"{title}\">📄 Save PDF</button><button class=\"btn\" data-action=\"share\">Share</button></div>\
         <div id=\"printable\">{invoice_html}</div>"
    )
}
