//! GST invoice form: ALM_Business_Manager.html L5332–5768 (gstBillForm, gstRow, renderGstTot).
//! Line parsing, defaults, item fill and totals come from core (`gst_line_input`, `gst_bill_form`).
use std::fmt::Write as _;

use alm_core::Role;
use alm_core::confirmation::{ConfirmationFixture, GstBillError, GstBillFields};

use crate::gst_print::{esc, inr};

fn input(label: &str, name: &str, value: &str, extra: &str) -> String {
    format!(
        r#"<div><label>{label} <input class="inp" name="{name}" value="{}"{extra}></label></div>"#,
        esc(value)
    )
}

fn select(label: &str, name: &str, value: &str, options: &[&str]) -> String {
    let mut html = format!(r#"<div><label>{label} <select class="inp" name="{name}">"#);
    for option in options {
        let selected = if *option == value { " selected" } else { "" };
        let text = if option.is_empty() {
            "— Select —"
        } else {
            option
        };
        let _ = write!(
            html,
            r#"<option value="{option}"{selected}>{text}</option>"#
        );
    }
    html + "</select></label></div>"
}

pub fn gst_form(
    fixture: &ConfirmationFixture,
    role: Role,
    f: &GstBillFields,
) -> Result<String, GstBillError> {
    let mut html = String::from(
        r#"<div style="max-width:1050px;margin:0 auto"><div class="help" style="margin-bottom:10px">Fill this GST bill yourself. It is not linked to stock or customer credit — it only produces a GST tax invoice to print or save as PDF.</div><form data-action="gst_submit" data-payload="terms"><div class="card">"#,
    );
    let _ = write!(
        html,
        r#"<div class="autobox field"><label>Pick customer (optional — fills the fields below) <input class="inp" role="combobox" aria-autocomplete="list" aria-expanded="false" aria-controls="gst-customer-list" name="pick_q" data-search="gst_customer_search" placeholder="Search a saved customer to auto-fill…" autocomplete="off"></label><input type="hidden" name="pick" value=""><div class="autolist" id="gst-customer-list" role="listbox" aria-label="Customer suggestions"></div></div><div class="f2">{}{}</div><div class="field">{}</div><div class="f2">{}{}</div><div class="f2">{}{}</div><div class="f2">{}{}</div><div class="f2">{}<div><label>GST Invoice No <input class="inp" value="{}" readonly></label></div></div><div class="f2">{}{}</div><div class="field">{}</div><div class="f3">{}{}{}</div><div class="field">{}</div></div>"#,
        input("Customer Name *", "customer", &f.customer, ""),
        input(
            "GSTIN",
            "gstin",
            &f.gstin,
            r#" placeholder="optional, 15 characters""#
        ),
        input("Address", "cust_addr", &f.cust_addr, ""),
        input("Phone", "cust_phone", &f.cust_phone, ""),
        input("Email", "cust_email", &f.cust_email, ""),
        input("State", "cust_state", &f.cust_state, ""),
        input("State Code", "cust_state_code", &f.cust_state_code, ""),
        input("Place of Supply (State)", "pos_state", &f.pos_state, ""),
        input("Place of Supply Code", "pos_code", &f.pos_code, ""),
        input("Date", "date", &f.date, r#" type="date""#),
        f.number.0,
        select(
            "Payment Mode",
            "pay_mode",
            &f.pay_mode,
            &["Cash", "UPI", "Bank Transfer", "Credit"]
        ),
        select(
            "Reverse Charge",
            "reverse_charge",
            &f.reverse_charge,
            &["No", "Yes"]
        ),
        input(
            "E-Way Bill No.",
            "eway_number",
            &f.eway_number,
            r#" placeholder="12 digits (optional)""#
        ),
        select(
            "Transporter",
            "transporter",
            &f.transporter,
            &["", "Self Delivery", "Customer Pickup"]
        ),
        input("Vehicle No.", "vehicle_no", &f.vehicle_no, ""),
        input("LR / GR No.", "lr_no", &f.lr_no, ""),
        input(
            "Terms of Delivery",
            "terms_delivery",
            &f.terms_delivery,
            r#" placeholder="optional — e.g. Door delivery""#
        ),
    );
    const GRID: &str = "display:grid;grid-template-columns:2.1fr .9fr .7fr .7fr 1fr .8fr .7fr 1fr 26px;gap:8px;align-items:center";
    let _ = write!(
        html,
        r#"<div class="card"><div class="row" style="justify-content:space-between"><div class="sectit" style="margin:0">Items</div><span><button class="btn sm" type="submit" name="intent" value="add">＋ Add Line</button></span></div><div class="bill-items" style="margin-top:12px"><div style="{GRID};padding:0 2px 6px;font-size:11px;color:var(--mut2);text-transform:uppercase;letter-spacing:.4px;font-weight:700"><div>Item</div><div>HSN</div><div>Unit</div><div class="num">Qty</div><div class="num">Rate ₹ (incl.)</div><div class="num">GST %</div><div class="num">Pack</div><div class="num">Amount</div><div></div></div>"#
    );
    for (i, line) in f.lines.iter().enumerate() {
        let amount = fixture
            .gst_line_amount(role, line)
            .map_or_else(|_| "—".to_owned(), |amount| format!("₹{}", inr(amount)));
        let unit = select("", &format!("unit_{i}"), &line.unit, &["Case", "Piece"]);
        let _ = write!(
            html,
            r#"<div style="{GRID};margin-bottom:7px"><div class="autobox"><input class="inp" role="combobox" aria-autocomplete="list" aria-expanded="false" aria-controls="gst-item-list-{i}" id="gst-item-{i}" name="item_{i}" data-search="gst_item_search" autocomplete="off" placeholder="Type item…" aria-label="Item {n}" value="{}"><div class="autolist" id="gst-item-list-{i}" role="listbox" aria-label="Item {n} suggestions"></div></div><input class="inp" name="hsn_{i}" aria-label="HSN {n}" value="{}">{unit}<input class="inp num" name="qty_{i}" inputmode="numeric" aria-label="Quantity {n}" value="{}"><input class="inp num" name="rate_{i}" inputmode="decimal" aria-label="Rate {n}" value="{}"><input class="inp num" name="gst_{i}" placeholder="auto" aria-label="GST % {n}" value="{}"><input class="inp num" name="pack_{i}" inputmode="numeric" aria-label="Pack {n}" value="{}"><span class="num">{amount}</span><button class="x" type="submit" name="intent" value="remove:{i}" aria-label="Remove line {n}">×</button></div>"#,
            esc(&line.item),
            esc(&line.hsn),
            esc(&line.quantity),
            esc(&line.rate),
            esc(&line.gst),
            esc(&line.packet_qty),
            n = i + 1
        );
    }
    html += "</div></div>";
    let lines: Result<Vec<_>, _> = f
        .lines
        .iter()
        .map(|line| fixture.gst_line_input(role, line))
        .collect();
    let totals = lines.and_then(|lines| {
        fixture.gst_bill_form(role, &fixture.gst_bill_draft(role, f, &lines, 0)?)
    });
    html += r#"<div class="card"><div style="min-width:280px;margin-left:auto">"#;
    match totals {
        Ok(t) => {
            for (label, value) in [
                ("Taxable Value", t.taxable),
                ("CGST", t.cgst),
                ("SGST", t.sgst),
                ("Total", t.total),
            ] {
                let _ = write!(
                    html,
                    r#"<div class="row" style="justify-content:space-between"><span class="muted">{label}</span><b>₹{}</b></div>"#,
                    inr(value)
                );
            }
        }
        Err(error) => {
            let _ = write!(
                html,
                r#"<div class="help" role="alert">Totals unavailable: {}</div>"#,
                error.warning().unwrap_or("check the amounts")
            );
        }
    }
    Ok(
        html + r#"</div></div><div class="card"><div class="row" style="justify-content:flex-end"><button class="btn" type="button" data-action="gst_back">Cancel</button><button class="btn primary" type="submit" name="intent" value="save">Save GST Invoice</button></div></div></form></div>"#
    )
}
