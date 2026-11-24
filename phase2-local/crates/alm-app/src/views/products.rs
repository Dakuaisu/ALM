//! Products: ALM_Business_Manager.html L7462–7580 (VIEWS.products) and the editProduct form
//! (L7582–7950, name/brand/buy/sell/reorder only, inline). Rows, margin %, status and brand chips
//! come from `product_list`; prices from `ProductEntry::pricing`.
use std::fmt::Write as _;

use alm_core::Role;
use alm_core::confirmation::{ConfirmationFixture, ProductEntry, ProductEntryError, StockStatus};

use super::{denied, money_r};
use crate::gst_print::{esc, indian_digits};

#[derive(Clone, Debug, Default)]
pub struct ProductsState {
    pub query: String,
    pub brand: Option<String>,
    pub form: Option<ProductEntry>,
}

pub fn products_screen(
    fixture: &ConfirmationFixture,
    role: Role,
    state: &ProductsState,
) -> Result<String, ProductEntryError> {
    let list = match fixture.product_list(role, &state.query, state.brand.as_deref()) {
        Ok(list) => list,
        Err(ProductEntryError::AccessDenied) => return Ok(denied("Products")),
        Err(error) => return Err(error),
    };
    let mut html = String::new();
    if let Some(form) = &state.form {
        html += &edit_form(form);
    }
    let _ = write!(
        html,
        r#"<form class="row" style="margin-bottom:12px" data-action="products_filter" data-payload="terms"><div class="search" style="flex:1;min-width:200px"><span class="si">🔍</span><input class="inp" name="q" placeholder="Search product…" aria-label="Search product" value="{}"></div><button class="btn" type="submit" name="brand" value="">All</button>"#,
        esc(&state.query)
    );
    for brand in &list.brands {
        let on = if state.brand.as_deref() == Some(brand) {
            "pill on"
        } else {
            "pill"
        };
        let _ = write!(
            html,
            r#"<button class="{on}" type="submit" name="brand" value="{0}">{0}</button>"#,
            esc(brand)
        );
    }
    html += r#"</form><div class="tbl-wrap" style="max-height:60vh"><table><thead><tr><th>Product</th><th class="num">Buy</th><th class="num">Sell</th><th class="num">Margin</th><th class="num">Stock</th><th class="num">Actions</th></tr></thead><tbody>"#;
    if list.rows.is_empty() {
        html += r#"<tr><td colspan="6"><div class="empty">🥤<br>No products</div></td></tr>"#;
    }
    for row in &list.rows {
        let low = match row.status {
            StockStatus::Ok => "",
            StockStatus::Low | StockStatus::Out => r#" <span class="tag low">low</span>"#,
        };
        let _ = write!(
            html,
            r#"<tr><td><b>{}</b><div class="muted" style="font-size:11px">{}</div></td><td class="num">{}</td><td class="num">{}</td><td class="num">{}</td><td class="num"><span>{} pkt</span>{low}</td><td class="num"><button class="btn sm" data-action="product_edit" data-id="{}" aria-label="Edit {}">✎</button></td></tr>"#,
            esc(&row.item),
            esc(&row.brand),
            money_r(row.buy),
            money_r(row.sell),
            row.margin_percent.as_deref().unwrap_or("—"),
            indian_digits(u64::from(row.on_hand)),
            row.product,
            esc(&row.item)
        );
    }
    Ok(html + "</tbody></table></div>")
}

/// The edit card alone, so a live price preview can replace just this fragment.
pub fn product_form(form: &ProductEntry) -> String {
    edit_form(form)
}

fn edit_form(form: &ProductEntry) -> String {
    let pricing = form.pricing();
    let reorder = form
        .reorder_level
        .map_or_else(String::new, |r| r.to_string());
    // The HTML asks "Delete <item>?" before removing; the second press confirms.
    let delete = if form.target().1 {
        String::new()
    } else {
        format!(
            r#"<button class="btn danger" type="button" style="margin-right:auto" data-action="product_delete" data-q="ask" aria-label="Delete {0}">Delete</button><span class="help" data-delete-ask hidden>Delete {0}? <button class="btn sm danger" type="button" data-action="product_delete" data-q="confirm">Yes, delete</button></span>"#,
            esc(&form.item)
        )
    };
    format!(
        r#"<div class="card" id="product-form" style="margin-bottom:14px"><h3>🥤 Product</h3><form data-action="product_save" data-payload="terms" data-live="product_preview" data-target="product-form"><div class="f2"><div><label>Item name * <input class="inp" name="item" value="{}"></label></div><div><label>Company / Brand <input class="inp" name="brand" value="{}"></label></div></div><div class="f3"><div><label>Buy Rate ₹ <input class="inp num" name="buy" inputmode="decimal" value="{}"></label></div><div><label>Margin ₹ <input class="inp num" name="margin" inputmode="decimal" value="{}"></label></div><div><label>Sell Rate ₹ <input class="inp num" name="sell" inputmode="decimal" value="{}"></label></div></div><div class="field"><label>Reorder level (pkt) <input class="inp num" name="reorder" inputmode="numeric" value="{reorder}"></label></div><div class="row" style="justify-content:flex-end">{delete}<button class="btn" type="button" data-action="product_close">Cancel</button><button class="btn primary" type="submit">Save</button></div></form></div>"#,
        esc(&form.item),
        esc(&form.brand),
        pricing.buy().to_rupee_text(),
        pricing.margin().to_rupee_text(),
        pricing.sell().to_rupee_text(),
    )
}
