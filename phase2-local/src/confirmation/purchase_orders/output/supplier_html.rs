use crate::confirmation::estimate::{display_date, escape_text};
use crate::confirmation::image_handoff::{format_amount, group_indian_digits};

use super::{PurchaseOrder, PurchaseOutputRequest, PurchaseProductDisplay};

pub(super) fn order_html(order: &PurchaseOrder, request: PurchaseOutputRequest<'_>) -> String {
    let products = request.products;
    let business = request.business;
    let mut rows = String::new();
    let mut total = 0;
    for (index, saved) in order.items.iter().enumerate() {
        let line = &saved.line;
        let product = line
            .product_id
            .and_then(|id| products.iter().find(|product| product.id == id))
            .or_else(|| products.iter().find(|product| product.item == line.item))
            .copied();
        let item = escape_text(&line.item);
        let pack = product
            .and_then(PurchaseProductDisplay::pack)
            .map_or_else(|| "—".into(), |pack| escape_text(&pack));
        let mrp = product
            .and_then(PurchaseProductDisplay::displayed_mrp)
            .map_or_else(
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
    let supplier = escape_text(&order.supplier_name);
    let number = request.selection.number();
    let date = display_date(order.date);
    let total = group_indian_digits(total);
    let note = if order.note.is_empty() {
        String::new()
    } else {
        format!(" Note: {}", escape_text(&order.note))
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
