//! Counter: ALM_Business_Manager.html L2303–2445 (VIEWS.counter, greetNow, initialsOf).
//! Counts only, never amounts; Owner-only in core (`ConfirmationFixture::counter`).
use std::fmt::Write as _;

use alm_core::confirmation::{ConfirmationError, ConfirmationFixture, CounterTag};
use alm_core::{BillDate, Role};

use super::denied;
use super::icons::{icon, nav};
use crate::gst_print::esc;

fn act(class: &str, ico: &str, name: &str, desc: &str, tab: &str) -> String {
    let desc = if desc.is_empty() {
        String::new()
    } else {
        format!(r#"<span class="ad">{desc}</span>"#)
    };
    format!(
        r#"<button class="act {class}" type="button" data-action="screen" data-tab="{tab}"><span class="ai">{ico}</span><span class="atx"><span class="an">{name}</span>{desc}</span></button>"#
    )
}

fn initials(name: &str) -> String {
    let words: Vec<&str> = name.split_whitespace().collect();
    let first = words.first().and_then(|w| w.chars().next()).unwrap_or('?');
    let last = if words.len() > 1 {
        words.last().and_then(|w| w.chars().next())
    } else {
        None
    };
    first
        .to_uppercase()
        .chain(last.into_iter().flat_map(char::to_uppercase))
        .collect()
}

fn plural(count: usize, one: &str, many: &str) -> String {
    format!("<b>{count}</b>{}", if count == 1 { one } else { many })
}

pub fn counter_screen(
    fixture: &ConfirmationFixture,
    role: Role,
    today: BillDate,
) -> Result<String, ConfirmationError> {
    let counter = match fixture.counter(role, today) {
        Ok(counter) => counter,
        Err(ConfirmationError::AccessDenied) => return Ok(denied("Counter")),
        Err(error) => return Err(error),
    };
    let recents = fixture.counter_recents(role, &counter)?;
    let billed = match counter.billed_today_count {
        0 => "No bills yet today".to_owned(),
        1 => "1 bill billed today".to_owned(),
        n => format!("{n} bills billed today"),
    };
    let mut html = format!(
        r#"<div id="counter"><div class="ctr-hi"><h3>Counter</h3><p>{billed} · {}</p></div><div class="ctr-acts">{}{}</div><div class="ctr-tiles">{}</div>"#,
        today.display(),
        act(
            "primary",
            &nav("bill"),
            "New Bill",
            "Start a fresh estimate for a customer",
            "bill"
        ),
        act(
            "",
            &icon("cash", 19),
            "Receive Payment",
            "Settle udhaar",
            "receipt"
        ),
        act("sm", &nav("sales"), "Sales", "", "sales")
            + &act("sm", &nav("customers"), "Customers", "", "customers"),
    );
    html += r#"<div class="ctr-sec">Needs attention</div><div class="ctr-flag">"#;
    let attention = counter.attention;
    let flag = |class: &str, ico: String, text: String| {
        format!(
            r#"<div class="flag {class}"><span class="fi">{ico}</span><span>{text}</span></div>"#
        )
    };
    if attention.low_stock > 0 {
        html += &flag(
            "bad",
            icon("alert", 15),
            plural(
                attention.low_stock,
                " item needs reordering",
                " items need reordering",
            ),
        );
    }
    if attention.open_purchase_orders > 0 {
        html += &flag(
            "warn",
            nav("purchases"),
            plural(
                attention.open_purchase_orders,
                " order awaiting goods",
                " orders awaiting goods",
            ),
        );
    }
    if attention.debtors > 0 {
        let _ = write!(
            html,
            r#"<form data-action="customers_filter" data-payload="terms"><input type="hidden" name="q" value=""><button class="flag warn" type="submit" name="mode" value="due"><span class="fi">{}</span><span>{}</span></button></form>"#,
            nav("customers"),
            plural(
                attention.debtors,
                " customer to collect from",
                " customers to collect from"
            )
        );
    }
    if attention.low_stock + attention.open_purchase_orders + attention.debtors == 0 {
        html += &flag(
            "ok",
            icon("check", 15),
            "Nothing needs attention right now".into(),
        );
    }
    html += r#"</div><div class="ctr-sec">Recent bills</div><div class="card" style="padding:0;overflow:hidden">"#;
    if recents.is_empty() {
        html += r#"<div class="empty" style="padding:34px 20px"><div class="big">🧾</div>No bills yet — tap New Bill to make the first one.</div>"#;
    }
    for row in recents {
        let name = row.customer.as_deref().unwrap_or("—");
        let when = row
            .date
            .map_or_else(String::new, alm_core::BillDate::display);
        let (tag, label) = match row.tag {
            CounterTag::Credit => ("credit", "Credit"),
            CounterTag::Cash => ("cash", "Cash"),
            CounterTag::Online => ("cash", "Online"),
        };
        let _ = write!(
            html,
            r#"<button class="rbill" type="button" data-action="screen" data-tab="sales"><span class="rav">{}</span><span style="flex:1;min-width:0"><div class="rnm">{}</div><div class="rsub">#{} · {when}</div></span><span class="tag {tag}">{label}</span></button>"#,
            esc(&initials(name)),
            esc(name),
            row.bill.invoice
        );
    }
    Ok(html + "</div></div>")
}
