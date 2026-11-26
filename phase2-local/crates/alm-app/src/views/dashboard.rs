//! Dashboard: ALM_Business_Manager.html L2457–2500 (dashRange), L2689–2984 (VIEWS.dashboard),
//! L2574–2579 (AGE_BANDS). Windows and totals come from `ConfirmationFixture::dashboard`, deltas,
//! rates, packets, top lists and chart geometry from `dashboard_insights`, receivables/top debtors
//! from `customer_book`, bands from `customer_ageing`. Charts are inline SVG (no JS charting).
use std::fmt::Write as _;

use alm_core::confirmation::owner_billing::{BillingDirectory, CustomerFilter};
use alm_core::confirmation::{ConfirmationFixture, DashboardError, DashboardPeriod, Delta};
use alm_core::{BillDate, Role};

use super::{denied, money_r};
use crate::gst_print::{esc, indian_digits};
use crate::refusal;

mod charts;

fn chip(delta: Delta) -> String {
    let (class, text) = delta.chip();
    let arrow = match class {
        "up" => "▲ ",
        "dn" => "▼ ",
        _ => "",
    };
    format!(r#"<span class="delta {class}">{arrow}{text}</span>"#)
}

fn stat(label: &str, value: &str, sub: &str) -> String {
    stat_with(label, value, sub, "")
}

fn stat_with(label: &str, value: &str, sub: &str, delta: &str) -> String {
    let private = if value.starts_with('₹') { "priv" } else { "" };
    format!(
        r#"<div class="stat"><div class="sl">{label}</div><div class="sv"><span class="{private}">{value}</span>{delta}</div><div class="ss">{}</div></div>"#,
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
        Err(error) => return Err(refusal(error)),
    };
    let (label, prior_label) = match period {
        DashboardPeriod::Today => ("today", "yesterday"),
        DashboardPeriod::Week => ("last 7 days", "previous 7 days"),
        DashboardPeriod::Month => ("this month", "last month"),
    };
    let cur = &dashboard.current;
    let insights = fixture
        .dashboard_insights(role, today, period)
        .map_err(refusal)?;
    let book = directory
        .customer_book(fixture, role, &CustomerFilter::All)
        .map_err(refusal)?;
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
        r#"</div><div class="hero"><div class="hl">Sales · {label}</div><div class="hv"><span class="priv">{}</span>{}</div><div class="hm">{} {bills} · {} pkt · avg <span class="priv">{}</span> per bill · {prior_label} <span class="priv">{}</span></div><div class="help">{} to {}</div></div><div class="stat3">"#,
        money_r(cur.sales),
        chip(insights.sales_delta),
        cur.bill_count,
        indian_digits(insights.packets),
        money_r(insights.average),
        money_r(dashboard.prior.sales),
        cur.window.start.display(),
        cur.window.end.display(),
    );
    html += &match &insights.profit {
        Some((profit, delta, margin)) => stat_with(
            "Profit",
            &money_r(*profit),
            &format!("{margin}% margin · estimated from FIFO cost"),
            &chip(*delta),
        ),
        None => stat("Profit", "Unavailable", "estimated from FIFO cost"),
    };
    html += &stat_with(
        "Collected",
        &money_r(cur.collected),
        &format!(
            "{}% of what you billed · bills {} · receipts {}",
            insights.collection_rate,
            money_r(cur.bill_paid),
            money_r(cur.receipts)
        ),
        &chip(insights.collected_delta),
    );
    html += &stat(
        "Receivables",
        &money_r(book.receivable),
        &format!("{} customers owe", book.debtors),
    );
    html += &stat_with(
        "Packets Sold",
        &format!("{} pkt", indian_digits(insights.packets)),
        &format!("whole cases billed · {label}"),
        &chip(insights.packets_delta),
    );
    let mix = insights.pay_mix.as_ref().map_or_else(
        || r#"<div class="empty" style="height:230px;display:grid;place-items:center">No sales in this period</div>"#.to_owned(),
        |donut| charts::donut(donut, "Payment mix"),
    );
    let _ = write!(
        html,
        r#"</div><div class="split" style="margin-top:16px"><div class="card"><div class="sectit">Sales — last 14 days</div>{}</div><div class="card"><div class="sectit">Payment mix · {label}</div>{mix}<div class="mini"><span>Cash</span><b class="priv">{}</b></div><div class="mini"><span>Online</span><b class="priv">{}</b></div></div></div><div class="split" style="margin-top:16px;grid-template-columns:1fr 1fr"><div class="card"><div class="sectit">Top products · {label}</div>"#,
        charts::line_chart(&insights.trend, "Sales — last 14 days"),
        money_r(cur.cash),
        money_r(cur.online)
    );
    if insights.top_products.is_empty() {
        html += r#"<div class="empty">No sales in this period</div>"#;
    }
    for row in &insights.top_products {
        let _ = write!(
            html,
            r#"<div style="margin-bottom:11px"><div class="row" style="justify-content:space-between;margin-bottom:4px"><span>{}</span><span class="muted">{} pkt · <span class="priv">{}</span></span></div><div class="minibar"><span style="width:{}%"></span></div></div>"#,
            esc(&row.item),
            indian_digits(row.packets),
            money_r(row.amount),
            row.bar_percent
        );
    }
    html += r#"</div><div class="card"><div class="sectit">Top debtors</div>"#;
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
    let ageing = fixture.customer_ageing(role, today).map_err(refusal)?;
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
    let _ = write!(
        html,
        r#"</div><div class="card"><div class="sectit">Top customers · {label}</div>"#
    );
    if insights.top_customers.is_empty() {
        html += r#"<div class="empty">No sales in this period</div>"#;
    }
    for (name, total) in &insights.top_customers {
        let link = book
            .rows
            .iter()
            .find(|row| row.customer.name == name.as_str());
        let name_html = link.map_or_else(
            || format!("<span>{}</span>", esc(name)),
            |row| {
                format!(
                    r#"<button class="list-link" type="button" data-action="customer_open" data-id="{}">{}</button>"#,
                    esc(row.customer.id),
                    esc(name)
                )
            },
        );
        let _ = write!(
            html,
            r#"<div class="mini">{name_html}<b class="priv">{}</b></div>"#,
            money_r(*total)
        );
    }
    Ok(html + "</div></div></div>")
}
