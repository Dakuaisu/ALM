//! Review fragments. HTML source: ALM_Business_Manager.html L3373–4359 (bill entry).
//! Line amounts and totals come from `alm_core` (`priced_draft`); no arithmetic here.
use std::fmt::Write as _;

use alm_core::confirmation::ConfirmationError;
use alm_core::confirmation::owner_billing::PricedDraft;
use alm_core::{DraftCustomer, Money, ProvisionalDraft};

use crate::gst_print::{esc, inr};

fn rupees(money: Money) -> String {
    format!("₹{}", inr(money))
}

pub fn customer(draft: &ProvisionalDraft) -> &str {
    match draft.customer() {
        DraftCustomer::Unassigned => "Unassigned",
        DraftCustomer::Typed(name) | DraftCustomer::Selected { name, .. } => name,
    }
}

pub struct ReviewView<'a> {
    pub role_is_owner: bool,
    pub draft: Option<&'a ProvisionalDraft>,
    pub priced: Result<PricedDraft, ConfirmationError>,
}

pub fn review_screen(view: &ReviewView<'_>) -> String {
    let (owner, manager) = if view.role_is_owner {
        ("btn primary", "btn")
    } else {
        ("btn", "btn primary")
    };
    let mut html = format!(
        "<div class=\"card\"><h3>Owner review</h3><div class=\"row\" style=\"gap:6px;margin:8px 0\"><span class=\"help\">Synthetic role (not a login):</span><button class=\"{owner}\" data-action=\"review_role\" data-owner=\"true\">Owner</button><button class=\"{manager}\" data-action=\"review_role\" data-owner=\"false\">Manager</button></div>"
    );
    let Some(draft) = view.draft else {
        return html + "<div class=\"help\">Open or save a Manager draft first.</div></div>";
    };
    match &view.priced {
        Err(ConfirmationError::AccessDenied) => {
            html += "<div class=\"help\">Access denied: only the Owner reviews prices and totals.</div>";
        }
        Err(error) => {
            let _ = write!(
                html,
                "<div class=\"help\">Review unavailable: {error:?}</div>"
            );
        }
        Ok(priced) => {
            let _ = write!(
                html,
                "<div>Draft #{} · Customer: {}</div><table class=\"tbl\" style=\"width:100%;margin-top:8px\"><thead><tr><th>Product</th><th class=\"num\">Qty</th><th class=\"num\">Unit price</th><th class=\"num\">Amount</th></tr></thead><tbody>",
                draft.id().value(),
                esc(customer(draft))
            );
            for line in &priced.lines {
                let _ = write!(
                    html,
                    "<tr><td>Product {}</td><td class=\"num\">{}</td><td class=\"num\">{}</td><td class=\"num\">{}</td></tr>",
                    line.product_id,
                    line.quantity,
                    rupees(line.unit_price),
                    rupees(line.amount)
                );
            }
            for line in &priced.crates {
                let _ = write!(
                    html,
                    "<tr><td>📦 Crate: {}</td><td class=\"num\">{}</td><td class=\"num\">{}</td><td class=\"num\">{}</td></tr>",
                    esc(&line.name),
                    line.quantity,
                    rupees(line.rate),
                    rupees(line.amount)
                );
            }
            if !priced.loading.is_zero() {
                let _ = write!(
                    html,
                    "<tr><td>Loading</td><td></td><td></td><td class=\"num\">{}</td></tr>",
                    rupees(priced.loading)
                );
            }
            let _ = write!(
                html,
                "</tbody></table><div style=\"margin-top:8px\"><b>Owner total {}</b></div>",
                rupees(priced.total)
            );
        }
    }
    html + "</div>"
}

pub fn posting_review(
    today: alm_core::BillDate,
    posting: Option<&alm_core::confirmation::owner_billing::PostingReview>,
    saved: Option<&ProvisionalDraft>,
) -> String {
    let mut html = String::from(
        r#"<div class="card"><h3>Local Owner simulation</h3><p class="help">Development only. Owner prices can differ from the provisional draft. Confirming sends the sale through this device's outbox to the authority simulation; posted sales are kept by that simulation and shown from this device's projection.</p>"#,
    );
    let Some(saved) = saved else {
        return html + "<p>Save or reopen the Manager draft first.</p></div>";
    };
    let date = saved.bill_date().unwrap_or(today);
    let _ = write!(
        html,
        r#"<form class="row" data-action="sale_prepare" data-payload="terms"><label>Bill date <input class="inp" type="date" name="date" value="{date}" required></label><label>Paid in cash (₹) <input class="inp" name="paid" inputmode="decimal" value="20.00" required></label><button class="btn" type="submit">Review sale terms</button></form>"#
    );
    if let Some(posting) = posting {
        let review = posting.review();
        if let (Ok(amounts), Some(date), alm_core::confirmation::ProposedPayment::Named(paid)) = (
            review.amounts(),
            review.draft().bill_date(),
            review.payment(),
        ) {
            let _ = write!(
                html,
                r#"<p>Draft #{} · {} · {} · Total <b>{}</b> · Paid (cash) <b>{}</b></p><button class="btn primary" data-action="sale_confirm" data-id="{}">Confirm simulated sale</button>"#,
                review.draft().id().value(),
                esc(customer(review.draft())),
                date.display(),
                rupees(amounts.total),
                rupees(paid),
                posting.serial()
            );
        }
    }
    html + "</div>"
}
