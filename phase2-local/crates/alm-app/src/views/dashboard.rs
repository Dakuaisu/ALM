//! Dashboard: ALM_Business_Manager.html L2457–2500 (dashRange), L2689–2984 (VIEWS.dashboard),
//! L2574–2579 (AGE_BANDS). Windows and totals come from `ConfirmationFixture::dashboard`, profit
//! from `sales_history`, receivables/top debtors from `customer_book`, bands from `customer_ageing`.
use std::fmt::Write as _;

use alm_core::confirmation::owner_billing::{BillingDirectory, CustomerFilter};
use alm_core::confirmation::{ConfirmationFixture, DashboardError, DashboardPeriod, SalesPeriod};
use alm_core::{BillDate, Role};

use super::{denied, money_r};
use crate::debug;
use crate::gst_print::esc;

fn stat(label: &str, value: &str, sub: &str) -> String {
    let private = if value.starts_with('₹') { "priv" } else { "" };
    format!(
        r#"<div class="stat"><div class="sl">{label}</div><div class="sv"><span class="{private}">{value}</span></div><div class="ss">{}</div></div>"#,
        esc(sub)
    )
}

pub fn dashboard_screen(
    fixture: &ConfirmationFixture,
    directory: &BillingDirectory,
    role: Role,
    today: BillDate,
    period: DashboardPeriod,
) -> Result<String, String> {
    let dashboard = match fixture.dashboard(role, today, period) {
        Ok(dashboard) => dashboard,
        Err(DashboardError::AccessDenied) => return Ok(denied("Dashboard")),
        Err(error) => return Err(debug(error)),
    };
    let (label, prior_label) = match period {
        DashboardPeriod::Today => ("today", "yesterday"),
        DashboardPeriod::Week => ("last 7 days", "previous 7 days"),
        DashboardPeriod::Month => ("this month", "last month"),
    };
    let cur = &dashboard.current;
    let window = SalesPeriod::Custom((Some(cur.window.start), Some(cur.window.end)));
    let profit = fixture
        .sales_history(role, today, window, "")
        .map_err(debug)?
        .profit
        .map_or_else(|_| "Unavailable".to_owned(), |p| money_r(p.profit));
    let book = directory
        .customer_book(fixture, role, &CustomerFilter::All)
        .map_err(debug)?;
    let mut html = String::from(r#"<div id="dash"><div class="seg">"#);
    for (key, value, text) in [
        (DashboardPeriod::Today, "today", "Today"),
        (DashboardPeriod::Week, "week", "7 days"),
        (DashboardPeriod::Month, "month", "This month"),
    ] {
        let on = if key == period { "on" } else { "" };
        let _ = write!(
            html,
            r#"<button class="{on}" type="button" data-action="dash_period" data-period="{value}">{text}</button>"#
        );
    }
    let bills = if cur.bill_count == 1 { "bill" } else { "bills" };
    let _ = write!(
        html,
        r#"</div><div class="hero"><div class="hl">Sales · {label}</div><div class="hv"><span class="priv">{}</span></div><div class="hm">{} {bills} · {} to {} · {prior_label} <span class="priv">{}</span></div></div><div class="stat3">"#,
        money_r(cur.sales),
        cur.bill_count,
        cur.window.start.display(),
        cur.window.end.display(),
        money_r(dashboard.prior.sales)
    );
    html += &stat("Profit", &profit, "estimated from FIFO cost");
    html += &stat(
        "Collected",
        &money_r(cur.collected),
        &format!(
            "bills {} · receipts {}",
            money_r(cur.bill_paid),
            money_r(cur.receipts)
        ),
    );
    html += &stat(
        "Receivables",
        &money_r(book.receivable),
        &format!("{} customers owe", book.debtors),
    );
    let _ = write!(
        html,
        r#"</div><div class="split" style="margin-top:16px;grid-template-columns:1fr 1fr"><div class="card"><div class="sectit">Payment mix · {label}</div><div class="mini"><span>Cash</span><b class="priv">{}</b></div><div class="mini"><span>Online</span><b class="priv">{}</b></div></div><div class="card"><div class="sectit">Top debtors</div>"#,
        money_r(cur.cash),
        money_r(cur.online)
    );
    let top = book.top_debtors(6);
    if top.is_empty() {
        html += r#"<div class="empty">No pending dues 🎉</div>"#;
    }
    for row in top {
        let _ = write!(
            html,
            r#"<div class="mini"><button class="list-link" type="button" data-action="customer_open" data-id="{}">{}</button><b class="neg priv">{}</b></div>"#,
            esc(row.customer.id),
            esc(row.customer.name),
            money_r(row.balance)
        );
    }
    html += r#"</div></div><div class="split" style="margin-top:16px;grid-template-columns:1fr 1fr"><div class="card"><div class="sectit">How old is the udhaar</div>"#;
    let ageing = fixture.customer_ageing(role, today).map_err(debug)?;
    if ageing.rows.is_empty() {
        html += r#"<div class="empty">Nothing outstanding 🎉</div>"#;
    } else {
        for (band, value) in [
            ("0–15 days", ageing.d15),
            ("16–30 days", ageing.d30),
            ("31–60 days", ageing.d60),
            ("60+ days", ageing.old),
        ] {
            let _ = write!(
                html,
                r#"<div class="agerow"><span class="al">{band}</span><span class="av priv">{}</span></div>"#,
                money_r(value)
            );
        }
    }
    Ok(html + "</div></div></div>")
}
