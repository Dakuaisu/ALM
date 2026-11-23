//! Order Planner: ALM_Business_Manager.html L10577–10925 (VIEWS.order, orderCheckout) and the PO
//! form's sheet paste (poForm paste, L6640–6760). Rows, suggestions, rates, cart total, checkout
//! and paste parsing come from `open_order_planner`, `OrderPlanner` and `parse_purchase_paste`.
use std::fmt::Write as _;

use alm_core::Role;
use alm_core::confirmation::order_planner::{OrderPlanner, OrderPlannerError};
use alm_core::confirmation::{ConfirmationFixture, PurchaseDraft};

use super::{denied, money_r};
use crate::gst_print::{esc, indian_digits};

#[derive(Clone, Debug, Default)]
pub struct OrderState {
    pub planner: Option<OrderPlanner>,
    pub low_only: bool,
    pub draft: Option<PurchaseDraft>,
    pub warnings: Vec<String>,
}

pub fn order_screen(
    fixture: &ConfirmationFixture,
    role: Role,
    state: &OrderState,
) -> Result<String, OrderPlannerError> {
    let opened;
    let planner = match &state.planner {
        Some(planner) => planner,
        None => match fixture.open_order_planner(role) {
            Ok(planner) => {
                opened = planner;
                &opened
            }
            Err(OrderPlannerError::AccessDenied) => return Ok(denied("Order Planner")),
            Err(error) => return Err(error),
        },
    };
    if role != Role::Owner {
        return Ok(denied("Order Planner"));
    }
    let cart = planner.cart();
    let mut html = format!(
        r#"<div class="card" style="margin-bottom:12px;display:flex;gap:14px;align-items:center;flex-wrap:wrap"><div style="font-weight:800">🛒 Cart: {} item{} · {}</div><div style="flex:1"></div><button class="btn primary" data-action="order_checkout">→ Create Purchase</button></div>"#,
        cart.len(),
        if cart.len() == 1 { "" } else { "s" },
        money_r(planner.cart_total()?)
    );
    if let Some(draft) = &state.draft {
        html += &draft_card(draft, &state.warnings);
    }
    if !cart.is_empty() {
        html += r#"<div class="card" style="margin-bottom:12px"><div class="sectit">Cart</div><div class="tbl-wrap"><table><thead><tr><th>Item</th><th class="num">Qty</th><th class="num">Rate</th><th>Supplier</th><th></th></tr></thead><tbody>"#;
        for line in cart {
            let _ = write!(
                html,
                r#"<tr><td><b>{}</b></td><td class="num"><form data-action="order_qty" data-payload="terms" class="row" style="justify-content:flex-end"><input type="hidden" name="product" value="{}"><input class="inp num" name="qty" inputmode="numeric" aria-label="Cart quantity {}" value="{}" style="width:80px"><button class="btn sm" type="submit">Set</button></form></td><td class="num">{}</td><td>{}</td><td><button class="x" data-action="order_remove" data-id="{}" aria-label="Remove {}">×</button></td></tr>"#,
                esc(&line.item),
                line.product,
                esc(&line.item),
                line.quantity,
                money_r(line.unit_rate),
                line.best_supplier.as_ref().map_or("—", |s| s.name.as_str()),
                line.product,
                esc(&line.item)
            );
        }
        html += "</tbody></table></div></div>";
    }
    let pill = if state.low_only { "pill on" } else { "pill" };
    let _ = write!(
        html,
        r#"<div class="row" style="margin-bottom:10px"><button class="{pill}" data-action="order_low">⚠️ Low / out only</button></div><div class="tbl-wrap" style="max-height:56vh"><table><thead><tr><th>Item</th><th class="num">On Hand</th><th class="num">Reorder</th><th class="num">Suggested</th><th class="num">Rate</th><th>Best Supplier</th><th class="num">Add</th></tr></thead><tbody>"#
    );
    for row in planner.rows(state.low_only) {
        let in_cart = cart.iter().any(|line| line.product == row.product);
        let add = if in_cart {
            r#"<span class="tag ok">in cart</span>"#.to_owned()
        } else {
            format!(
                r#"<form data-action="order_add" data-payload="terms" class="row" style="justify-content:flex-end"><input type="hidden" name="product" value="{}"><input class="inp num" name="qty" placeholder="{}" inputmode="numeric" aria-label="Order quantity {}" style="width:80px"><button class="btn sm" type="submit">＋ Add</button></form>"#,
                row.product,
                row.suggested_quantity,
                esc(&row.item)
            )
        };
        let _ = write!(
            html,
            r#"<tr><td><b>{}</b></td><td class="num">{}</td><td class="num">{}</td><td class="num" style="font-weight:700;color:var(--accent)">{} pkt</td><td class="num">{}</td><td>{}</td><td class="num">{add}</td></tr>"#,
            esc(&row.item),
            indian_digits(u64::from(row.on_hand)),
            indian_digits(u64::from(row.reorder_level)),
            row.suggested_quantity,
            money_r(row.unit_rate),
            row.best_supplier.as_ref().map_or("—", |s| s.name.as_str())
        );
    }
    Ok(html + "</tbody></table></div>")
}

fn draft_card(draft: &PurchaseDraft, warnings: &[String]) -> String {
    let supplier = draft.supplier.as_ref().map_or("", |s| s.name.as_str());
    let date = draft.date.map_or_else(String::new, |d| d.to_string());
    let mut html = format!(
        r#"<div class="card" style="margin-bottom:12px"><div class="sectit">New Purchase Order</div><form data-action="order_po" data-payload="terms"><div class="f3"><div><label>Supplier * <input class="inp" name="supplier" value="{}"></label></div><div><label>Order date <input class="inp" type="date" name="date" value="{date}"></label></div><div><label>Note <input class="inp" name="note" value="{}"></label></div></div><div class="tbl-wrap"><table><thead><tr><th>Item</th><th class="num">Order Qty</th><th class="num">Rate</th></tr></thead><tbody>"#,
        esc(supplier),
        esc(&draft.note)
    );
    for line in &draft.items {
        let _ = write!(
            html,
            r#"<tr><td>{}</td><td class="num">{}</td><td class="num">{}</td></tr>"#,
            esc(&line.item),
            line.ordered,
            money_r(line.rate)
        );
    }
    html += r#"</tbody></table></div><div class="field"><label>Paste from the supplier's sheet <textarea class="inp" name="paste" rows="3" placeholder="Select rows in the sheet, copy, paste here"></textarea></label></div>"#;
    for warning in warnings {
        let _ = write!(
            html,
            r#"<div class="help" role="alert">⚠ {}</div>"#,
            esc(warning)
        );
    }
    html + r#"<div class="row" style="justify-content:flex-end"><button class="btn" type="button" data-action="order_discard">Discard</button><button class="btn" type="submit" name="intent" value="paste">Fill from paste</button><button class="btn primary" type="submit" name="intent" value="save">Save Purchase Order</button></div></form></div>"#
}
