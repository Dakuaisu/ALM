//! ALM_Business_Manager.html L3452–3600 (cards), L3611–3753 (items), L3819–3942 (totals), L3946–3986 (actions).
use std::fmt::Write as _;

use alm_core::confirmation::ConfirmationFixture;
use alm_core::confirmation::owner_billing::BillingDirectory;
use alm_core::provisional_preview::{PreviewAmounts, ProvisionalPreviewLine};
use alm_core::{DraftCustomer, ProvisionalDraft, Role};

use crate::gst_print::{esc, inr};

pub struct BillView<'a> {
    pub authority: &'a ConfirmationFixture,
    pub directory: &'a BillingDirectory,
    pub draft: &'a ProvisionalDraft,
    pub saved: bool,
    pub saved_ids: &'a [u64],
    pub role: Role,
}

pub fn bill_screen(view: &BillView<'_>) -> String {
    let mut html = String::from(
        r#"<div class="bill-form" style="max-width:1050px;margin:0 auto"><div class="grid" style="grid-template-columns:1fr">"#,
    );
    header(&mut html, view);
    html += &bill_body(view);
    html += r#"<div class="row" style="justify-content:flex-end;gap:10px"><button class="btn" type="button" data-action="bill_fresh">Clear</button><button class="btn primary" type="submit" form="bill-header" name="intent" value="save" style="padding:11px 22px">Save Bill as Draft</button>"#;
    if view.role == Role::Owner && view.saved {
        html += r#"<button class="btn" type="button" data-action="screen" data-tab="review">Owner review</button>"#;
    }
    html + "</div></div></div>"
}

