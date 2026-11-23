//! Sales history: ALM_Business_Manager.html L5975–6239; table and KPI helpers L2270–2300, L2986–3001.
use std::fmt::Write as _;

use alm_core::confirmation::owner_billing::BalanceSide;
use alm_core::confirmation::{ConfirmationError, ConfirmationFixture, ReceiptMode, SalesPeriod};
use alm_core::{BillDate, Role};

use super::{customer, denied, money_r};
use crate::gst_print::{esc, indian_digits};

#[derive(serde::Deserialize)]
pub struct SalesFilter {
    q: String,
    from: String,
    to: String,
    #[serde(default)]
    period: String,
}

impl SalesFilter {
    pub fn checked(self) -> Result<Filter, alm_core::InvalidBillDate> {
        let date = |text: &str| {
            if text.is_empty() {
                Ok(None)
            } else {
                text.parse().map(Some)
            }
        };
        let period = match self.period.as_str() {
            "Today" => SalesPeriod::Today,
            "This Month" => SalesPeriod::ThisMonth,
            "All" => SalesPeriod::All,
            _ => SalesPeriod::Custom((date(&self.from)?, date(&self.to)?)),
        };
        Ok(Filter {
            period,
            q: if self.period.is_empty() {
                self.q
            } else {
                String::new()
            },
        })
    }
}

pub struct Filter {
    period: SalesPeriod,
    q: String,
}

impl Default for Filter {
    fn default() -> Self {
        Self {
            period: SalesPeriod::All,
            q: String::new(),
        }
    }
}

pub fn sales_screen(
    fixture: &ConfirmationFixture,
    role: Role,
    filter: &Filter,
    today: BillDate,
) -> Result<String, ConfirmationError> {
    let history = match fixture.sales_history(role, today, filter.period, &filter.q) {
        Ok(history) => history,
        Err(ConfirmationError::AccessDenied) => return Ok(denied("Sales")),
        Err(error) => return Err(error),
    };
    let mut html = filters(filter, today);
    let bills = history.collected.bill_ids.len();
    let cancelled = history.rows.len() - bills;
    let note = if cancelled > 0 {
        format!("{cancelled} cancelled in view")
    } else {
        "in view".into()
    };
    let collected = format!(
        "({} bills · {} received)",
        money_r(history.bill_paid),
        money_r(history.receipt_paid)
    );
    let profit = history
        .profit
        .map_or_else(|_| "Unavailable".into(), |p| money_r(p.profit));
    html += r#"<div class="kpis" style="margin-bottom:14px">"#;
    for (class, label, value, sub) in [
        ("b", "Bills", bills.to_string(), note.as_str()),
        ("g privcard", "Sales Value", money_r(history.sales), ""),
        (
            "a",
            "Packets Sold",
            format!("{} pkt", indian_digits(u64::from(history.active_packets))),
            "in view",
        ),
        (
            "a privcard",
            "Collected",
            money_r(history.total_collected),
            collected.as_str(),
        ),
        ("w privcard", "Est. Profit", profit, ""),
    ] {
        let _ = write!(
            html,
            r#"<div class="kpi {class}"><div class="label">{label}</div><div class="val">{value}</div><div class="sub">{sub}</div></div>"#
        );
    }
    html += r#"</div><div class="tbl-wrap" style="max-height:58vh"><table><thead><tr><th>Date</th><th>Bill</th><th>Customer</th><th>Details</th><th>Mode</th><th class="num">Amount</th><th class="num">Balance</th><th class="num">Actions</th></tr></thead><tbody>"#;
    if history.rows.is_empty() {
        html += r#"<tr><td colspan="8"><div class="empty">No sales in this range</div></td></tr>"#;
    }
    for row in history.rows {
        let (review, _) = fixture.current_sale(role, row.bill.draft_id)?;
        let id = row.bill.draft_id.value();
        let (style, tag, action) = if row.cancelled {
            (
                "opacity:.55",
                r#" <span class="tag out">CANCELLED</span>"#,
                format!(
                    r#"<button class="btn sm" data-action="sale_restore" data-id="{id}">Undo</button>"#
                ),
            )
        } else {
            (
                "",
                "",
                format!(
                    r#"<button class="btn sm" data-action="sale_edit_open" data-id="{id}">Edit</button> <details style="display:inline-block"><summary class="btn sm danger">Cancel</summary><button class="btn sm danger" data-action="sale_cancel" data-id="{id}">Cancel this bill</button></details>"#
                ),
            )
        };
        let (mode, mode_class) = if row.is_credit() {
            ("Credit", "credit")
        } else {
            match review.payment_mode() {
                ReceiptMode::Cash => ("Cash", "cash"),
                ReceiptMode::Online => ("Online", "cash"),
            }
        };
        let (balance_class, balance) = match BalanceSide::of(row.balance)? {
            BalanceSide::Advance(amount) => ("pos", format!("{} adv", money_r(amount))),
            BalanceSide::Settled => ("", "—".into()),
            BalanceSide::Due(amount) => ("neg", money_r(amount)),
        };
        let _ = write!(
            html,
            r#"<tr style="{style}"><td>{}</td><td><b>#{}</b>{tag}</td><td>{}</td><td class="muted">{} items · {} pkt</td><td><span class="tag {mode_class}">{mode}</span></td><td class="num">{}</td><td class="num {balance_class}">{balance}</td><td class="num">{action}</td></tr>"#,
            row.date.display(),
            row.bill.invoice,
            esc(customer(review.draft())),
            review.draft().lines().len(),
            row.bill.quantity,
            money_r(row.bill.total)
        );
    }
    Ok(html + "</tbody></table></div>")
}

fn filters(filter: &Filter, today: BillDate) -> String {
    let (from, to) = match filter.period {
        SalesPeriod::All => (String::new(), String::new()),
        SalesPeriod::Today => (today.to_string(), today.to_string()),
        SalesPeriod::ThisMonth => (today.first_of_month().to_string(), String::new()),
        SalesPeriod::Custom((from, to)) => (
            from.map_or_else(String::new, |d| d.to_string()),
            to.map_or_else(String::new, |d| d.to_string()),
        ),
    };
    format!(
        r#"<div class="row no-print"><button class="btn primary" data-action="screen" data-tab="draft">New Bill</button><button class="btn" data-action="screen" data-tab="review">Owner review</button></div>
    <p class="help">Synthetic development records · in memory for this session · today: {}</p>
    <div class="card" style="margin-bottom:14px"><form class="row" data-action="sales_filter" data-payload="terms">
    <div class="search" style="flex:1;min-width:200px"><input class="inp" name="q" placeholder="Search customer or item…" aria-label="Search customer or item" value="{}"></div>
    <label class="muted">From <input class="inp" type="date" name="from" value="{from}"></label>
    <label class="muted">To <input class="inp" type="date" name="to" value="{to}"></label>
    <button class="btn" type="submit" name="period" value="">Filter</button><button class="pill" name="period" value="Today">Today</button><button class="pill" name="period" value="This Month">This Month</button><button class="pill" name="period" value="All">All</button></form></div>"#,
        today.display(),
        esc(&filter.q)
    )
}
