//! Cash Book: ALM_Business_Manager.html L8432–8508 (VIEWS.expenses) and L8510–8715 (editExpense
//! with the day-close system card), shown inline instead of a modal. Month totals, system figures,
//! differences and "Use system figures" come from `manual_cashbook` and `cashbook_day_close`.
use std::fmt::Write as _;

use alm_core::confirmation::ConfirmationFixture;
use alm_core::confirmation::manual_cashbook::{
    ManualCashBook, ManualCashBookDraft, ManualCashBookError,
};
use alm_core::{BillDate, Money, Role};

use super::{denied, kpi, money_r};
use crate::gst_print::indian_digits;

pub struct CashBookView<'a> {
    pub fixture: &'a ConfirmationFixture,
    pub book: &'a ManualCashBook,
    pub entry: Option<&'a ManualCashBookDraft>,
    pub editing: Option<u64>,
    pub role: Role,
    pub today: BillDate,
}

fn signed(rupees: i64) -> String {
    let sign = match rupees.signum() {
        1 => "+₹",
        -1 => "−₹",
        _ => "₹",
    };
    format!("{sign}{}", indian_digits(rupees.unsigned_abs()))
}

/// Whole rupees as moneyR shows them ("₹20", "₹-3").
fn whole(rupees: i64) -> String {
    let sign = if rupees < 0 { "-" } else { "" };
    format!("₹{sign}{}", indian_digits(rupees.unsigned_abs()))
}

fn shown(amount: Money) -> String {
    if amount.is_zero() {
        String::new()
    } else {
        money_r(amount)
    }
}

pub fn cashbook_screen(view: &CashBookView<'_>) -> Result<String, ManualCashBookError> {
    let month = match view.book.month(view.role, view.today) {
        Ok(month) => month,
        Err(ManualCashBookError::AccessDenied) => return Ok(denied("Cash Book")),
        Err(error) => return Err(error),
    };
    let mut html = String::from(r#"<div class="kpis" style="margin-bottom:14px">"#);
    html += &kpi(
        "g",
        "Income (month)",
        &money_r(month.income),
        "cash+online",
        "💰",
    );
    html += &kpi(
        "r",
        "Expenses (month)",
        &money_r(month.expenses),
        "goods+misc+salary",
        "📉",
    );
    html += &kpi("b", "Net (month)", &money_r(month.net), "", "🧮");
    html += "</div>";
    if let Some(entry) = view.entry {
        html += &entry_form(view, entry)?;
    }
    html += r#"<div class="tbl-wrap" style="max-height:56vh"><table><thead><tr><th>Date</th><th class="num">Cash</th><th class="num">Paytm</th><th class="num">PhonePe</th><th class="num">Debt</th><th>Goods Purchase</th><th>Misc Expense</th><th class="num">Salary</th><th class="num">Actions</th></tr></thead><tbody>"#;
    let rows = view.book.rows(view.role)?;
    if rows.is_empty() {
        html += r#"<tr><td colspan="9"><div class="empty">📒<br>No entries yet</div></td></tr>"#;
    }
    for row in rows {
        let a = row.amounts;
        let _ = write!(
            html,
            r#"<tr><td>{}</td><td class="num pos">{}</td><td class="num">{}</td><td class="num">{}</td><td class="num">{}</td><td class="muted">{}</td><td class="muted">{}</td><td class="num">{}</td><td class="num"><button class="btn sm" data-action="cash_edit" data-id="{}" aria-label="Edit entry">✎</button></td></tr>"#,
            row.date.display(),
            shown(a.cash),
            shown(a.paytm),
            shown(a.phonepe),
            shown(a.debt),
            shown(a.goods),
            shown(a.misc),
            shown(a.salary),
            row.id.value()
        );
    }
    Ok(html + "</tbody></table></div>")
}

fn entry_form(
    view: &CashBookView<'_>,
    entry: &ManualCashBookDraft,
) -> Result<String, ManualCashBookError> {
    let date = entry.date.map_or_else(String::new, |d| d.to_string());
    let title = if view.editing.is_some() {
        "Edit Entry"
    } else {
        "Add Day Entry"
    };
    let mut html = format!(
        r#"<form data-action="cash_entry" data-payload="terms" data-live="cash_live" data-target="cash-system"><input type="hidden" name="id" value="{}"><div class="field"><label>Date <input class="inp" type="date" name="date" value="{date}" required></label></div>"#,
        view.editing.map_or_else(String::new, |id| id.to_string())
    );
    html += &cash_system_card(view.fixture, view.role, entry)?;
    let a = entry.amounts;
    let field = |label: &str, name: &str, value: Money| {
        format!(
            r#"<div><label>{label} <input class="inp num" id="cash-{name}" name="{name}" inputmode="decimal" value="{}"></label></div>"#,
            value.to_rupee_text()
        )
    };
    let _ = write!(
        html,
        r#"<div class="sectit">Income (counted)</div><div class="f3">{}{}{}</div><div class="sectit" style="margin-top:8px">Expenses</div><div class="f2">{}{}</div><div class="f2">{}{}</div>"#,
        field("Cash in Hand ₹", "cash", a.cash),
        field("Paytm ₹", "paytm", a.paytm),
        field("PhonePe ₹", "phonepe", a.phonepe),
        field("Debt / Lent ₹", "debt", a.debt),
        field("Staff Salary ₹", "salary", a.salary),
        field("Goods Purchase ₹", "goods", a.goods),
        field("Misc Expense ₹", "misc", a.misc),
    );
    html += r#"<div class="row" style="margin-top:10px">"#;
    if view.editing.is_some() {
        html += r#"<details style="margin-right:auto"><summary class="btn danger">Delete</summary><button class="btn danger" type="submit" name="intent" value="delete">Delete this entry</button></details>"#;
    }
    html += r#"<button class="btn" type="button" data-action="cash_close">Cancel</button><button class="btn primary" type="submit" name="intent" value="save">Save</button></div></form>"#;
    Ok(super::modal("📒", title, "cash_close", false, &html))
}

/// The day-close card (`sysFor` / `updSys`), returned alone for live updates while typing.
pub fn cash_system_card(
    fixture: &ConfirmationFixture,
    role: Role,
    entry: &ManualCashBookDraft,
) -> Result<String, ManualCashBookError> {
    let mut html = String::from(
        r#"<div class="card" id="cash-system" style="padding:10px 12px;margin-bottom:12px;background:var(--accent-weak);border-color:#c5d6fb">"#,
    );
    if let Some(shown) = entry.date.map(alm_core::BillDate::display) {
        let check = entry.compare(fixture, role)?;
        let source = if check.system.count > 0 {
            format!(" — from {} bills/receipts", check.system.count)
        } else {
            " — no records".into()
        };
        let class = |rupees: i64| if rupees == 0 { "pos" } else { "neg" };
        let _ = write!(
            html,
            r#"<div style="font-weight:600">📟 System total for {shown}: Cash {} · Online {}{source}</div><div class="help" style="margin-top:4px">Part-payments on Credit bills count as cash. Enter what you actually counted below — any difference shows here instantly.</div><div style="margin-top:6px">Cash difference: <b class="{}">{}</b>  ·  Online difference: <b class="{}">{}</b></div>"#,
            whole(check.system.cash_rupees),
            whole(check.system.online_rupees),
            class(check.cash_difference_rupees),
            signed(check.cash_difference_rupees),
            class(check.online_difference_rupees),
            signed(check.online_difference_rupees)
        );
    }
    html += r#"<button class="btn sm" style="margin-top:8px" type="submit" name="intent" value="system">Use system figures</button></div>"#;
    Ok(html)
}
