//! ALM_Business_Manager.html L3452–3600 (cards), L3611–3753 (items), L3819–3942 (totals), L3946–3986 (actions).
use std::fmt::Write as _;

use alm_core::confirmation::ConfirmationFixture;
use alm_core::confirmation::owner_billing::{BillingDirectory, NewBillCrate, NewBillPricing};
use alm_core::provisional_preview::ProvisionalPreviewLine;
use alm_core::{DraftCustomer, ProvisionalDraft, Role};

use crate::gst_print::{esc, inr};

pub struct BillView<'a> {
    pub authority: &'a ConfirmationFixture,
    pub directory: &'a BillingDirectory,
    pub draft: &'a ProvisionalDraft,
    pub saved: bool,
    pub saved_ids: &'a [alm_core::DraftId],
    pub role: Role,
    /// Saved crate templates New Bill may add (`(id, name)`).
    pub crates: &'a [(u64, String)],
    /// A Manager's pulled catalog (names and sale prices), which its preview prices from.
    pub catalog: Option<&'a [alm_core::projection::ProjectedProduct]>,
    /// A Manager's saved bill as the authority will price it (selling rates), for its Post form.
    pub posts_at: Option<alm_core::Money>,
}

pub fn bill_screen(view: &BillView<'_>) -> String {
    let mut html = String::from(
        r#"<div class="bill-form" style="max-width:1050px;margin:0 auto"><div class="grid" style="grid-template-columns:1fr">"#,
    );
    header(&mut html, view);
    html += &bill_body(view);
    html += r#"<div class="row" style="justify-content:flex-end;gap:10px"><button class="btn" type="button" data-action="bill_fresh">Clear</button><button class="btn primary" type="submit" form="bill-header" name="intent" value="save" style="padding:11px 22px">Save Bill as Draft</button>"#;
    if view.role == Role::Owner && view.saved && !single_crate(view.draft) {
        html += r#"<button class="btn" type="button" data-action="screen" data-tab="review">Owner review</button>"#;
    }
    html += "</div>";
    // DEC-034: a Manager posts its saved bill itself; the authority re-checks it.
    if let (Role::Manager, true, false, Some(total)) = (
        view.role,
        view.saved,
        single_crate(view.draft),
        view.posts_at,
    ) {
        let _ = write!(
            html,
            r#"<form class="row" style="justify-content:flex-end;gap:8px;margin-top:10px" data-action="bill_post" data-payload="terms"><span class="help">Posts at today's rates: <b>₹{}</b></span><label>Paid ₹ <input class="inp num" name="paid" inputmode="decimal" value="0" aria-label="Amount paid" style="width:110px"></label><button class="btn primary" type="submit">Post bill</button></form>"#,
            inr(total)
        );
    }
    html + "</div></div>"
}

/// A bill of one crate row and no items, posted through the crate-sale command.
fn single_crate(draft: &ProvisionalDraft) -> bool {
    draft.lines().is_empty() && draft.crate_rows().len() == 1
}

