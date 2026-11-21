use std::fmt::Write;

use alm_phase2_local::confirmation::{BillPreview, EstimateLineDisplay};
use alm_phase2_local::{BillDate, Money};

pub(super) fn render(
    preview: &BillPreview,
    labels: &[EstimateLineDisplay<'_>],
    today: BillDate,
) -> String {
    let mut svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 720 470" width="720" height="470" role="img" aria-labelledby="title">
<title id="title">Synthetic source-inspired vector, not a legacy PNG</title>
<style>
.paper{{fill:#fff;stroke:#334155;stroke-width:1.5}} .heading{{fill:#1d4ed8;font-weight:800}} .rule{{stroke:#2563eb;stroke-width:3}} .shade{{fill:#eef2fb}} .cancel{{fill:#b91c1c;stroke:#b91c1c}} .ink{{fill:#000}} .due{{fill:#15803d;font-weight:800}} text{{font-family:'Segoe UI',Arial,sans-serif;font-size:13px}} .bold{{font-weight:700}} .small{{font-size:11px}}
</style>
<rect class="paper" x="1" y="1" width="718" height="468" rx="6"/>
<g id="header"><text class="heading" x="30" y="48" style="font-size:26px">Synthetic Business</text>
<text class="heading" x="690" y="48" text-anchor="end" style="font-size:19px">ESTIMATE</text>
<rect class="cancel" x="588" y="57" width="102" height="24" rx="6" fill-opacity="0" stroke-width="2"/><text class="cancel" x="639" y="74" text-anchor="middle" stroke="none">CANCELLED</text>
<text class="ink" x="690" y="102" text-anchor="end">No: {}</text>
<text class="ink" x="690" y="120" text-anchor="end">Date: {}</text></g>
<line class="rule" x1="30" y1="135" x2="690" y2="135"/>
<text class="ink" id="bill-to" x="30" y="160">Bill To: {}</text>
<g id="items"><rect class="shade" x="30" y="176" width="660" height="26"/>
<text class="ink bold" x="36" y="194">#</text><text class="ink bold" x="60" y="194">Item</text><text class="ink bold" x="310" y="194">Pack</text><text class="ink bold" x="373" y="194">MRP</text><text class="ink bold" x="477" y="194" text-anchor="end">Rate</text><text class="ink bold" x="550" y="194" text-anchor="end">Pkt</text><text class="ink bold" x="680" y="194" text-anchor="end">Amount</text>
"##,
        preview.invoice,
        display_date(preview.date),
        escape(&preview.customer),
    );
    for (index, (line, label)) in preview.lines.iter().zip(labels).enumerate() {
        let number = index + 1;
        let y = 223 + i32::try_from(index).unwrap() * 27;
        writeln!(
            svg,
            "<g id=\"line-{number}\"><text x=\"36\" y=\"{y}\">{number}</text><text x=\"60\" y=\"{y}\">{}</text><text x=\"477\" y=\"{y}\" text-anchor=\"end\">{}</text><text x=\"550\" y=\"{y}\" text-anchor=\"end\">{}</text><text x=\"680\" y=\"{y}\" text-anchor=\"end\">{}</text></g>",
            escape(label.label),
            money(line.price),
            line.quantity,
            money(line.amount),
        )
        .unwrap();
    }
    write!(
        svg,
        r##"<rect class="shade" x="30" y="258" width="660" height="26"/>
<text class="ink bold" x="477" y="276" text-anchor="end">Items Total</text><text class="ink bold" x="550" y="276" text-anchor="end">{}</text><text class="ink bold" x="680" y="276" text-anchor="end">{}</text></g>
<g id="summary"><text x="420" y="310">Loading/Delivery</text><text x="680" y="310" text-anchor="end">{}</text>
<text class="bold" x="420" y="334">Bill Amount</text><text class="bold" x="680" y="334" text-anchor="end">{}</text>
<text x="420" y="358">Paid on this bill</text><text x="680" y="358" text-anchor="end">{}</text>
<rect class="shade" x="410" y="366" width="280" height="26"/><text class="bold" x="420" y="384">Total Due (as on {})</text><text class="due" x="680" y="384" text-anchor="end">{}</text></g>
<g id="footer"><text class="small" x="30" y="428">Go paperless, save trees &amp; reduce carbon emission</text><text class="small" x="30" y="445">E&amp; OE</text><text class="small" x="690" y="428" text-anchor="end">For Synthetic Business</text><text class="small" x="690" y="445" text-anchor="end">Proprietor</text></g>
</svg>"##,
        preview.items_quantity,
        money(preview.items_total),
        money(preview.loading),
        money(preview.total),
        money(preview.paid),
        display_date(today),
        money(preview.balance),
    )
    .unwrap();
    svg
}

fn display_date(date: BillDate) -> String {
    let iso = date.to_string();
    let mut parts = iso.split('-');
    let year = parts.next().unwrap();
    let month = parts.next().unwrap().parse::<usize>().unwrap();
    let day = parts.next().unwrap();
    let months = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    format!("{day}-{}-{year}", months[month - 1])
}

fn money(value: Money) -> String {
    let minor = value.minor_units();
    let magnitude = minor.unsigned_abs();
    let sign = if minor < 0 { "-" } else { "" };
    format!("₹{sign}{}.{:02}", magnitude / 100, magnitude % 100)
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
