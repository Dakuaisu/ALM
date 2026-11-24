//! Crates: ALM_Business_Manager.html L7953–8044 (VIEWS.crates) and the template builder
//! (openCrateBuilder, template mode, inline). Contents text and today's value come from
//! `crate_template_list`; components from `crate_component_from_text`.
use std::fmt::Write as _;

use alm_core::Role;
use alm_core::confirmation::owner_billing::BillingDirectory;
use alm_core::confirmation::{ConfirmationFixture, CrateTemplateError};

use super::{denied, money_r};
use crate::gst_print::esc;

#[derive(Clone, Debug, Default)]
pub struct CrateForm {
    pub id: Option<u64>,
    pub name: String,
    pub price: String,
    /// (product ID, quantity, unit) as typed.
    pub rows: Vec<(String, String, String)>,
}

pub fn crates_screen(
    fixture: &ConfirmationFixture,
    directory: &BillingDirectory,
    role: Role,
    form: Option<&CrateForm>,
    today: alm_core::BillDate,
) -> Result<String, CrateTemplateError> {
    let rows = match fixture.crate_template_list(role) {
        Ok(rows) => rows,
        Err(CrateTemplateError::AccessDenied) => return Ok(denied("Crates")),
        Err(error) => return Err(error),
    };
    let mut html = String::from(
        r#"<div class="help" style="margin-bottom:12px">Build once, bill fast: a crate sells as ONE line at its price; stock still reduces item-by-item.</div>"#,
    );
    if let Some(form) = form {
        html += &builder(fixture, role, form);
    }
    html += r#"<div class="tbl-wrap" style="max-height:60vh"><table><thead><tr><th>Name</th><th>Contents</th><th class="num">Items</th><th class="num">Price ₹</th><th class="num">Contents value</th><th class="num">Actions</th></tr></thead><tbody>"#;
    if rows.is_empty() {
        html += r#"<tr><td colspan="6"><div class="empty">📦<br>No crate templates yet</div></td></tr>"#;
    }
    for row in rows {
        let id = row.id.value();
        let _ = write!(
            html,
            r#"<tr><td><b>{}</b></td><td class="muted">{}</td><td class="num">{}</td><td class="num">{}</td><td class="num" title="Sum of contents at today's rates">{}</td><td class="num"><button class="btn sm" data-action="crate_edit" data-id="{id}">✎ Edit</button> {sell} <details style="display:inline-block"><summary class="btn sm danger">✕</summary><button class="btn sm danger" data-action="crate_delete" data-id="{id}">Delete template</button></details></td></tr>"#,
            esc(&row.template.name),
            esc(&row.contents),
            row.template.components.len(),
            money_r(row.template.price),
            row.value.map_or_else(|_| "—".to_owned(), money_r),
            sell = sell_form(directory, id, today)
        );
    }
    Ok(html + "</tbody></table></div>")
}

/// Sells crates of one template to a directory customer (the HTML adds the crate as a New Bill
/// line; New Bill has no crate rows here).
fn sell_form(directory: &BillingDirectory, id: u64, today: alm_core::BillDate) -> String {
    let mut options = String::new();
    for c in &directory.customers {
        let _ = write!(
            options,
            r#"<option value="{}">{}</option>"#,
            esc(c.id),
            esc(c.name)
        );
    }
    format!(
        r#"<details style="display:inline-block"><summary class="btn sm">Sell</summary><form class="row" style="gap:6px;margin-top:6px" data-action="crate_sell" data-payload="terms"><input type="hidden" name="id" value="{id}"><select class="inp" name="customer" aria-label="Customer"><option value="">Choose customer…</option>{options}</select><input class="inp num" name="qty" inputmode="numeric" value="1" aria-label="Crates" style="width:70px"><input class="inp" type="date" name="date" value="{today}" aria-label="Bill date"><input class="inp num" name="paid" inputmode="decimal" placeholder="Paid ₹" aria-label="Amount paid" style="width:90px"><button class="btn sm primary" type="submit">Sell crate</button></form></details>"#
    )
}

/// The component picker lists every product in the catalog, as the HTML's autocomplete over
/// `DB.products` does (a product added on Products can be a component at once).
fn builder(fixture: &ConfirmationFixture, role: Role, form: &CrateForm) -> String {
    let products: Vec<(String, String)> = fixture
        .product_list(role, "", None)
        .map(|list| {
            list.rows
                .into_iter()
                .map(|row| (row.product.value().to_string(), row.item))
                .collect()
        })
        .unwrap_or_default();
    let mut html = format!(
        r#"<div class="card" style="margin-bottom:14px"><h3>📦 Crate template</h3><form data-action="crate_submit" data-payload="terms"><div class="f2"><div><label>Crate name <input class="inp" name="name" value="{}"></label></div><div><label>Crate price ₹ <input class="inp num" name="price" inputmode="decimal" value="{}"></label></div></div>"#,
        esc(&form.name),
        esc(&form.price)
    );
    for (i, (product, qty, unit)) in form.rows.iter().enumerate() {
        let _ = write!(
            html,
            r#"<div class="row" style="gap:8px;margin-top:6px"><select class="inp" name="product_{i}" aria-label="Component {n}"><option value="">Choose item…</option>"#,
            n = i + 1
        );
        for (id, name) in &products {
            let selected = if id == product { " selected" } else { "" };
            let _ = write!(
                html,
                r#"<option value="{id}"{selected}>{}</option>"#,
                esc(name)
            );
        }
        let _ = write!(
            html,
            r#"</select><input class="inp num" name="qty_{i}" inputmode="numeric" aria-label="Component quantity {n}" value="{}" style="width:80px"><select class="inp" name="unit_{i}" aria-label="Component unit {n}">"#,
            esc(qty),
            n = i + 1
        );
        for option in ["Case", "Piece"] {
            let selected = if option == unit { " selected" } else { "" };
            let _ = write!(html, r#"<option{selected}>{option}</option>"#);
        }
        html += "</select></div>";
    }
    html + r#"<div class="row" style="justify-content:flex-end;margin-top:10px"><button class="btn" type="submit" name="intent" value="add">＋ Component</button><button class="btn" type="button" data-action="crate_close">Cancel</button><button class="btn primary" type="submit" name="intent" value="save">Save crate</button></div></form></div>"#
}
