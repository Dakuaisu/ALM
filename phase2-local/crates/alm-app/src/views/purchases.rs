//! Purchase Orders: ALM_Business_Manager.html L6297–6420 (VIEWS.purchases) and the Receive Goods
//! form with close-remaining (L6760–6950, inline). Rows, status, progress and value come from
//! `purchase_order_list`; receive rows from `open_purchase_receive_form`.
use std::fmt::Write as _;

use alm_core::confirmation::{
    ConfirmationFixture, PurchaseError, PurchaseOrderId, PurchaseOrderStatus,
};
use alm_core::{BillDate, Role};

use super::{denied, kpi, money_r};
use crate::gst_print::{esc, indian_digits};

fn status(status: PurchaseOrderStatus) -> (&'static str, &'static str) {
    match status {
        PurchaseOrderStatus::Pending => ("low", "Pending"),
        PurchaseOrderStatus::PartiallyReceived => ("credit", "Partially Received"),
        PurchaseOrderStatus::Completed => ("ok", "Completed"),
        PurchaseOrderStatus::Cancelled => ("out", "Cancelled"),
    }
}

pub fn purchases_screen(
    fixture: &ConfirmationFixture,
    role: Role,
    receiving: Option<PurchaseOrderId>,
    today: BillDate,
) -> Result<String, String> {
    let list = match fixture.purchase_order_list(role) {
        Ok(list) => list,
        Err(PurchaseError::AccessDenied) => return Ok(denied("Purchase Orders")),
        Err(error) => return Err(error.to_string()),
    };
    let mut html = String::from(r#"<div class="kpis" style="margin-bottom:14px">"#);
    html += &kpi(
        "b",
        "Open POs",
        &list.open.to_string(),
        "awaiting goods",
        "📦",
    );
    html += &kpi(
        "a",
        "All POs",
        &list.rows.len().to_string(),
        "on file",
        "🧾",
    );
    // DEC-034: suppliers, values, printing and closing are the Owner's; a Manager receives
    // quantities only.
    let owner = role == Role::Owner;
    if owner {
        html += &kpi(
            "w",
            "Suppliers",
            &list.suppliers.to_string(),
            "on file",
            "🏭",
        );
    }
    html += "</div>";
    if let Some(id) = receiving {
        html += &receive_form(fixture, role, id, today)?;
    }
    if list.rows.is_empty() {
        return Ok(
            html + r#"<div class="card" style="text-align:center;color:var(--mut2);padding:42px">No purchase orders yet — plan one in the Order Planner. Stock increases only when you receive the goods.</div>"#
        );
    }
    html += if owner {
        r#"<div class="tbl-wrap" style="max-height:56vh"><table><thead><tr><th>Date</th><th>PO No</th><th>Supplier</th><th>Status</th><th class="num">Recv / Ordered</th><th class="num">Value</th><th class="num">Actions</th></tr></thead><tbody>"#
    } else {
        r#"<div class="tbl-wrap" style="max-height:56vh"><table><thead><tr><th>Date</th><th>PO No</th><th>Supplier</th><th>Status</th><th class="num">Recv / Ordered</th><th class="num">Actions</th></tr></thead><tbody>"#
    };
    for row in &list.rows {
        let (class, label) = status(row.status);
        let id = row.id.index();
        let print = if owner {
            format!(
                r#"<button class="btn sm" data-action="print_open" data-q="poprint" data-id="{}">🖨 Print</button> "#,
                row.selection.number
            )
        } else {
            String::new()
        };
        let receive = if row.status.can_receive() {
            format!(
                r#"<button class="btn sm primary" data-action="po_receive" data-id="{id}">Receive</button>"#
            )
        } else {
            String::new()
        };
        let close = if owner && row.status.can_receive() {
            format!(
                r#" <details style="display:inline-block"><summary class="btn sm danger">Close remaining</summary><button class="btn sm danger" data-action="po_close" data-id="{id}">Close {} unreceived (no stock change)</button></details>"#,
                indian_digits(row.remaining)
            )
        } else {
            String::new()
        };
        let actions = print + &receive + &close;
        let value = if owner {
            format!(r#"<td class="num">{}</td>"#, money_r(row.value))
        } else {
            String::new()
        };
        let _ = write!(
            html,
            r#"<tr><td>{}</td><td><b>{}</b></td><td>{}</td><td><span class="tag {class}">{label}</span></td><td class="num">{} / {}</td>{value}<td class="num">{actions}</td></tr>"#,
            row.date.display(),
            esc(&row.number),
            esc(&row.supplier),
            indian_digits(row.received),
            indian_digits(row.ordered),
        );
    }
    Ok(html + "</tbody></table></div>")
}

fn receive_form(
    fixture: &ConfirmationFixture,
    role: Role,
    id: PurchaseOrderId,
    today: BillDate,
) -> Result<String, String> {
    let form = fixture
        .open_purchase_receive_form(role, id, today)
        .map_err(|error| error.to_string())?;
    let owner = role == Role::Owner;
    let rate_head = if owner {
        r#"<th class="num">Rate ₹</th>"#
    } else {
        ""
    };
    let mut html = format!(
        r#"<div class="card" style="margin-bottom:14px"><h3>📥 Receive goods — {}</h3><form data-action="po_receive_save" data-payload="terms"><input type="hidden" name="po" value="{}"><div class="field"><label>Received on <input class="inp" type="date" name="date" value="{}" required></label></div><div class="tbl-wrap"><table><thead><tr><th>Item</th><th class="num">Ordered</th><th class="num">Received</th><th class="num">Remaining</th><th class="num">Receive now</th>{rate_head}</tr></thead><tbody>"#,
        esc(&form.supplier_name),
        id.index(),
        form.received_on
    );
    for (i, row) in form.rows.iter().enumerate() {
        let lock = if row.quantity_editable {
            ""
        } else {
            " readonly"
        };
        // DEC-034: a Manager receives quantities only; the order's rates apply.
        let rate = if owner {
            format!(
                r#"<td class="num"><input class="inp num" name="rate_{i}" inputmode="decimal" aria-label="Rate {}" value="{}"{lock}></td>"#,
                esc(&row.item),
                row.rate.to_rupee_text()
            )
        } else {
            String::new()
        };
        let _ = write!(
            html,
            r#"<tr><td>{}</td><td class="num">{}</td><td class="num">{}</td><td class="num">{}</td><td class="num"><input class="inp num" name="qty_{i}" inputmode="numeric" aria-label="Receive now {}" value="{}"{lock}></td>{rate}</tr>"#,
            esc(&row.item),
            row.ordered,
            row.received,
            row.remaining,
            esc(&row.item),
            row.receive_now,
        );
    }
    Ok(
        html + r#"</tbody></table></div><div class="row" style="justify-content:flex-end;margin-top:10px"><button class="btn" type="button" data-action="po_receive_close">Cancel</button><button class="btn primary" type="submit">Save receipt</button></div></form></div>"#
    )
}
