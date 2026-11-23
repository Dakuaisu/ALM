//! GST Sales list: ALM_Business_Manager.html L5232–5330 (VIEWS.gstsales). Rows, order, numbers and
//! totals come from `gst_bill_list`; the form lives in `gst_sales/form.rs`.
use std::fmt::Write as _;

use alm_core::Role;
use alm_core::confirmation::{ConfirmationFixture, GstBillError, GstBillFields};

use super::{denied, money_r};
use crate::gst_print::esc;

mod form;

pub fn gst_sales_screen(
    fixture: &ConfirmationFixture,
    role: Role,
    form: Option<&GstBillFields>,
) -> Result<String, GstBillError> {
    if role != Role::Owner {
        return Ok(denied("GST Sales"));
    }
    if let Some(fields) = form {
        return form::gst_form(fixture, role, fields);
    }
    let rows = fixture.gst_bill_list(role)?;
    let mut html = String::from(
        r#"<div style="max-width:1050px;margin:0 auto"><div class="help" style="margin-bottom:12px">Manual GST tax invoices for customers who need a GST bill (e.g. combining several estimates). These are SEPARATE from your normal Sales — creating one here never changes stock, customer balance (udhaar) or sales reports.</div>"#,
    );
    if rows.is_empty() {
        return Ok(
            html + r#"<div class="card" style="text-align:center;color:var(--mut2);padding:42px">No GST invoices yet — click ＋ New GST Invoice to create one.</div></div>"#
        );
    }
    html += r#"<div class="tbl-wrap" style="max-height:64vh"><table><thead><tr><th>Date</th><th>GST Invoice No</th><th>Customer</th><th class="num">Items</th><th class="num">Total</th><th>Actions</th></tr></thead><tbody>"#;
    for row in rows {
        let id = row.id.0;
        let (style, tag, edit) = if row.cancelled {
            (
                "opacity:.5",
                r#"<span class="tag" style="background:#fde8e8;color:#b91c1c;margin-left:6px;font-size:10px">CANCELLED</span>"#,
                String::new(),
            )
        } else {
            (
                "",
                "",
                format!(
                    r#" <button class="btn sm" data-action="gst_edit" data-id="{id}">Edit</button> <details style="display:inline-block"><summary class="btn sm">Cancel</summary><button class="btn sm" data-action="gst_cancel" data-id="{id}">Cancel this GST invoice (number stays used)</button></details>"#
                ),
            )
        };
        let _ = write!(
            html,
            r#"<tr style="{style}"><td>{}</td><td><b>{}</b>{tag}</td><td>{}</td><td class="num">{}</td><td class="num">{}</td><td class="num"><button class="btn sm" data-action="gst_view" data-id="{id}">View</button>{edit} <details style="display:inline-block"><summary class="btn sm danger">Delete</summary><button class="btn sm danger" data-action="gst_delete" data-id="{id}">Delete permanently</button></details></td></tr>"#,
            row.date.display(),
            esc(&row.number),
            esc(&row.customer),
            row.items,
            money_r(row.total)
        );
    }
    Ok(html + "</tbody></table></div></div>")
}