/// Items and totals, returned alone for live quantity updates (`#bill-body`).
pub fn bill_body(view: &BillView<'_>) -> String {
    let mut html = String::from(r#"<div id="bill-body">"#);
    let preview = view.directory.preview(view.authority, view.draft);
    html += r#"<div class="card"><div class="row" style="justify-content:space-between"><div class="sectit" style="margin:0">Items</div><button type="submit" form="bill-item" class="btn sm">＋ Add Item</button></div><div class="bill-items" style="margin-top:12px">"#;
    html += r#"<div class="bill-line bill-labels"><div>Item</div><div>Unit</div><div class="num">Rate ₹</div><div class="num">Qty</div><div class="num">Amount</div><div></div></div>"#;
    if let Ok(preview) = &preview {
        for (index, line) in preview.lines.iter().enumerate() {
            item_row(&mut html, index + 1, line);
        }
    }
    html += r#"<form id="bill-item" class="bill-line" data-action="bill_add" data-payload="line"><div class="autobox"><input class="inp" role="combobox" aria-autocomplete="list" aria-expanded="false" aria-controls="bill-item-list" id="bill-item-q" name="product_q" data-search="bill_item_search" placeholder="Type item…" aria-label="Item" autocomplete="off"><input type="hidden" name="product" value=""><div class="autolist" id="bill-item-list" role="listbox" aria-label="Item suggestions"></div></div><span class="muted">Packet</span><span class="num muted">—</span><input class="inp num" name="quantity" inputmode="numeric" value="1" aria-label="New item quantity" required><span class="num muted">—</span><span></span></form></div></div>"#;

    let with_loading = preview.and_then(|preview| {
        alm_core::provisional_preview::with_loading(view.draft, &preview.totals)
            .map(|charges| (preview, charges))
    });
    match with_loading {
        Ok((preview, charges)) => totals(&mut html, &preview.totals, view.draft, charges),
        Err(error) => {
            let _ = write!(
                html,
                r#"<div class="card" role="alert">Preview unavailable: {error:?}. Clear the draft to start again.</div>"#
            );
        }
    }
    html + "</div>"
}

fn header(html: &mut String, view: &BillView<'_>) {
    let id = view.draft.id().value();
    let status = if view.saved { "SAVED" } else { "UNSAVED" };
    let date = view
        .draft
        .bill_date()
        .map_or_else(String::new, |date| date.to_string());
    let (selected, name) = match view.draft.customer() {
        DraftCustomer::Selected { id, name } => (id.value(), name.as_str()),
        DraftCustomer::Typed(_) | DraftCustomer::Unassigned => ("", ""),
    };
    let _ = write!(
        html,
        r#"<div class="card"><form id="bill-header" class="f3" data-action="bill_header" data-payload="header"><div class="autobox"><label for="bill-customer">Customer / Party</label><input id="bill-customer" class="inp" role="combobox" aria-autocomplete="list" aria-expanded="false" aria-controls="bill-customer-list" name="customer_q" data-search="bill_customer_search" placeholder="Search customer…" autocomplete="off" value="{}"><input type="hidden" name="customer" value="{}"><div class="autolist" id="bill-customer-list" role="listbox" aria-label="Customer suggestions"></div></div>"#,
        esc(name),
        esc(selected)
    );
    let _ = write!(
        html,
        r#"<div><label for="bill-date">Date</label><input id="bill-date" class="inp" type="date" name="date" value="{date}" required></div><div><label>Payment Mode</label><div class="help">Set during Owner review</div></div><div><button class="btn sm" type="submit" name="intent" value="select">Set customer and date</button></div></form><div class="row" style="margin-top:12px;gap:8px"><b>Draft #{id}</b><span class="tag">{status}</span><span class="help">Provisional · Not posted</span></div>"#
    );
    if !view.saved_ids.is_empty() {
        html.push_str(r#"<div class="row" style="margin-top:8px;gap:6px"><span class="help">Saved drafts:</span>"#);
        for id in view.saved_ids {
            let _ = write!(
                html,
                r#"<button class="btn sm" type="button" data-action="bill_open" data-id="{id}">Open #{id}</button>"#
            );
        }
        html.push_str("</div>");
    }
    html.push_str("</div>");
}

fn item_row(html: &mut String, index: usize, line: &ProvisionalPreviewLine) {
    let _ = write!(
        html,
        r#"<form class="bill-line" data-action="bill_quantity" data-line="{index}" data-product="{}" data-live="bill_live" data-target="bill-body"><input type="hidden" name="line" value="{index}"><b>{}</b><span class="muted">Packet</span><span class="num">₹{}</span><input class="inp num" id="bill-qty-{index}" name="qty" inputmode="numeric" value="{}" aria-label="Quantity for item {index}" required><span class="num"><b>₹{}</b></span><span class="row" style="gap:4px"><button class="btn sm" type="submit">Set</button><button class="x" type="button" data-action="bill_remove" data-id="{index}" aria-label="Remove item {index}">×</button></span></form>"#,
        line.product_id,
        esc(line.name),
        inr(line.inclusive_unit_price),
        line.quantity,
        inr(line.amounts.inclusive)
    );
}

fn totals(
    html: &mut String,
    amounts: &PreviewAmounts,
    draft: &ProvisionalDraft,
    (loading, grand): (alm_core::Money, alm_core::Money),
) {
    html.push_str(r#"<div class="card"><div class="row" style="align-items:flex-start;gap:24px"><div style="flex:1"><div class="sectit">Provisional totals</div><div class="help">Saving a draft does not post a sale or record payment.</div>"#);
    let rate = draft.loading_rate();
    let _ = write!(
        html,
        r#"<div style="margin-top:12px;max-width:220px"><label for="bill-loading">Loading/Delivery ₹ per packet</label><input id="bill-loading" class="inp num" form="bill-header" name="loading" inputmode="decimal" value="{}"></div></div><div class="bill-totals">"#,
        if rate.is_zero() {
            "0".to_owned()
        } else {
            rate.to_rupee_text()
        }
    );
    let loading_row = (!loading.is_zero()).then_some(("Loading", loading));
    for (label, amount) in [
        Some(("Items total", amounts.inclusive)),
        Some(("Taxable", amounts.taxable)),
        Some(("CGST", amounts.cgst)),
        Some(("SGST", amounts.sgst)),
        loading_row,
        Some(("Bill Amount", grand)),
    ]
    .into_iter()
    .flatten()
    {
        if label == "Bill Amount" {
            html.push_str(r#"<hr class="sep" style="margin:7px 0">"#);
        }
        let _ = write!(
            html,
            r#"<div class="row" style="justify-content:space-between;padding:5px 0;gap:12px"><span class="muted">{label}</span><b>₹{}</b></div>"#,
            inr(amount)
        );
    }
    html.push_str("</div></div></div>");
}
