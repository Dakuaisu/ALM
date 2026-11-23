use std::num::NonZeroU32;
use std::time::Duration;

use crate::confirmation::estimate::{display_date, escape_text};
use crate::confirmation::image_handoff::{format_amount, group_indian_digits};
use crate::confirmation::{PopupDocument, PrintIntent};
use crate::{BillDate, Money};

use super::PurchaseBusiness;

pub(crate) struct SupplierDocument<'a> {
    pub(crate) number: u64,
    pub(crate) supplier_name: &'a str,
    pub(crate) date: BillDate,
    pub(crate) note: &'a str,
    pub(crate) business: PurchaseBusiness<'a>,
    pub(crate) rows: Vec<SupplierRow<'a>>,
}

pub(crate) struct SupplierRow<'a> {
    pub(crate) item: &'a str,
    pub(crate) pack: Option<String>,
    pub(crate) mrp: Option<Money>,
    pub(crate) ordered: NonZeroU32,
}

pub(in crate::confirmation::purchase_orders) fn purchase_order_number(number: u64) -> String {
    format!("PO-{number:03}")
}

pub(crate) fn render_supplier_print(document: &SupplierDocument<'_>) -> PopupDocument {
    let number = purchase_order_number(document.number);
    let title = escape_text(&format!("Order-{number}-{}", document.supplier_name));
    let body = order_html(document);
    PopupDocument {
        target: "_p",
        intent: PrintIntent::AfterLoad(Duration::from_millis(350)),
        html: format!(
            r#"<html><head><title>{title}</title><style>@page{{margin:0}}body{{margin:0;padding:10mm}}</style></head><body style="margin:0">{body}</body></html>"#
        ),
    }
}

pub(super) fn order_html(document: &SupplierDocument<'_>) -> String {
    let business = document.business;
    let mut rows = String::new();
    let mut total = 0;
    for (index, line) in document.rows.iter().enumerate() {
        let item = escape_text(line.item);
        let pack = line.pack.as_deref().map_or_else(|| "—".into(), escape_text);
        let mrp = line.mrp.map_or_else(
            || "—".into(),
            |amount| format!("₹{}", format_amount(amount)),
        );
        let ordered = u64::from(line.ordered.get());
        total += ordered;
        rows.push_str(&format!(
            r#"<tr><td class="c">{}</td><td>{item}</td><td class="r">{pack}</td><td class="r">{mrp}</td><td class="r"><b>{}</b></td></tr>"#,
            index + 1,
            group_indian_digits(ordered),
        ));
    }
    let name = escape_text(business.name);
    let address = escape_text(business.address);
    let mobile = escape_text(business.mobile);
    let supplier = escape_text(document.supplier_name);
    let number = purchase_order_number(document.number);
    let date = display_date(document.date);
    let total = group_indian_digits(total);
    let note = if document.note.is_empty() {
        String::new()
    } else {
        format!(" Note: {}", escape_text(document.note))
    };
    format!(
        r#"<div style="background:#fff;color:#000;font-family:'Segoe UI',Arial,sans-serif;padding:26px 30px;width:100%;max-width:720px;margin:0 auto;box-sizing:border-box;border:1.5px solid #334155;border-radius:6px">
    <style>.pot{{border-collapse:collapse;width:100%;font-size:14px}}.pot td,.pot th{{border:1px solid #8b97a8;padding:7px 9px}}.pot th{{background:#eef2fb;text-align:left}}.pot .r{{text-align:right}}.pot .c{{text-align:center}}</style>
    <div style="display:flex;justify-content:space-between;border-bottom:3px solid #2563eb;padding-bottom:12px">
      <div><div style="font-size:24px;font-weight:800;color:#1d4ed8">{name}</div>
      <div style="font-size:12px;margin-top:4px;max-width:340px;overflow-wrap:break-word">{address}</div>
      <div style="font-size:12px;font-weight:700">Mob: {mobile}</div></div>
      <div style="text-align:right"><div style="font-size:18px;font-weight:700;letter-spacing:1px;color:#1d4ed8">PURCHASE ORDER</div>
      <div style="font-size:13px;margin-top:8px"><b>{number}</b></div>
      <div style="font-size:13px"><b>Date:</b> {date}</div></div>
    </div>
    <div style="margin:13px 0;font-size:14px">To: <b>{supplier}</b></div>
    <table class="pot"><thead><tr><th class="c">#</th><th>Item</th><th class="r">Pack</th><th class="r">MRP</th><th class="r">Qty (cases)</th></tr></thead>
    <tbody>{rows}</tbody>
    <tfoot><tr style="background:#eef2fb;font-weight:800"><td colspan="4" style="text-align:right">Total</td><td class="r">{total} cases</td></tr></tfoot></table>
    <div style="margin-top:10px;font-size:12px;color:#333">Kindly supply the above items at the earliest.{note}</div>
    <div style="display:flex;justify-content:flex-end;margin-top:26px;font-size:11px;color:#000">
      <div style="text-align:right">For <b>{name}</b><br><br>______________________<br>Authorised Signatory</div>
    </div>
  </div>"#
    )
}
