//! Receive Payment: ALM_Business_Manager.html L5860–5965.
use std::fmt::Write as _;

use alm_core::confirmation::owner_billing::BillingDirectory;
use alm_core::confirmation::{ConfirmationError, ConfirmationFixture};
use alm_core::{BillDate, DraftCustomerId, Role};

use super::{denied, money_r};
use crate::gst_print::esc;

pub fn receipt_screen(
    fixture: &ConfirmationFixture,
    directory: &BillingDirectory,
    role: Role,
    today: BillDate,
) -> Result<String, ConfirmationError> {
    if role != Role::Owner {
        return Ok(denied("Receive Payment"));
    }
    let mut options = String::new();
    for customer in directory.customers(fixture) {
        let ledger = fixture.customer_ledger(role, &DraftCustomerId::new(customer.id.into()))?;
        let _ = write!(
            options,
            r#"<option value="{}">{} · balance {}</option>"#,
            esc(customer.id),
            esc(customer.name),
            money_r(ledger.closing)
        );
    }
    Ok(format!(
        r#"<div class="card"><p class="help">Local simulation only. This payment is dated independently of any bill.</p>
    <form data-action="receipt_save" data-payload="terms"><div class="field"><label>Customer <select class="inp" name="customer" required>{options}</select></label></div>
    <div class="f2"><div class="field"><label>Amount ₹ <input class="inp num" name="amount" inputmode="decimal" required></label></div>
    <div class="field"><label>Mode <select class="inp" name="mode"><option>Cash</option><option>Online</option></select></label></div></div>
    <div class="field"><label>Date <input class="inp" type="date" name="date" value="{today}" required></label></div>
    <button class="btn primary" type="submit">Save Payment</button></form></div>"#
    ))
}
