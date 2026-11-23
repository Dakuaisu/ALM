//! Reports: ALM_Business_Manager.html L8719–8800 (Profit & Loss by Month) and L8905–8945
//! (Slow-moving / Dead Stock). Months, bills, gross, net and the dead-stock cutoff, rows and
//! locked values come from `monthly_profit_report`, `dead_stock_cutoff` and `dead_stock_report`.
use std::fmt::Write as _;

use alm_core::confirmation::manual_cashbook::ManualCashBook;
use alm_core::confirmation::{
    ConfirmationError, ConfirmationFixture, ReportsError, dead_stock_cutoff,
};
use alm_core::{BillDate, Role};

use super::{denied, money_r};
use crate::gst_print::{esc, indian_digits};

pub fn reports_screen(
    fixture: &ConfirmationFixture,
    book: &ManualCashBook,
    role: Role,
    today: BillDate,
) -> Result<String, String> {
    let months = match fixture.monthly_profit_report(role, book, today) {
        Ok(months) => months,
        Err(ReportsError::Confirmation(ConfirmationError::AccessDenied)) => {
            return Ok(denied("Reports"));
        }
        Err(error) => return Err(format!("{error:?}")),
    };
    let mut html = String::from(
        r#"<div class="card priv"><div class="sectit">Profit &amp; Loss by Month</div><div class="help" style="margin-bottom:8px"><b>Net = Gross Profit − Other Expenses.</b> Gross Profit already has cost of goods deducted (FIFO), so Cash Book Goods (cash) is listed separately.</div><div class="tbl-wrap"><table><thead><tr><th>Month</th><th class="num">Bills</th><th class="num">Sales</th><th class="num">Gross Profit</th><th class="num">Goods (cash)</th><th class="num">Other Expenses</th><th class="num">Net</th></tr></thead><tbody>"#,
    );
    for month in months.iter().rev() {
        let goods = if month.goods.is_zero() {
            "—".to_owned()
        } else {
            money_r(month.goods)
        };
        let other = month
            .other_expenses()
            .map_err(|error| format!("{error:?}"))?;
        let _ = write!(
            html,
            r#"<tr><td>{:04}-{:02}</td><td class="num">{}</td><td class="num">{}</td><td class="num pos">{}</td><td class="num muted" title="Cash Book goods purchases — already inside Gross Profit if they came through a Purchase Order">{goods}</td><td class="num neg">{}</td><td class="num" style="font-weight:700">{}</td></tr>"#,
            month.year,
            month.month,
            month.bills.len(),
            money_r(month.revenue),
            money_r(month.gross),
            money_r(other),
            money_r(month.net)
        );
    }
    let cutoff = dead_stock_cutoff(today).map_err(|error| format!("{error:?}"))?;
    let dead = fixture
        .dead_stock_report(role, cutoff)
        .map_err(|error| format!("{error:?}"))?;
    let _ = write!(
        html,
        r#"</tbody></table></div></div><div class="card priv" style="margin-top:16px"><div class="sectit">Slow-moving / Dead Stock (in stock, not sold in 30+ days)</div><div class="help">Last sold before {cutoff}.</div><div class="tbl-wrap"><table><thead><tr><th>Item</th><th class="num">In Stock</th><th>Last Sold</th><th class="num">Locked Value</th></tr></thead><tbody>"#
    );
    if dead.is_empty() {
        html += r#"<tr><td colspan="4"><div class="empty">No dead stock 👍</div></td></tr>"#;
    }
    for row in dead {
        let _ = write!(
            html,
            r#"<tr><td>{}</td><td class="num">{} pkt</td><td class="muted">{}</td><td class="num">{}</td></tr>"#,
            esc(row.item.as_deref().unwrap_or("—")),
            indian_digits(u64::from(row.on_hand)),
            row.last_sale
                .map_or_else(|| "never".to_owned(), |d| d.to_string()),
            money_r(row.locked_value)
        );
    }
    Ok(html + "</tbody></table></div></div>")
}
