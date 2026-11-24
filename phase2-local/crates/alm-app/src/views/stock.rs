//! Stock: ALM_Business_Manager.html L8046–8180 (VIEWS.stock), L1339–1395 (showLots) and
//! L8184–8264 (adjustStock: the Adjust dialog of a stock row).
//! Status, counts and FIFO value come from the account's projection (`stock_view`); lots are
//! read from the authority on demand (`open_stock_lots`).
use std::fmt::Write as _;

use alm_core::confirmation::{ConfirmationFixture, StockLotsError, StockStatus};
use alm_core::projection::ProjectionSnapshot;
use alm_core::{ProductId, Role};

use super::{denied, kpi, money_r};
use crate::gst_print::{esc, indian_digits};

#[derive(Clone, Debug, Default)]
pub struct StockState {
    pub query: String,
    pub low_only: bool,
    pub lots: Option<ProductId>,
    /// The product whose Adjust dialog is open.
    pub adjust: Option<ProductId>,
}

pub fn stock_screen(
    projected: &ProjectionSnapshot,
    fixture: &ConfirmationFixture,
    role: Role,
    state: &StockState,
) -> Result<String, StockLotsError> {
    let Ok(list) = projected.stock_view(role, &state.query, state.low_only) else {
        return Ok(denied("Stock"));
    };
    let value = list.value.map_or_else(|| "Unknown".to_owned(), money_r);
    let mut html = String::from(r#"<div class="kpis" style="margin-bottom:14px">"#);
    html += &kpi("b", "Stock Value", &value, "at FIFO lot cost", "🏬");
    html += &kpi(
        "a",
        "SKUs in stock",
        &list.in_stock.to_string(),
        &format!("of {}", list.skus),
        "📦",
    );
    html += &kpi(
        "w",
        "Low Stock",
        &list.low.to_string(),
        "need reorder soon",
        "⚠️",
    );
    html += &kpi(
        "r",
        "Out of Stock",
        &list.out.to_string(),
        "reorder now",
        "🛑",
    );
    let pill = if state.low_only { "pill on" } else { "pill" };
    let _ = write!(
        html,
        r#"</div><form class="row" style="margin-bottom:12px" data-action="stock_filter" data-payload="terms"><div class="search" style="flex:1"><span class="si">🔍</span><input class="inp" name="q" placeholder="Search item…" aria-label="Search item" value="{}"></div><button class="btn" type="submit" name="low" value="keep">Search</button><button class="{pill}" type="submit" name="low" value="toggle">⚠️ Low/Out only</button></form>"#,
        esc(&state.query)
    );
    html += r#"<div class="tbl-wrap" style="max-height:56vh"><table><thead><tr><th>Item</th><th class="num">On Hand</th><th class="num">Reorder</th><th>Status</th><th class="num">Actions</th></tr></thead><tbody>"#;
    if list.rows.is_empty() {
        html += r#"<tr><td colspan="5"><div class="empty">🏬<br>No items</div></td></tr>"#;
    }
    for (row, status) in &list.rows {
        let (class, label) = match status {
            StockStatus::Out => ("out", "OUT"),
            StockStatus::Low => ("low", "LOW"),
            StockStatus::Ok => ("ok", "OK"),
        };
        let _ = write!(
            html,
            r#"<tr><td><b>{}</b><div class="muted" style="font-size:11px">{}</div></td><td class="num" style="font-weight:800">{}</td><td class="num">{}</td><td><span class="tag {class}">{label}</span></td><td class="num"><button class="btn sm" data-action="stock_lots" data-id="{id}">📅 Lots</button> <button class="btn sm" data-action="stock_adjust" data-id="{id}">Adjust</button></td></tr>"#,
            esc(&row.item),
            esc(&row.brand),
            indian_digits(u64::from(row.on_hand)),
            indian_digits(u64::from(row.reorder_level)),
            id = row.product
        );
    }
    html += "</tbody></table></div>";
    if let Some(id) = state.lots {
        let lots = fixture.open_stock_lots(role, id)?;
        let name = list
            .rows
            .iter()
            .find(|(row, _)| row.product == id.value())
            .map_or("Product", |(row, _)| row.item.as_str());
        let mut body = format!(
            r#"<div class="help" style="margin-bottom:8px">Oldest stock sells first (FIFO). {} on hand.</div><div class="tbl-wrap"><table><thead><tr><th>Received</th><th class="num">Left</th><th class="num">Rate ₹</th><th class="num">Value</th></tr></thead><tbody>"#,
            indian_digits(u64::from(lots.on_hand))
        );
        for lot in &lots.rows {
            let _ = write!(
                body,
                r#"<tr><td>{}</td><td class="num" style="font-weight:800">{}</td><td class="num">{}</td><td class="num">{}</td></tr>"#,
                lot.date
                    .map_or_else(|| "Opening".to_owned(), alm_core::BillDate::display),
                indian_digits(u64::from(lot.remaining.get())),
                lot.unit_cost.map_or_else(|| "—".to_owned(), money_r),
                lot.value.map_or_else(|| "—".to_owned(), money_r)
            );
        }
        let _ = write!(
            body,
            r#"</tbody></table></div><div style="margin-top:8px"><b>Remaining value {}</b></div>"#,
            lots.total_value
                .map_or_else(|| "Unknown".to_owned(), money_r)
        );
        html += &super::modal(
            "📅",
            &format!("{name} — stock lots"),
            "stock_lots_close",
            false,
            &body,
        );
    }
    if let Some(id) = state.adjust {
        html += &adjust(fixture, role, &list.rows, id)?;
    }
    Ok(html)
}

/// `adjustStock`: set the counted on-hand and the fridge display; the difference goes to the
/// permanent adjustment log with a reason.
fn adjust(
    fixture: &ConfirmationFixture,
    role: Role,
    rows: &[(&alm_core::projection::ProjectedStock, StockStatus)],
    id: ProductId,
) -> Result<String, StockLotsError> {
    let count = fixture
        .open_physical_count(role, id)
        .map_err(|_| StockLotsError::ProductNotFound)?;
    let name = rows
        .iter()
        .find(|(row, _)| row.product == id.value())
        .map_or("Product", |(row, _)| row.item.as_str());
    let mut reasons = String::new();
    for (code, label) in [
        ("count", "Count correction"),
        ("damage", "Damage / leakage"),
        ("expired", "Expired"),
        ("theft", "Theft / unknown"),
        ("own-use", "Own use / sample"),
        ("other", "Other"),
    ] {
        let _ = write!(reasons, r#"<option value="{code}">{label}</option>"#);
    }
    let body = format!(
        r#"<form data-action="stock_count" data-payload="terms"><p class="help" style="margin-bottom:12px">Current on-hand: <b>{shown}</b> packets. Set the counted quantity — the difference is saved to the permanent adjustment log with a reason.</p><input type="hidden" name="product" value="{id}"><input type="hidden" name="on_hand" value="{on_hand}"><div class="f2"><div><label>Set actual on-hand (physical count) <input class="inp num" name="counted" inputmode="numeric" value="{on_hand}"></label></div><div><label>In Fridge (display) <input class="inp num" name="fridge" inputmode="numeric" value="{fridge}"></label></div></div><div class="f2"><div><label>Reason <select class="inp" name="reason">{reasons}</select></label></div><div><label>Note <input class="inp" name="note" placeholder="optional note"></label></div></div><div class="row"><button class="btn" type="button" data-action="stock_adjust_close">Cancel</button><button class="btn primary" type="submit">Save</button></div></form>"#,
        shown = indian_digits(u64::from(count.on_hand)),
        id = id.value(),
        on_hand = count.on_hand,
        fridge = count.fridge,
    );
    Ok(super::modal(
        "🔧",
        &format!("Adjust — {name}"),
        "stock_adjust_close",
        false,
        &body,
    ))
}
