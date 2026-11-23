//! Godown: ALM_Business_Manager.html L10927–11110 (VIEWS.godown manual list). The list, filter
//! and case total come from `manual_godown_list`; it never reads or changes billing stock.
use std::fmt::Write as _;

use alm_core::Role;
use alm_core::confirmation::{ConfirmationFixture, ManualGodownError};

use super::denied;
use crate::gst_print::{esc, indian_digits};

pub fn godown_screen(
    fixture: &ConfirmationFixture,
    role: Role,
    query: &str,
) -> Result<String, ManualGodownError> {
    let list = match fixture.manual_godown_list(role, query) {
        Ok(list) => list,
        Err(ManualGodownError::AccessDenied) => return Ok(denied("Godown")),
        Err(error) => return Err(error),
    };
    let mut html = String::from(
        r#"<div class="card" style="margin-bottom:14px"><div class="sectit">Add / update item</div><div class="help" style="margin-bottom:10px">Manual list — just so you always know what's lying in the godown. It is NOT connected to billing stock; update it whenever goods move in or out.</div><form data-action="godown_save" data-payload="terms" style="display:grid;grid-template-columns:2fr .8fr 1.4fr auto;gap:8px;align-items:end"><div class="autobox"><label>Item <input class="inp" name="item" data-search="godown_item_search" autocomplete="off" placeholder="Item…" required></label><div class="autolist"></div></div><div><label>Qty (cases) <input class="inp num" name="cases" placeholder="cases" inputmode="numeric" required></label></div><div><label>Note <input class="inp" name="note" placeholder="note (optional, e.g. back rack)"></label></div><button class="btn primary" type="submit">＋ Save</button></form></div>"#,
    );
    let _ = write!(
        html,
        r#"<form class="row" style="margin-bottom:12px" data-action="godown_filter" data-payload="terms"><div class="search" style="flex:1"><span class="si">🔍</span><input class="inp" name="q" placeholder="Search godown…" aria-label="Search godown" value="{}"></div><button class="btn" type="submit">Search</button></form><div class="card"><div class="row" style="justify-content:space-between"><div class="sectit" style="margin:0">In the godown</div><span class="muted">{} items · {} cases</span></div><div class="tbl-wrap"><table><thead><tr><th>Item</th><th class="num">Qty (cases)</th><th>Note</th></tr></thead><tbody>"#,
        esc(query),
        list.rows.len(),
        indian_digits(list.total_cases)
    );
    if list.rows.is_empty() {
        html += r#"<tr><td colspan="3"><div class="empty">🏪<br>Nothing in the godown list yet</div></td></tr>"#;
    }
    for row in &list.rows {
        let _ = write!(
            html,
            r#"<tr><td><b>{}</b></td><td class="num" style="font-weight:800">{}</td><td class="muted">{}</td></tr>"#,
            esc(&row.item),
            indian_digits(u64::from(row.cases.get())),
            esc(row.note.as_deref().unwrap_or(""))
        );
    }
    Ok(html + "</tbody></table></div></div>")
}
