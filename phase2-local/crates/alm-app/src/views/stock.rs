//! Stock: ALM_Business_Manager.html L8046–8180 (VIEWS.stock) and L1339–1395 (showLots, inline).
//! Status, counts and FIFO value come from `stock_list`; lots from `open_stock_lots`.
use std::fmt::Write as _;

use alm_core::confirmation::{ConfirmationFixture, StockLotsError, StockStatus};
use alm_core::{ProductId, Role};

use super::{denied, kpi, money_r};
use crate::gst_print::{esc, indian_digits};

#[derive(Clone, Debug, Default)]
pub struct StockState {
    pub query: String,
    pub low_only: bool,
    pub lots: Option<ProductId>,
}

pub fn stock_screen(
    fixture: &ConfirmationFixture,
    role: Role,
    state: &StockState,
) -> Result<String, StockLotsError> {
    let list = match fixture.stock_list(role, &state.query, state.low_only) {
        Ok(list) => list,
        Err(StockLotsError::AccessDenied) => return Ok(denied("Stock")),
        Err(error) => return Err(error),
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
    for row in &list.rows {
        let (class, label) = match row.status {
            StockStatus::Out => ("out", "OUT"),
            StockStatus::Low => ("low", "LOW"),
            StockStatus::Ok => ("ok", "OK"),
        };
        let _ = write!(
            html,
            r#"<tr><td><b>{}</b><div class="muted" style="font-size:11px">{}</div></td><td class="num" style="font-weight:800">{}</td><td class="num">{}</td><td><span class="tag {class}">{label}</span></td><td class="num"><button class="btn sm" data-action="stock_lots" data-id="{}">📅 Lots</button></td></tr>"#,
            esc(&row.item),
            esc(&row.brand),
            indian_digits(u64::from(row.on_hand)),
            indian_digits(u64::from(row.reorder_level)),
            row.product
        );
    }
    html += "</tbody></table></div>";
    if let Some(id) = state.lots {
        let lots = fixture.open_stock_lots(role, id)?;
        let name = list
            .rows
            .iter()
            .find(|row| row.product == id)
            .map_or("Product", |row| row.item.as_str());
        let _ = write!(
            html,
            r#"<div class="card" style="margin-top:14px"><h3>📅 {} — stock lots</h3><div class="help" style="margin-bottom:8px">Oldest stock sells first (FIFO). {} on hand.</div><div class="tbl-wrap"><table><thead><tr><th>Received</th><th class="num">Left</th><th class="num">Rate ₹</th><th class="num">Value</th></tr></thead><tbody>"#,
            esc(name),
            indian_digits(u64::from(lots.on_hand))
        );
        for lot in &lots.rows {
            let _ = write!(
                html,
                r#"<tr><td>{}</td><td class="num" style="font-weight:800">{}</td><td class="num">{}</td><td class="num">{}</td></tr>"#,
                lot.date
                    .map_or_else(|| "Opening".to_owned(), alm_core::BillDate::display),
                indian_digits(u64::from(lot.remaining.get())),
                lot.unit_cost.map_or_else(|| "—".to_owned(), money_r),
                lot.value.map_or_else(|| "—".to_owned(), money_r)
            );
        }
        let _ = write!(
            html,
            r#"</tbody></table></div><div style="margin-top:8px"><b>Remaining value {}</b></div></div>"#,
            lots.total_value
                .map_or_else(|| "Unknown".to_owned(), money_r)
        );
    }
    Ok(html)
}
