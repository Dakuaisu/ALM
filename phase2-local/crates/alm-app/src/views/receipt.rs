//! Receive Payment: ALM_Business_Manager.html L5860–5965.
use alm_core::Role;
use alm_core::confirmation::{ConfirmationError, ConfirmationFixture};

use super::{denied, money_r};
use crate::billing::{CUSTOMER, customer_id};

pub fn receipt_screen(
    fixture: &ConfirmationFixture,
    role: Role,
) -> Result<String, ConfirmationError> {
    let ledger = match fixture.customer_ledger(role, &customer_id()) {
        Ok(ledger) => ledger,
        Err(ConfirmationError::AccessDenied) => return Ok(denied("Receive Payment")),
        Err(error) => return Err(error),
    };
    let balance = money_r(ledger.closing);
    Ok(format!(
        r#"<div class="card"><p class="help">Local simulation only. This payment is dated independently of any bill.</p>
    <form data-action="receipt_save" data-payload="terms"><div class="field"><label>Customer <input class="inp" value="{CUSTOMER}" readonly></label><div class="help">Current balance: <b>{balance}</b></div></div>
    <div class="f2"><div class="field"><label>Amount ₹ <input class="inp num" name="amount" inputmode="decimal" required></label></div>
    <div class="field"><label>Mode <select class="inp" name="mode"><option>Cash</option><option>Online</option></select></label></div></div>
    <div class="field"><label>Date <input class="inp" type="date" name="date" value="2025-05-01" required></label></div>
    <button class="btn primary" type="submit">Save Payment</button></form></div>"#
    ))
}
