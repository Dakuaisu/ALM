use alm_store::confirmation::{GstBillInvoice, GstJurisdiction};

const BORDER: &str = "border:1px solid #c2c8d2";
const CELL: &str = "padding:3px 7px;border-bottom:1px solid #eaedf1";
const HEADING: &str = "padding:4px 7px;border-bottom:1.5px solid #c2c8d2";

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn inr(minor: i64) -> String {
    let digits = (minor / 100).to_string();
    let mut groups = Vec::new();
    let mut end = digits.len();
    let mut width = 3;
    while end > 0 {
        let start = end.saturating_sub(width);
        groups.push(&digits[start..end]);
        end = start;
        width = 2;
    }
    groups.reverse();
    format!("{}.{:02}", groups.join(","), minor % 100)
}

pub fn title(invoice: &GstBillInvoice) -> String {
    let date = invoice.record.date.to_string();
    let year: u16 = date[..4].parse().unwrap();
    let month: u8 = date[5..7].parse().unwrap();
    let start = if month >= 4 { year } else { year - 1 };
    format!(
        "GST-Invoice-{:02}-{:02}-{}",
        start % 100,
        (start + 1) % 100,
        invoice.record.number.0
    )
}

fn row(index: usize, line: &alm_store::confirmation::GstBillLine) -> String {
    let quantity = line.quantity.get();
    let inclusive = inr(line.inclusive_rate.minor_units());
    let taxable_rate = inr(line.taxable_rate().unwrap().minor_units());
    let taxable_amount = inr(line.taxable_amount().unwrap().minor_units());
    let bg = if index % 2 == 1 {
        "background:#f7f9fc"
    } else {
        ""
    };
    format!(
        "<tr style=\"{bg}\"><td style=\"{CELL};text-align:center\">{}</td><td style=\"{CELL}\">{}</td><td style=\"{CELL};text-align:center\">22029090</td><td style=\"{CELL};text-align:right;white-space:nowrap\">{quantity} CASE <span style=\"font-size:9px;color:#666\">({quantity} PCS)</span></td><td style=\"{CELL};text-align:right\">{inclusive}</td><td style=\"{CELL};text-align:right\">{taxable_rate}</td><td style=\"{CELL};text-align:center\">CASE</td><td style=\"{CELL};text-align:right\">{taxable_amount}</td></tr>",
        index + 1,
        escape(&line.item)
    )
}

fn summary(invoice: &GstBillInvoice) -> String {
    let totals = invoice.totals;
    let tax_row = |label, value: alm_store::Money| {
        format!(
            "<tr><td style=\"{CELL};text-align:right\" colspan=\"7\">{label}</td><td style=\"{CELL};text-align:right\">{}</td></tr>",
            inr(value.minor_units())
        )
    };
    let tax_rows = match invoice.record.jurisdiction {
        GstJurisdiction::Intra => format!(
            "{}\n       {}",
            tax_row("CGST", totals.cgst),
            tax_row("SGST", totals.sgst)
        ),
        GstJurisdiction::Inter => tax_row("IGST", totals.igst),
    };
    let grand = inr(totals.total.minor_units());
    let quantity: u64 = invoice
        .record
        .lines
        .iter()
        .map(|line| u64::from(line.quantity.get()))
        .sum();
    format!(
        "{tax_rows}\n       <tr style=\"font-weight:800;background:#eef1f6\"><td style=\"padding:6px 7px;text-align:right;border-top:1.5px solid #c2c8d2;border-bottom:1.5px solid #c2c8d2\" colspan=\"3\">Grand Total</td><td style=\"padding:6px 7px;text-align:right;border-top:1.5px solid #c2c8d2;border-bottom:1.5px solid #c2c8d2\">{quantity} CASE</td><td style=\"border-top:1.5px solid #c2c8d2;border-bottom:1.5px solid #c2c8d2\" colspan=\"3\"></td><td style=\"padding:6px 7px;text-align:right;font-size:12px;border-top:1.5px solid #c2c8d2;border-bottom:1.5px solid #c2c8d2\">&#8377; {grand}</td></tr>"
    )
}

