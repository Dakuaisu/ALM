//! Customers: ALM_Business_Manager.html L7094–7207 (VIEWS.customers), L7209–7318 (editCustomer,
//! opening balance only), L7320–7440 (openCustomer ledger, shown inline instead of a modal).
//! Balances, totals, debtor rule and filter come from `BillingDirectory::customer_book`.
use std::fmt::Write as _;

use alm_core::confirmation::owner_billing::{BalanceSide, BillingDirectory, CustomerFilter};
use alm_core::confirmation::{
    ConfirmationError, ConfirmationFixture, CustomerLedgerKind, OpeningLedgerKind, ReceiptMode,
};
use alm_core::{DraftCustomerId, Money, Role};

use super::{denied, kpi, money_r};
use crate::gst_print::esc;

#[derive(Clone, Debug, Default)]
pub struct CustomersState {
    pub query: String,
    pub due_only: bool,
    pub open: Option<String>,
}

impl CustomersState {
    fn filter(&self) -> CustomerFilter {
        if self.due_only {
            CustomerFilter::Due
        } else if self.query.trim().is_empty() {
            CustomerFilter::All
        } else {
            CustomerFilter::Name(self.query.clone())
        }
    }
}

fn side(side: BalanceSide) -> (&'static str, String) {
    match side {
        BalanceSide::Due(amount) => (" neg", money_r(amount)),
        BalanceSide::Advance(amount) => (" pos", format!("{} adv", money_r(amount))),
        BalanceSide::Settled => ("", "—".into()),
    }
}

fn shown(amount: Money) -> String {
    if amount.minor_units() == 0 {
        String::new()
    } else {
        money_r(amount)
    }
}

pub fn customers_screen(
    fixture: &ConfirmationFixture,
    directory: &BillingDirectory,
    role: Role,
    state: &CustomersState,
) -> Result<String, ConfirmationError> {
    let book = match directory.customer_book(fixture, role, &state.filter()) {
        Ok(book) => book,
        Err(ConfirmationError::AccessDenied) => return Ok(denied("Customers")),
        Err(error) => return Err(error),
    };
    let mut html = String::from(r#"<div class="kpis" style="margin-bottom:14px">"#);
    html += &kpi(
        "w",
        "Total Receivable",
        &money_r(book.receivable),
        &format!("{} customers", book.debtors),
        "⏳",
    );
    html += &kpi("g", "Advances", &money_r(book.advances), "paid extra", "💎");
    html += &kpi(
        "b",
        "Customers",
        &book.customers.to_string(),
        "on file",
        "👥",
    );
    let _ = write!(
        html,
        r#"</div><form class="row" style="margin-bottom:12px" data-action="customers_filter" data-payload="terms"><div class="search" style="flex:1"><span class="si">🔍</span><input class="inp" name="q" placeholder="Search customers…" aria-label="Search customers" value="{}"></div><button class="btn" type="submit" name="mode" value="search">Search</button><button class="pill" type="submit" name="mode" value="due">Only Due</button><button class="pill" type="submit" name="mode" value="all">All</button></form>"#,
        esc(&state.query)
    );
    html += r#"<div class="tbl-wrap" style="max-height:56vh"><table><thead><tr><th>Name</th><th>Mobile</th><th>Address</th><th class="num">Balance</th><th class="num">Actions</th></tr></thead><tbody>"#;
    if book.rows.is_empty() {
        html += r#"<tr><td colspan="5"><div class="empty">👥<br>No customers</div></td></tr>"#;
    }
    for row in &book.rows {
        let (class, balance) = side(row.side);
        let _ = write!(
            html,
            r#"<tr><td><button class="list-link" type="button" data-action="customer_open" data-id="{id}">{name}</button></td><td class="muted">—</td><td class="muted"></td><td class="num{class}">{balance}</td><td class="num"><button class="btn sm" data-action="customer_open" data-id="{id}">Ledger</button></td></tr>"#,
            id = esc(row.customer.id),
            name = esc(row.customer.name)
        );
    }
    html += "</tbody></table></div>";
    if let Some(open) = &state.open
        && let Some(row) = directory.customers(fixture).find(|c| c.id == open)
    {
        html += &ledger(fixture, role, row.id, row.name)?;
    }
    Ok(html)
}

fn ledger(
    fixture: &ConfirmationFixture,
    role: Role,
    id: &str,
    name: &str,
) -> Result<String, ConfirmationError> {
    let ledger = fixture.customer_ledger_with_opening(role, &DraftCustomerId::new(id.into()))?;
    let (class, balance) = match BalanceSide::of(ledger.closing)? {
        BalanceSide::Settled => ("pos", "₹0".to_owned()),
        other => {
            let (class, text) = side(other);
            (class.trim(), text)
        }
    };
    let mut html = format!(
        r#"<div class="card" style="margin-top:14px"><div class="row" style="justify-content:space-between"><h3>{}</h3><div style="text-align:right"><div class="muted" style="font-size:11px">BALANCE</div><div style="font-size:22px;font-weight:800" class="{class}">{balance}</div></div></div><div class="row" style="margin:12px 0"><button class="btn sm" data-action="screen" data-tab="receipt">💵 Receive Payment</button></div>"#,
        esc(name)
    );
    let _ = write!(
        html,
        r#"<form class="row" data-action="customer_opening" data-payload="terms"><input type="hidden" name="customer" value="{}"><label>Opening Balance ₹ <input class="inp num" name="amount" inputmode="decimal" placeholder="0"></label><label>Opening date <input class="inp" type="date" name="date"></label><button class="btn sm" type="submit">Save opening</button></form><div class="help">Opening balance = old udhaar before using this app (customer owes you).</div>"#,
        esc(id)
    );
    html += r#"<div class="sectit" style="margin-top:8px">Ledger</div><div class="tbl-wrap" style="max-height:40vh"><table><thead><tr><th>Date</th><th>Particulars</th><th class="num">Debit</th><th class="num">Credit</th><th class="num">Balance</th></tr></thead><tbody>"#;
    for entry in &ledger.entries {
        let particulars = match &entry.kind {
            OpeningLedgerKind::Opening => "Opening Balance".to_owned(),
            OpeningLedgerKind::Transaction(CustomerLedgerKind::Bill { draft_id, invoice }) => {
                let (review, _) = fixture.current_sale(role, *draft_id)?;
                format!("Bill #{invoice} ({} items)", review.draft().lines().len())
            }
            OpeningLedgerKind::Transaction(CustomerLedgerKind::Receipt { mode }) => match mode {
                ReceiptMode::Cash => "Payment (Cash)".to_owned(),
                ReceiptMode::Online => "Payment (Online)".to_owned(),
            },
        };
        let running = if matches!(BalanceSide::of(entry.balance)?, BalanceSide::Due(_)) {
            " neg"
        } else {
            ""
        };
        let _ = write!(
            html,
            r#"<tr><td>{}</td><td>{particulars}</td><td class="num">{}</td><td class="num pos">{}</td><td class="num{running}">{}</td></tr>"#,
            entry.date.map_or_else(String::new, |date| date.to_string()),
            shown(entry.debit),
            shown(entry.credit),
            money_r(entry.balance)
        );
    }
    Ok(html + "</tbody></table></div></div>")
}
