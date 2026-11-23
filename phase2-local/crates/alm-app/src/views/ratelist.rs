//! Rate List: ALM_Business_Manager.html L11295–11420 (VIEWS.ratelist, waRateList). Rows, order,
//! ticks, group toggles and the WhatsApp text come from `confirmation::rate_list`.
use std::collections::BTreeSet;
use std::fmt::Write as _;

use alm_core::confirmation::rate_list::RateListSelection;
use alm_core::confirmation::{ConfirmationFixture, ProductEntryError};
use alm_core::{BillDate, Role};

use super::{denied, money_r};
use crate::gst_print::esc;

pub fn ratelist_screen(
    fixture: &ConfirmationFixture,
    role: Role,
    selection: &mut RateListSelection,
    business: &str,
    today: BillDate,
) -> Result<String, ProductEntryError> {
    let rows = match fixture.rate_list_rows(role, selection) {
        Ok(rows) => rows,
        Err(ProductEntryError::AccessDenied) => return Ok(denied("Rate List")),
        Err(error) => return Err(error),
    };
    let mut html = String::from(
        r#"<div class="card" style="margin-bottom:14px"><div class="sectit">Brands</div><div class="row" style="gap:6px;flex-wrap:wrap">"#,
    );
    let mut brands = BTreeSet::new();
    let mut probe = RateListSelection::default();
    for row in fixture.rate_list_rows(role, &mut probe)? {
        brands.insert(row.brand);
    }
    for brand in &brands {
        let on = if selection.brands.contains(brand) {
            "pill on"
        } else {
            "pill"
        };
        let _ = write!(
            html,
            r#"<button class="{on}" data-action="rate_brand" data-q="{0}" aria-label="Filter {0}">{0}</button> <button class="btn sm" data-action="rate_group" data-q="{0}" aria-label="Tick all {0}">✓ all</button>"#,
            esc(brand)
        );
    }
    let _ = write!(
        html,
        r#"</div></div><form class="row" style="margin-bottom:12px" data-action="rate_search" data-payload="terms"><div class="search" style="flex:1"><span class="si">🔍</span><input class="inp" name="q" placeholder="Search item or brand…" aria-label="Search rate list" value="{}"></div><button class="btn" type="submit">Search</button></form><div class="tbl-wrap" style="max-height:48vh"><table><thead><tr><th></th><th>Brand</th><th>Item</th><th class="num">Our Rate</th></tr></thead><tbody>"#,
        esc(&selection.query)
    );
    for row in &rows {
        let tick = if row.selected { "☑" } else { "☐" };
        let _ = write!(
            html,
            r#"<tr><td><button class="btn sm" data-action="rate_toggle" data-id="{}" aria-label="Tick {}">{tick}</button></td><td class="muted">{}</td><td><b>{}</b></td><td class="num">{}</td></tr>"#,
            row.product,
            esc(&row.item),
            esc(&row.brand),
            esc(&row.item),
            money_r(row.rate)
        );
    }
    html += "</tbody></table></div>";
    match fixture.rate_list_text(role, selection, business, today)? {
        Some((text, uri)) => {
            let _ = write!(
                html,
                r#"<div class="card" style="margin-top:14px"><div class="sectit">💬 WhatsApp text</div><textarea class="inp" readonly rows="8" style="width:100%">{}</textarea><div class="help">Link: {}</div></div>"#,
                esc(&text),
                esc(&uri)
            );
        }
        None => {
            html += r#"<div class="help" style="margin-top:12px">Tick at least one item.</div>"#
        }
    }
    Ok(html)
}
