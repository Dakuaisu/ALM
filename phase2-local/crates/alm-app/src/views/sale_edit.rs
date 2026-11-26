//! Edit Bill: ALM_Business_Manager.html L4129–4205 (editBill → buildBillForm → saveBill). Each
//! existing line's quantity and rate are edited in place; core `edit_posted_lines` recalculates
//! stock, lots, balance and totals.
use std::fmt::Write as _;

use alm_core::confirmation::owner_billing::BillingDirectory;
use alm_core::confirmation::{BalanceMode, ConfirmationError, ConfirmationFixture};
use alm_core::{DraftId, Role};

use super::money_r;
use crate::gst_print::esc;

pub fn sale_edit_form(
    fixture: &ConfirmationFixture,
    directory: &BillingDirectory,
    role: Role,
    id: DraftId,
) -> Result<String, ConfirmationError> {
    let bill = fixture.bill_preview(role, id, BalanceMode::AsOnBillDate)?;
    let mut html = format!(
        r#"<div class="help" style="margin-bottom:10px">Editing bill #{} — {}. Saving replaces it and recalculates stock and the customer's balance.</div><form data-action="sale_edit_save" data-payload="terms"><input type="hidden" name="id" value="{}"><div class="tbl-wrap"><table><thead><tr><th>Item</th><th class="num">Qty</th><th class="num">Rate ₹</th><th class="num">Amount</th></tr></thead><tbody>"#,
        bill.invoice,
        bill.date.display(),
        id
    );
    for (i, line) in bill.lines.iter().enumerate() {
        let name = line.label.clone().unwrap_or_else(|| {
            directory
                .products(fixture)
                .find(|(product, _)| *product == line.product)
                .map_or_else(|| line.product.to_string(), |(_, name)| name.to_owned())
        });
        let _ = write!(
            html,
            r#"<tr><td>{}</td><td class="num"><input class="inp num" name="qty_{i}" inputmode="numeric" aria-label="Edit quantity {}" value="{}"></td><td class="num"><input class="inp num" name="rate_{i}" inputmode="decimal" aria-label="Edit rate {}" value="{}"></td><td class="num">{}</td></tr>"#,
            esc(&name),
            esc(&name),
            line.quantity,
            esc(&name),
            line.price.to_rupee_text(),
            money_r(line.amount)
        );
    }
    let _ = write!(
        html,
        r#"</tbody></table></div><div class="help">Paid {} stays as entered on the bill.</div><div class="row" style="justify-content:flex-end;margin-top:10px"><button class="btn" type="button" data-action="sale_edit_close">Cancel</button><button class="btn primary" type="submit">Save Changes</button></div></form>"#,
        money_r(bill.paid)
    );
    Ok(super::modal(
        "🧾",
        &format!("Edit Bill #{}", bill.invoice),
        "sale_edit_close",
        true,
        &html,
    ))
}
