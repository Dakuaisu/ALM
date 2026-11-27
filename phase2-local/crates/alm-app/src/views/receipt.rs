//! Receive Payment: ALM_Business_Manager.html L5860–5965 (openPaymentModal with the customer
//! autocomplete). Matching customers and their balances come from core (`search_customers`,
//! `customer_ledger`).
use alm_core::confirmation::owner_billing::BillingDirectory;
use alm_core::confirmation::{ConfirmationError, ConfirmationFixture};
use alm_core::{BillDate, DraftCustomerId, Role};

use super::{SearchOption, denied, money_r, search_options};

pub fn receipt_screen(
    _fixture: &ConfirmationFixture,
    _directory: &BillingDirectory,
    role: Role,
    today: BillDate,
) -> Result<String, ConfirmationError> {
    if !alm_core::permissions::allows(role, alm_core::permissions::Capability::RecordReceipt) {
        return Ok(denied("Receive Payment"));
    }
    Ok(format!(
        r#"<div class="card"><p class="help">Local simulation only. This payment is dated independently of any bill.</p>
    <form data-action="receipt_save" data-payload="terms"><div class="field autobox"><label>Customer <input class="inp" role="combobox" aria-autocomplete="list" aria-expanded="false" aria-controls="receipt-customer-list" name="customer_q" data-search="receipt_customer_search" autocomplete="off" placeholder="Search customer…" required></label><input type="hidden" name="customer" value=""><div class="autolist" id="receipt-customer-list" role="listbox" aria-label="Customer suggestions"></div></div>
    <div class="f2"><div class="field"><label>Amount ₹ <input class="inp num" name="amount" inputmode="decimal" required></label></div>
    <div class="field"><label>Mode <select class="inp" name="mode"><option>Cash</option><option>Online</option></select></label></div></div>
    <div class="field"><label>Date <input class="inp" type="date" name="date" value="{today}" required></label></div>
    <button class="btn primary" type="submit">Save Payment</button></form></div>"#
    ))
}

/// The customer autocomplete with each match's current balance: from the ledger, or from
/// `balances` (a Manager's customers, DEC-034).
pub fn receipt_customer_options(
    fixture: &ConfirmationFixture,
    directory: &BillingDirectory,
    role: Role,
    balances: Option<&[alm_core::projection::ProjectedCustomer]>,
    query: &str,
) -> Result<String, ConfirmationError> {
    let found = directory.search_customers(fixture, query);
    let mut shown = Vec::new();
    for customer in &found {
        let closing = match balances {
            Some(rows) => rows
                .iter()
                .find(|row| row.id == customer.id)
                .map_or(alm_core::Money::from_minor_units(0), |row| row.balance),
            None => {
                fixture
                    .customer_ledger(role, &DraftCustomerId::new(customer.id.clone()))?
                    .closing
            }
        };
        shown.push(format!("balance {}", money_r(closing)));
    }
    let balances = shown;
    let fills: Vec<_> = found
        .iter()
        .map(|c| [("customer", c.id.as_str()), ("customer_q", c.name.as_str())])
        .collect();
    let options: Vec<_> = found
        .iter()
        .zip(&fills)
        .zip(&balances)
        .map(|((c, fill), sub)| SearchOption {
            fill,
            label: &c.name,
            sub,
            submit: None,
        })
        .collect();
    Ok(search_options(&options))
}
