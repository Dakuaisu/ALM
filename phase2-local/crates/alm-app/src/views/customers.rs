//! Customers: ALM_Business_Manager.html L7094–7207 (VIEWS.customers), L7209–7318 (editCustomer,
//! opening balance only), L7320–7440 (openCustomer ledger, shown inline instead of a modal).
//! Balances, totals, debtor rule and filter come from the account's projection
//! (`ProjectionSnapshot::customer_view`); the ledger modal is read from the authority on demand.
use std::fmt::Write as _;

use alm_core::confirmation::owner_billing::{BalanceSide, BillingDirectory, CustomerFilter};
use alm_core::confirmation::{
    ConfirmationError, ConfirmationFixture, CustomerLedgerKind, OpeningLedgerKind, ReceiptMode,
};
use alm_core::projection::ProjectionSnapshot;
use alm_core::{DraftCustomerId, Money, Role};

use super::{denied, kpi, money_r};
use crate::gst_print::esc;

#[derive(Clone, Debug, Default)]
pub struct CustomersState {
    pub query: String,
    pub due_only: bool,
    pub open: Option<String>,
    /// The Add / Edit Customer form, when open.
    pub form: Option<alm_core::confirmation::owner_billing::CustomerContact>,
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
    if amount.is_zero() {
        String::new()
    } else {
        money_r(amount)
    }
}

pub fn customers_screen(
    projected: &ProjectionSnapshot,
    fixture: &ConfirmationFixture,
    directory: &BillingDirectory,
    role: Role,
    state: &CustomersState,
) -> Result<String, ConfirmationError> {
    let book = match projected.customer_view(role, &state.filter()) {
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
        r#"</div><form class="row" style="margin-bottom:12px" data-action="customers_filter" data-payload="terms"><div class="search" style="flex:1"><span class="si">🔍</span><input class="inp" name="q" placeholder="Search customers…" aria-label="Search customers by name or mobile" value="{}"></div><button class="btn" type="submit" name="mode" value="search">Search</button><button class="pill" type="submit" name="mode" value="due">Only Due</button><button class="pill" type="submit" name="mode" value="all">All</button></form>"#,
        esc(&state.query)
    );
    html += r#"<div class="tbl-wrap" style="max-height:56vh"><table><thead><tr><th>Name</th><th>Mobile</th><th>Address</th><th class="num">Balance</th><th class="num">Actions</th></tr></thead><tbody>"#;
    if book.rows.is_empty() {
        html += r#"<tr><td colspan="5"><div class="empty">👥<br>No customers</div></td></tr>"#;
    }
    for (customer, balance_side) in &book.rows {
        let (class, balance) = side(*balance_side);
        let _ = write!(
            html,
            r#"<tr><td>{name}</td><td class="muted">{mobile}</td><td class="muted">{address}</td><td class="num{class}">{balance}</td><td class="num">{ledger}<button class="btn sm" data-action="customer_edit" data-id="{id}" aria-label="Edit customer">✎</button></td></tr>"#,
            // DEC-034: the ledger stays the Owner's.
            ledger = if role == Role::Owner {
                format!(
                    r#"<button class="btn sm" data-action="customer_open" data-id="{}">Ledger</button> "#,
                    esc(&customer.id)
                )
            } else {
                String::new()
            },
            id = esc(&customer.id),
            name = if role == Role::Owner {
                format!(
                    r#"<button class="list-link" type="button" data-action="customer_open" data-id="{}">{}</button>"#,
                    esc(&customer.id),
                    esc(&customer.name)
                )
            } else {
                esc(&customer.name)
            },
            mobile = if customer.mobile.is_empty() {
                "—".to_owned()
            } else {
                esc(&customer.mobile)
            },
            address = esc(&customer.address()),
        );
    }
    html += "</tbody></table></div>";
    if let Some(form) = &state.form {
        html += &customer_form(form);
    }
    if let Some(open) = &state.open
        && alm_core::permissions::allows(role, alm_core::permissions::Capability::CustomerLedger)
        && let Some(row) = directory.customers(fixture).find(|c| c.id == *open)
    {
        let contact = projected
            .customers
            .iter()
            .find(|c| c.id == row.id)
            .map_or_else(
                || "no mobile · ".to_owned(),
                |c| {
                    let mobile = if c.mobile.is_empty() {
                        "no mobile"
                    } else {
                        &c.mobile
                    };
                    format!("{mobile} · {}", c.address())
                },
            );
        html += &ledger(fixture, role, (&row.id, &row.name), &contact)?;
    }
    Ok(html)
}

/// `editCustomer`: name, mobile and address (the opening balance is on the Owner's ledger).
fn customer_form(form: &alm_core::confirmation::owner_billing::CustomerContact) -> String {
    let readonly = if form.id.is_some() { " readonly" } else { "" };
    let body = format!(
        r#"<form data-action="customer_save" data-payload="terms"><input type="hidden" name="id" value="{}"><div class="field"><label>Name * <input class="inp" name="name" value="{}"{readonly} required></label></div><div class="field"><label>Mobile <input class="inp" name="mobile" inputmode="numeric" value="{}"></label></div><div class="f2"><div><label>Address 1 <input class="inp" name="addr1" value="{}"></label></div><div><label>Address 2 <input class="inp" name="addr2" value="{}"></label></div></div><div class="row" style="justify-content:flex-end;margin-top:10px"><button class="btn" type="button" data-action="customer_close">Cancel</button><button class="btn primary" type="submit">Save</button></div></form>"#,
        esc(form.id.as_deref().unwrap_or("")),
        esc(&form.name),
        esc(&form.mobile),
        esc(&form.addr1),
        esc(&form.addr2)
    );
    let title = if form.id.is_some() {
        "Edit Customer"
    } else {
        "Add Customer"
    };
    super::modal("👤", title, "customer_close", true, &body)
}

fn ledger(
    fixture: &ConfirmationFixture,
    role: Role,
    (id, name): (&str, &str),
    contact: &str,
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
        r#"<div class="row" style="justify-content:space-between"><div style="font-size:13px;color:var(--mut)">{contact}</div><div style="text-align:right"><div class="muted" style="font-size:11px">BALANCE</div><div style="font-size:22px;font-weight:800" class="{class}">{balance}</div></div></div><div class="row" style="margin:12px 0"><button class="btn sm" data-action="screen" data-tab="receipt">💵 Receive Payment</button></div>"#
    );
    let _ = write!(
        html,
        r#"<form class="row" data-action="customer_opening" data-payload="terms"><input type="hidden" name="customer" value="{}"><label>Opening Balance ₹ <input class="inp num" name="amount" inputmode="decimal" placeholder="0"></label><label>Opening date <input class="inp" type="date" name="date"></label><button class="btn sm" type="submit">Save opening</button></form><div class="help">Opening balance = old udhaar before using this app (customer owes you); enter a negative amount for an advance.</div>"#,
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
            entry
                .date
                .map_or_else(String::new, alm_core::BillDate::display),
            shown(entry.debit),
            shown(entry.credit),
            money_r(entry.balance)
        );
    }
    html += "</tbody></table></div>";
    Ok(super::modal("👤", name, "customer_close", true, &html))
}
