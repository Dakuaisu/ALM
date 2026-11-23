//! Draft and review fragments. HTML source: ALM_Business_Manager.html L3373–4359 (bill entry).
use std::fmt::Write as _;

use alm_core::confirmation::ConfirmationError;
use alm_core::{DraftCustomer, Money, ProvisionalDraft, provisional_preview};

use crate::gst_print::{esc, inr};

pub struct DraftView<'a> {
    pub draft: Option<&'a ProvisionalDraft>,
    pub saved: bool,
    pub saved_ids: &'a [u64],
}

fn rupees(money: Money) -> String {
    format!("₹{}", inr(money))
}

pub fn customer(draft: &ProvisionalDraft) -> &str {
    match draft.customer() {
        DraftCustomer::Unassigned => "Unassigned",
        DraftCustomer::Typed(name) | DraftCustomer::Selected { name, .. } => name,
    }
}

pub fn draft_screen(view: &DraftView<'_>) -> String {
    let mut html = String::from(
        "<div class=\"card\"><h3>Manager draft — PROVISIONAL, NOT POSTED</h3>\
         <form class=\"row\" data-action=\"draft_new\" style=\"gap:8px;margin:8px 0\">\
         <input class=\"inp\" name=\"customer\" value=\"Synthetic Customer\" aria-label=\"Customer\">\
         <button class=\"btn primary\" type=\"submit\">＋ New draft</button></form>",
    );
    if !view.saved_ids.is_empty() {
        html += "<div class=\"row\" style=\"gap:6px;margin-bottom:8px\"><span class=\"help\">Saved drafts:</span>";
        for id in view.saved_ids {
            let _ = write!(
                html,
                "<button class=\"btn sm\" data-action=\"draft_open\" data-id=\"{id}\">Open #{id}</button>"
            );
        }
        html += "</div>";
    }
    let Some(draft) = view.draft else {
        return html
            + "<div class=\"help\">No draft open. Start one above or open a saved draft.</div></div>";
    };
    let state = if view.saved { "SAVED" } else { "UNSAVED" };
    let _ = write!(
        html,
        "<div class=\"row\" style=\"gap:10px\"><b>Draft #{}</b><span class=\"tag\">{state}</span><span>Customer: {}</span></div>",
        draft.id().value(),
        esc(customer(draft))
    );
    match provisional_preview::preview(draft) {
        Ok(preview) => {
            html += "<table class=\"tbl\" style=\"width:100%;margin-top:8px\"><thead><tr><th>#</th><th>Item</th><th class=\"num\">Qty</th><th class=\"num\">Unit (incl.)</th><th class=\"num\">Line total</th><th></th></tr></thead><tbody>";
            for (index, line) in preview.lines.iter().enumerate() {
                let number = index + 1;
                let _ = write!(
                    html,
                    "<tr><td>{number}</td><td>{}</td><td class=\"num\"><form data-action=\"draft_quantity\" data-line=\"{number}\" class=\"row\" style=\"gap:4px;justify-content:flex-end\"><input class=\"inp\" style=\"width:80px\" name=\"quantity\" inputmode=\"numeric\" value=\"{}\" aria-label=\"Quantity for line {number}\"><button class=\"btn sm\" type=\"submit\">Set</button></form></td><td class=\"num\">{}</td><td class=\"num\">{}</td><td></td></tr>",
                    esc(line.name),
                    line.quantity,
                    rupees(line.inclusive_unit_price),
                    rupees(line.amounts.inclusive),
                );
            }
            let _ = write!(
                html,
                "</tbody></table><div style=\"margin-top:8px\">Taxable {} · CGST {} · SGST {} · <b>Total {}</b></div>",
                rupees(preview.totals.taxable),
                rupees(preview.totals.cgst),
                rupees(preview.totals.sgst),
                rupees(preview.totals.inclusive)
            );
        }
        Err(error) => {
            let _ = write!(
                html,
                "<div class=\"help\">Preview unavailable: {error:?}</div>"
            );
        }
    }
    html + "<div class=\"row\" style=\"margin-top:10px\"><button class=\"btn primary\" data-action=\"draft_save\">Save draft</button></div></div>"
}

pub struct ReviewView<'a> {
    pub role_is_owner: bool,
    pub draft: Option<&'a ProvisionalDraft>,
    pub prices: Result<Vec<Money>, ConfirmationError>,
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
    match &view.prices {
        Err(ConfirmationError::AccessDenied) => {
            html += "<div class=\"help\">Access denied: only the Owner reviews prices and totals.</div>";
        }
        Err(error) => {
            let _ = write!(
                html,
                "<div class=\"help\">Review unavailable: {error:?}</div>"
            );
        }
        Ok(prices) => {
            let _ = write!(
                html,
                "<div>Draft #{} · Customer: {}</div><table class=\"tbl\" style=\"width:100%;margin-top:8px\"><thead><tr><th>Product</th><th class=\"num\">Qty</th><th class=\"num\">Unit price</th><th class=\"num\">Amount</th></tr></thead><tbody>",
                draft.id().value(),
                esc(customer(draft))
            );
            let mut total = 0_i64;
            for (line, price) in draft.lines().iter().zip(prices) {
                let amount = price.minor_units() * i64::from(line.quantity().get());
                total += amount;
                let _ = write!(
                    html,
                    "<tr><td>Product {}</td><td class=\"num\">{}</td><td class=\"num\">{}</td><td class=\"num\">{}</td></tr>",
                    line.product_id(),
                    line.quantity(),
                    rupees(*price),
                    rupees(Money::from_minor_units(amount))
                );
            }
            let _ = write!(
                html,
                "</tbody></table><div style=\"margin-top:8px\"><b>Owner total {}</b></div>",
                rupees(Money::from_minor_units(total))
            );
        }
    }
    html + "</div>"
}

pub fn posting_review(
    posting: Option<&alm_core::confirmation::trial_billing::PostingReview>,
    saved: bool,
) -> String {
    let mut html = String::from(
        r#"<div class="card"><h3>Local Owner simulation</h3><p class="help">Development only. Owner prices can differ from the provisional draft. Sales are not saved across restarts.</p>"#,
    );
    if !saved {
        return html + "<p>Save or reopen the Manager draft first.</p></div>";
    }
    let date = alm_core::confirmation::trial_billing::DAY;
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
                r#"<p>Draft #{} · {} · {date} · Total <b>{}</b> · Paid (cash) <b>{}</b></p><button class="btn primary" data-action="sale_confirm" data-id="{}">Confirm simulated sale</button>"#,
                review.draft().id().value(),
                esc(customer(review.draft())),
                rupees(amounts.total),
                rupees(paid),
                posting.serial()
            );
        }
    }
    html + "</div>"
}
