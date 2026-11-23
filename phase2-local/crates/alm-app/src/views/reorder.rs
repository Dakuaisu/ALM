//! Reorder Planner: ALM_Business_Manager.html L8308–8429 (VIEWS.reorder, suggestQty,
//! copyReorder). Rows, suggested quantities, best supplier and estimated cost come from
//! `ConfirmationFixture::open_reorder_planner`; the Copy List text (clipboard) from `copy_reorder_list`.
use std::fmt::Write as _;

use alm_core::Role;
use alm_core::confirmation::{ConfirmationFixture, ReorderError, ReorderStatus};

use super::denied;
use crate::gst_print::{esc, indian_digits, inr};

pub fn reorder_screen(fixture: &ConfirmationFixture, role: Role) -> Result<String, ReorderError> {
    let rows = match fixture.open_reorder_planner(role) {
        Ok(rows) => rows,
        Err(ReorderError::AccessDenied) => return Ok(denied("Reorder")),
        Err(error) => return Err(error),
    };
    let mut html = String::new();
    if rows.is_empty() {
        return Ok(
            html + r#"<div class="card"><div class="empty"><div class="big">✅</div><div>Everything is well stocked! No reorders needed right now.</div></div></div>"#
        );
    }
    let _ = write!(
        html,
        r#"<div class="card" style="margin-bottom:14px;border-left:3px solid var(--warn)"><b>⚠️ {} item(s) at or below reorder level.</b><div class="help" style="margin-top:4px">Suggested quantity brings you back to ~3× reorder level. 'Best Supplier' is the cheapest buy rate from your comparison sheet.</div></div>"#,
        rows.len()
    );
    html += r#"<div class="tbl-wrap" style="max-height:62vh"><table><thead><tr><th>Item</th><th class="num">On Hand</th><th class="num">Reorder</th><th class="num">Suggested Qty</th><th>Best Supplier</th><th class="num">Est. Cost</th><th>Status</th></tr></thead><tbody>"#;
    for row in rows {
        let supplier = match &row.best_supplier {
            Some(name) => format!(
                r#"<span><b class="pos">{}</b> @ ₹{}</span>"#,
                esc(name),
                inr(row.unit_rate)
            ),
            None => format!(r#"<span class="muted">buy ₹{}</span>"#, inr(row.unit_rate)),
        };
        let (out, tag) = match row.status {
            ReorderStatus::Out => (" neg", r#"<span class="tag out">OUT</span>"#),
            ReorderStatus::Low => ("", r#"<span class="tag low">LOW</span>"#),
        };
        let _ = write!(
            html,
            r#"<tr><td><b>{}</b><div class="muted" style="font-size:11px">{}</div></td><td class="num{out}">{}</td><td class="num">{}</td><td class="num" style="font-weight:700;color:var(--accent)">{} pkt</td><td>{supplier}</td><td class="num">{}</td><td>{tag}</td></tr>"#,
            esc(&row.item),
            esc(&row.brand),
            indian_digits(u64::from(row.on_hand)),
            indian_digits(u64::from(row.reorder_level)),
            indian_digits(row.suggested_quantity.get()),
            row.estimated_cost_display()
        );
    }
    Ok(html + "</tbody></table></div>")
}
