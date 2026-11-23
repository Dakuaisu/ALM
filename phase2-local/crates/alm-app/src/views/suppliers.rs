//! Suppliers: ALM_Business_Manager.html L10362–10440 (VIEWS.suppliers) and L7835–7950
//! (priceCompareModal, shown inline). Counts and amounts come from `supplier_book`; quotes, best
//! supplier and highlighting from `compare_supplier_prices`.
use std::fmt::Write as _;

use alm_core::Role;
use alm_core::confirmation::{ConfirmationFixture, PurchaseError};

use super::{denied, money_r};
use crate::gst_print::esc;

pub fn suppliers_screen(fixture: &ConfirmationFixture, role: Role) -> Result<String, String> {
    let book = match fixture.supplier_book(role) {
        Ok(book) => book,
        Err(PurchaseError::AccessDenied) => return Ok(denied("Suppliers")),
        Err(error) => return Err(error.to_string()),
    };
    let mut html = String::from(
        r#"<div class="tbl-wrap" style="max-height:62vh"><table><thead><tr><th>Supplier</th><th class="num">Purchases</th><th class="num">Total Bought</th></tr></thead><tbody>"#,
    );
    if book.is_empty() {
        html += r#"<tr><td colspan="3"><div class="empty">🏭<br>No suppliers yet</div></td></tr>"#;
    }
    for row in &book {
        let _ = write!(
            html,
            r#"<tr><td><b>{}</b></td><td class="num">{}</td><td class="num">{}</td></tr>"#,
            esc(&row.name),
            row.purchases,
            money_r(row.bought)
        );
    }
    html += r#"</tbody></table></div><form class="row" style="margin:12px 0" data-action="supplier_add" data-payload="terms"><input class="inp" name="name" placeholder="Supplier name" aria-label="New supplier name" required><button class="btn primary" type="submit">＋ Add Supplier</button></form>"#;
    let comparison = fixture
        .compare_supplier_prices(role, None)
        .map_err(|error| format!("{error:?}"))?;
    html += r#"<div class="card"><div class="sectit">⚖️ Compare supplier prices</div><div class="help">Buy rate per packet from each supplier. Blank or 0 clears a quote; the cheapest is highlighted.</div><div class="tbl-wrap"><table><thead><tr><th>Product</th><th class="num">Our Buy</th>"#;
    for supplier in &comparison.suppliers {
        let _ = write!(html, r#"<th class="num">{}</th>"#, esc(&supplier.name));
    }
    html += r#"<th>Best</th><th></th></tr></thead><tbody>"#;
    for row in &comparison.rows {
        let form = format!("quote-{}", row.product);
        let _ = write!(
            html,
            r#"<tr><td><b>{}</b></td><td class="num">{}</td>"#,
            esc(&row.item),
            money_r(row.our_buy)
        );
        for (column, (supplier, price)) in comparison.suppliers.iter().zip(&row.prices).enumerate()
        {
            let style = if row.is_highlighted(column) {
                r#" style="color:var(--good);font-weight:700""#
            } else {
                ""
            };
            let _ = write!(
                html,
                r#"<td class="num"><input class="inp num" form="{form}" name="s{}" inputmode="decimal" aria-label="{} quote for {}" value="{}"{style}></td>"#,
                supplier.id.index(),
                esc(&supplier.name),
                esc(&row.item),
                price.map_or_else(String::new, |p| p.to_rupee_text())
            );
        }
        let best = row.best.and_then(|(id, rate)| {
            let name = &comparison.suppliers.iter().find(|s| s.id == id)?.name;
            Some(format!(
                "<b class=\"pos\">{}</b> @ {}",
                esc(name),
                money_r(rate)
            ))
        });
        let _ = write!(
            html,
            r#"<td>{}</td><td><form id="{form}" data-action="supplier_quotes" data-payload="terms"><input type="hidden" name="product" value="{}"><button class="btn sm" type="submit">Save</button></form></td></tr>"#,
            best.unwrap_or_else(|| "—".into()),
            row.product
        );
    }
    Ok(html + "</tbody></table></div></div>")
}