pub fn render(invoice: &GstBillInvoice) -> String {
    let rows: Vec<_> = invoice
        .record
        .lines
        .iter()
        .enumerate()
        .map(|(index, line)| row(index, line))
        .collect();
    let mut chunks: Vec<_> = rows.chunks(20).collect();
    if chunks.is_empty() {
        chunks.push(&[]);
    }
    if chunks.last().unwrap().len() > 12 {
        chunks.push(&[]);
    }
    let page_count = chunks.len();
    let mut html = String::new();
    for (index, chunk) in chunks.iter().enumerate() {
        let last = index + 1 == page_count;
        let page_break = if last { "" } else { "page-break-after:always" };
        let page_number = if page_count > 1 {
            format!(
                " <span style=\"font-size:11px;font-weight:600\">(Page {} of {page_count})</span>",
                index + 1
            )
        } else {
            String::new()
        };
        html.push_str(&format!("<div style=\"{BORDER};font-family:'Segoe UI',Arial;font-size:11px;color:#000;background:#fff;max-width:820px;margin:0 auto 14px;{page_break}\">\n   <div style=\"text-align:center;font-weight:800;font-size:18px;letter-spacing:1px;padding:8px;border-bottom:1px solid #c2c8d2\">TAX INVOICE{page_number}</div>\n   <!-- Seller, buyer, and transport metadata omitted from this pagination replay. -->\n   <div style=\"display:flex\"><div style=\"width:56%;border-right:1px solid #c2c8d2\"><div style=\"padding:6px\"><span style=\"font-size:8.5px;color:#555\">Buyer (Bill to)</span><br><b>{}</b></div></div></div>\n   <table style=\"width:100%;border-collapse:collapse;font-size:11px;border-top:1px solid #c2c8d2\"><thead><tr style=\"background:#eef1f6\"><th style=\"{HEADING}\">Sl<br>No.</th><th style=\"{HEADING};text-align:left\">Description of Goods</th><th style=\"{HEADING}\">HSN/SAC</th><th style=\"{HEADING}\">Quantity</th><th style=\"{HEADING}\">Selling Price<br><span style=\"font-size:8px;font-weight:400\">(Incl. of GST)</span></th><th style=\"{HEADING}\">Taxable<br>Rate</th><th style=\"{HEADING}\">per</th><th style=\"{HEADING}\">Taxable<br>Amount</th></tr></thead><tbody>{}", escape(&invoice.record.customer), chunk.join("")));
        if last {
            html.push_str(&summary(invoice));
        } else {
            html.push_str(&format!("<tr><td style=\"border-top:1px solid #d5d9e0;padding:6px 7px;text-align:right;font-style:italic;font-weight:700;color:#444\" colspan=\"8\">Continued on page {} &rarr;</td></tr>", index + 2));
        }
        html.push_str("</tbody></table>");
        if last {
            let taxable = inr(invoice.totals.taxable.minor_units());
            let tax = inr(invoice.totals.tax.minor_units());
            html.push_str(&format!("<table><thead><tr><th>Taxable Value</th><th>Total Tax Amount</th></tr></thead><tbody><tr><td>{taxable}</td><td>{tax}</td></tr></tbody></table>"));
            html.push_str("<div style=\"padding:6px;border-bottom:1px solid #c2c8d2\">Amount Chargeable (in words)<span style=\"float:right;font-style:italic;font-size:9px\">E. &amp; O.E</span><br><b><!-- Amount in words omitted. --></b></div><!-- HSN breakdown and tax-in-words omitted. --><div style=\"display:flex;justify-content:space-between;padding:8px 6px\"><div style=\"font-size:9px;max-width:55%;color:#444\">Declaration:<br>We declare that this invoice shows the actual price of the goods described and that all particulars are true and correct.</div><div style=\"text-align:right;font-size:11px\">for <b>Synthetic Business</b><br><br><br><span style=\"font-size:10px\">Authorised Signatory</span></div></div><div style=\"text-align:center;font-size:9px;border-top:1px solid #c2c8d2;padding:3px;color:#555\">This is a Computer Generated Invoice</div>");
        }
        html.push_str("</div>");
    }
    html
}