/// Items and totals, returned alone for live quantity updates (`#bill-body`).
pub fn bill_body(view: &BillView<'_>) -> String {
    if let ([row], true) = (view.draft.crate_rows(), view.draft.lines().is_empty()) {
        return crate_body(view, row);
    }
    let mut html = String::from(r#"<div id="bill-body">"#);
    // One pricing result for every row, the loading charge and the bill amount (`lineAmt`).
    let pricing = view
        .directory
        .new_bill_pricing(view.authority, view.catalog, view.draft);
    html += r#"<div class="card"><div class="row" style="justify-content:space-between"><div class="sectit" style="margin:0">Items</div><button type="submit" form="bill-item" class="btn sm">＋ Add Item</button></div><div class="bill-items" style="margin-top:12px">"#;
    html += r#"<div class="bill-line bill-labels"><div>Item</div><div>Unit</div><div class="num">Rate ₹</div><div class="num">Qty</div><div class="num">Amount</div><div></div></div>"#;
    match &pricing {
        Ok(pricing) => {
            for (index, line) in pricing.items.lines.iter().enumerate() {
                item_row(&mut html, index + 1, line, true);
            }
        }
        Err(_) => {
            // Without a total, rows show no amount; their names and rates still come from core.
            let ordinary = ProvisionalDraft::new(view.draft.id(), view.draft.lines().to_vec());
            let names = match view.catalog {
                Some(catalog) => view.directory.catalog_preview(catalog, &ordinary),
                None => view.directory.preview(view.authority, &ordinary),
            };
            if let Ok(names) = names {
                for (index, line) in names.lines.iter().enumerate() {
                    item_row(&mut html, index + 1, line, false);
                }
            }
        }
    }
    for (index, row) in view.draft.crate_rows().iter().enumerate() {
        let priced = pricing
            .as_ref()
            .ok()
            .and_then(|pricing| pricing.crates.get(index));
        crate_line(&mut html, view, index, row, priced);
    }
    html += r#"<form id="bill-item" class="bill-line" data-action="bill_add" data-payload="line"><div class="autobox"><input class="inp" role="combobox" aria-autocomplete="list" aria-expanded="false" aria-controls="bill-item-list" id="bill-item-q" name="product_q" data-search="bill_item_search" placeholder="Type item…" aria-label="Item" autocomplete="off"><input type="hidden" name="product" value=""><div class="autolist" id="bill-item-list" role="listbox" aria-label="Item suggestions"></div></div><span class="muted">Packet</span><span class="num muted">—</span><input class="inp num" name="quantity" inputmode="numeric" value="1" aria-label="New item quantity" required><span class="num muted">—</span><span></span></form>"#;
    add_crate(&mut html, view.crates);
    html += "</div></div>";
    match pricing {
        Ok(pricing) => totals(&mut html, view.draft, &pricing),
        Err(error) => {
            let _ = write!(
                html,
                r#"<div class="card" role="alert">Totals unavailable: {}</div>"#,
                esc(&crate::refusals::pricing(&error))
            );
        }
    }
    html + "</div>"
}

/// "📦 Add Crate": a saved template and its number of crates (Owner only).
fn add_crate(html: &mut String, crates: &[(u64, String)]) {
    if crates.is_empty() {
        return;
    }
    let mut options = String::new();
    for (id, name) in crates {
        let _ = write!(options, r#"<option value="{id}">{}</option>"#, esc(name));
    }
    let _ = write!(
        html,
        r#"<form class="bill-line" data-action="bill_crate_add" data-payload="line"><select class="inp" name="template" aria-label="Crate template" required><option value="">📦 Choose a crate…</option>{options}</select><span class="muted">Crate</span><span class="num muted">—</span><input class="inp num" name="quantity" inputmode="numeric" value="1" aria-label="Crates" required><span class="num muted">—</span><button class="btn sm" type="submit">📦 Add Crate</button></form>"#
    );
}

/// One crate row (`crateRow`): name, contents, Crate, rate, crates, amount and ×.
fn crate_line(
    html: &mut String,
    view: &BillView<'_>,
    index: usize,
    row: &alm_core::CrateRow,
    priced: Option<&NewBillCrate>,
) {
    let name = row.name().unwrap_or("Crate");
    let rate = priced
        .map(|p| p.rate)
        .or(row.rate())
        .map_or_else(|| "—".to_owned(), inr);
    let line = priced.map_or_else(|| "—".to_owned(), |p| format!("<b>₹{}</b>", inr(p.amount)));
    let _ = write!(
        html,
        r#"<form class="bill-line" style="background:var(--bg2);border-radius:9px;padding:6px 4px" data-action="bill_crate_quantity" data-payload="terms" data-line="{index}"><div style="padding-left:6px"><b>📦 {}</b><div class="muted" style="font-size:11px">{}</div></div><span class="muted">Crate</span><span class="num">₹{rate}</span><input class="inp num" name="quantity" inputmode="numeric" value="{}" aria-label="Crates for {}" required><span class="num">{line}</span><span class="row" style="gap:4px"><button class="btn sm" type="submit">Set</button><button class="x" type="button" data-action="bill_crate_remove" data-line="{index}" aria-label="Remove crate {}">×</button></span></form>"#,
        esc(name),
        esc(&view.authority.crate_row_contents(row)),
        row.quantity(),
        esc(name),
        esc(name)
    );
}

/// A bill of one crate row: the crate row (`crateRow`) and, once the draft is saved, the Owner's
/// post form.
fn crate_body(view: &BillView<'_>, row: &alm_core::CrateRow) -> String {
    let mut html = String::from(
        r#"<div id="bill-body"><div class="card"><div class="row" style="justify-content:space-between"><div class="sectit" style="margin:0">Items</div><button type="submit" form="bill-item" class="btn sm">＋ Add Item</button></div><div class="bill-items" style="margin-top:12px"><div class="bill-line bill-labels"><div>Item</div><div>Unit</div><div class="num">Rate ₹</div><div class="num">Qty</div><div class="num">Amount</div><div></div></div>"#,
    );
    let amounts = view.authority.crate_line_amounts(view.role, row);
    let priced = amounts.as_ref().ok().map(|a| NewBillCrate {
        rate: row.rate().unwrap_or_else(|| {
            alm_core::Money::from_minor_units(
                a.subtotal.minor_units() / i64::from(row.quantity().get()),
            )
        }),
        amount: a.subtotal,
    });
    crate_line(&mut html, view, 0, row, priced.as_ref());
    html += r#"<form id="bill-item" class="bill-line" data-action="bill_add" data-payload="line"><div class="autobox"><input class="inp" role="combobox" aria-autocomplete="list" aria-expanded="false" aria-controls="bill-item-list" id="bill-item-q" name="product_q" data-search="bill_item_search" placeholder="Type item…" aria-label="Item" autocomplete="off"><input type="hidden" name="product" value=""><div class="autolist" id="bill-item-list" role="listbox" aria-label="Item suggestions"></div></div><span class="muted">Packet</span><span class="num muted">—</span><input class="inp num" name="quantity" inputmode="numeric" value="1" aria-label="New item quantity" required><span class="num muted">—</span><span></span></form>"#;
    add_crate(&mut html, view.crates);
    html += "</div></div>";
    match amounts {
        Ok(amounts) => {
            html.push_str(r#"<div class="card"><div class="row" style="align-items:flex-start;gap:24px"><div style="flex:1"><div class="sectit">Totals</div><div class="help">The crate sells as one line at its rate; stock reduces item by item. Loading is charged per packet of its contents.</div>"#);
            if view.saved {
                html.push_str(r#"<form class="row" style="gap:8px;margin-top:12px" data-action="bill_crate_post" data-payload="terms"><label>Paid ₹ <input class="inp num" name="paid" inputmode="decimal" value="0" aria-label="Amount paid" style="width:110px"></label><button class="btn primary" type="submit">Post crate bill</button></form>"#);
            }
            html.push_str(r#"</div><div class="bill-totals">"#);
            let loading = (!amounts.loading.is_zero()).then_some(("Loading", amounts.loading));
            for (label, amount) in [
                Some(("Items total", amounts.subtotal)),
                loading,
                Some(("Bill Amount", amounts.total)),
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
        Err(error) => {
            let _ = write!(
                html,
                r#"<div class="card" role="alert">Crate totals unavailable: {}</div>"#,
                esc(&crate::refusals::crate_pricing(&error))
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
                r#"<button class="btn sm" type="button" data-action="bill_open" data-id="{id}">Open #{}</button>"#,
                id.value()
            );
        }
        html.push_str("</div>");
    }
    html.push_str("</div>");
}

fn item_row(html: &mut String, index: usize, line: &ProvisionalPreviewLine, priced: bool) {
    let amount = if priced {
        format!("<b>₹{}</b>", inr(line.amounts.inclusive))
    } else {
        "—".to_owned()
    };
    let _ = write!(
        html,
        r#"<form class="bill-line" data-action="bill_quantity" data-line="{index}" data-product="{}" data-live="bill_live" data-target="bill-body"><input type="hidden" name="line" value="{index}"><b>{}</b><span class="muted">Packet</span><span class="num">₹{}</span><input class="inp num" id="bill-qty-{index}" name="qty" inputmode="numeric" value="{}" aria-label="Quantity for item {index}" required><span class="num">{amount}</span><span class="row" style="gap:4px"><button class="btn sm" type="submit">Set</button><button class="x" type="button" data-action="bill_remove" data-id="{index}" aria-label="Remove item {index}">×</button></span></form>"#,
        line.product_id,
        esc(&line.name),
        inr(line.inclusive_unit_price),
        line.quantity,
    );
}

fn totals(html: &mut String, draft: &ProvisionalDraft, pricing: &NewBillPricing) {
    let mixed = !pricing.crates.is_empty();
    html.push_str(r#"<div class="card"><div class="row" style="align-items:flex-start;gap:24px"><div style="flex:1"><div class="sectit">Provisional totals</div><div class="help">Saving a draft does not post a sale or record payment."#);
    if mixed {
        html.push_str(" Each crate sells as one line at its rate; stock reduces item by item and loading is charged per packet of its contents.");
    }
    html.push_str("</div>");
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
    let gst = &pricing.items.totals;
    let loading_row = (!pricing.loading.is_zero()).then_some(("Loading", pricing.loading));
    for (label, amount) in [
        Some(("Items total", pricing.items_total)),
        (!mixed).then_some(("Taxable", gst.taxable)),
        (!mixed).then_some(("CGST", gst.cgst)),
        (!mixed).then_some(("SGST", gst.sgst)),
        loading_row,
        Some(("Bill Amount", pricing.total)),
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
