//! Port of the frozen HTML's `gstInvoiceHTML` (L4989-5134) over `GstBillInvoice` data.
//! Markup and inline styles are copied; every number comes from `alm-core`.
use alm_core::confirmation::{GstBillInvoice, GstJurisdiction};

mod format;
mod parts;

pub(crate) use format::indian_digits;
use format::rupees_words;
pub use format::{esc, inr};
use parts::{header, hsn_row, row, tax_rows};

/// The seller block (`DB.meta`); synthetic until the business profile is ported.
pub struct Seller<'a> {
    pub name: &'a str,
    pub address: &'a str,
    pub fssai: &'a str,
    pub gstin: &'a str,
}

pub(super) const B: &str = "border:1px solid #c2c8d2";
pub(super) const TC: &str = "padding:3px 7px;border-bottom:1px solid #eaedf1";
const TH: &str = "padding:4px 7px;border-bottom:1.5px solid #c2c8d2";
pub(super) const EDGE: &str = "border-top:1.5px solid #c2c8d2;border-bottom:1.5px solid #c2c8d2";

fn below_table(
    invoice: &GstBillInvoice,
    seller: &Seller<'_>,
    interstate: bool,
) -> Result<String, String> {
    let totals = invoice.totals;
    let summary = invoice.hsn_summary().map_err(crate::refusal)?;
    let rows: String = summary
        .iter()
        .enumerate()
        .map(|(i, r)| hsn_row(i, r, interstate))
        .collect();
    let (head, total) = if interstate {
        (
            format!(
                "<tr style=\"background:#e8edf6\"><th style=\"{B};padding:5px\" rowspan=\"2\">HSN/SAC</th><th style=\"{B};padding:5px\" rowspan=\"2\">Taxable<br>Value</th><th style=\"{B};padding:5px\" colspan=\"2\">IGST</th><th style=\"{B};padding:5px\" rowspan=\"2\">Total<br>Tax Amount</th></tr><tr style=\"background:#e8edf6\"><th style=\"{B};padding:5px\">Rate</th><th style=\"{B};padding:5px\">Amount</th></tr>"
            ),
            format!(
                "<tr style=\"font-weight:800;background:#f0f2f7\"><td style=\"{B};padding:6px;text-align:right\">Total</td><td style=\"{B};padding:6px;text-align:right\">{}</td><td style=\"{B}\"></td><td style=\"{B};padding:6px;text-align:right\">{}</td><td style=\"{B};padding:6px;text-align:right\">{}</td></tr>",
                inr(totals.taxable),
                inr(totals.igst),
                inr(totals.tax)
            ),
        )
    } else {
        (
            format!(
                "<tr style=\"background:#e8edf6\"><th style=\"{B};padding:5px\" rowspan=\"2\">HSN/SAC</th><th style=\"{B};padding:5px\" rowspan=\"2\">Taxable<br>Value</th><th style=\"{B};padding:5px\" colspan=\"2\">CGST</th><th style=\"{B};padding:5px\" colspan=\"2\">SGST/UTGST</th><th style=\"{B};padding:5px\" rowspan=\"2\">Total<br>Tax Amount</th></tr><tr style=\"background:#e8edf6\"><th style=\"{B};padding:5px\">Rate</th><th style=\"{B};padding:5px\">Amount</th><th style=\"{B};padding:5px\">Rate</th><th style=\"{B};padding:5px\">Amount</th></tr>"
            ),
            format!(
                "<tr style=\"font-weight:800;background:#f0f2f7\"><td style=\"{B};padding:6px;text-align:right\">Total</td><td style=\"{B};padding:6px;text-align:right\">{}</td><td style=\"{B}\"></td><td style=\"{B};padding:6px;text-align:right\">{}</td><td style=\"{B}\"></td><td style=\"{B};padding:6px;text-align:right\">{}</td><td style=\"{B};padding:6px;text-align:right\">{}</td></tr>",
                inr(totals.taxable),
                inr(totals.cgst),
                inr(totals.sgst),
                inr(totals.tax)
            ),
        )
    };
    Ok(format!(
        "<div style=\"padding:6px;border-bottom:1px solid #c2c8d2\">Amount Chargeable (in words)<span style=\"float:right;font-style:italic;font-size:9px\">E. &amp; O.E</span><br><b>{}</b></div>\n   <table style=\"width:100%;border-collapse:collapse;font-size:10px\">\n     <thead>{head}</thead><tbody>{rows}\n       {total}\n     </tbody></table>\n   <div style=\"padding:5px 6px;border-bottom:1px solid #c2c8d2;font-size:10px\">Tax Amount (in words) : <b>{}</b></div>\n   <div style=\"display:flex;justify-content:space-between;padding:8px 6px\">\n     <div style=\"font-size:9px;max-width:55%;color:#444\">Declaration:<br>We declare that this invoice shows the actual price of the goods described and that all particulars are true and correct.</div>\n     <div style=\"text-align:right;font-size:11px\">for <b>{}</b><br><br><br><span style=\"font-size:10px\">Authorised Signatory</span></div>\n   </div>\n   <div style=\"text-align:center;font-size:9px;border-top:1px solid #c2c8d2;padding:3px;color:#555\">This is a Computer Generated Invoice</div>",
        rupees_words(totals.total),
        rupees_words(totals.tax),
        esc(seller.name),
    ))
}

/// The printable invoice pages: 20 rows per page, with the summary on its own page when the
/// last page holds more than 12 rows.
pub fn invoice_html(invoice: &GstBillInvoice, seller: &Seller<'_>) -> Result<String, String> {
    let interstate = invoice.jurisdiction() == GstJurisdiction::Inter;
    let lines = invoice.lines().map_err(crate::refusal)?;
    let rows: Vec<String> = lines
        .iter()
        .zip(&invoice.record.lines)
        .enumerate()
        .map(|(i, (line, saved))| row(i, &saved.item, line))
        .collect();
    let mut pages: Vec<(&[String], bool)> = rows.chunks(20).map(|chunk| (chunk, false)).collect();
    if pages.is_empty() {
        pages.push((&[], false));
    }
    if pages.last().is_some_and(|page| page.0.len() > 12) {
        pages.push((&[], true));
    } else if let Some(last) = pages.last_mut() {
        last.1 = true;
    }
    let cancelled = if invoice.record.cancelled {
        "<div style=\"text-align:center;color:#b91c1c;font-weight:800;font-size:14px;letter-spacing:3px;padding:5px;border-bottom:1px solid #c2c8d2;background:#fde8e8\">✖ CANCELLED ✖</div>"
    } else {
        ""
    };
    let head = header(invoice, seller);
    let below = below_table(invoice, seller, interstate)?;
    let tax = tax_rows(invoice, interstate);
    let count = pages.len();
    let col_head = format!(
        "<thead><tr style=\"background:#eef1f6\">\n       <th style=\"{TH}\">Sl<br>No.</th><th style=\"{TH};text-align:left\">Description of Goods</th><th style=\"{TH}\">HSN/SAC</th><th style=\"{TH}\">Quantity</th><th style=\"{TH}\">Selling Price<br><span style=\"font-size:8px;font-weight:400\">(Incl. of GST)</span></th><th style=\"{TH}\">Taxable<br>Rate</th><th style=\"{TH}\">per</th><th style=\"{TH}\">Taxable<br>Amount</th>\n     </tr></thead>"
    );
    Ok(pages
        .iter()
        .enumerate()
        .map(|(p, (page_rows, last))| {
            let numbered = if count > 1 {
                format!(" <span style=\"font-size:11px;font-weight:600\">(Page {} of {count})</span>", p + 1)
            } else {
                String::new()
            };
            let breaker = if p + 1 < count { "page-break-after:always" } else { "" };
            let tail = if *last {
                tax.clone()
            } else {
                format!("<tr><td style=\"border-top:1px solid #d5d9e0;padding:6px 7px;text-align:right;font-style:italic;font-weight:700;color:#444\" colspan=\"8\">Continued on page {} &rarr;</td></tr>", p + 2)
            };
            format!(
                "<div style=\"{B};font-family:'Segoe UI',Arial;font-size:11px;color:#000;background:#fff;max-width:820px;margin:0 auto 14px;{breaker}\">\n   <div style=\"text-align:center;font-weight:800;font-size:18px;letter-spacing:1px;padding:8px;border-bottom:1px solid #c2c8d2\">TAX INVOICE{numbered}</div>\n   {cancelled}\n   {head}\n   <table style=\"width:100%;border-collapse:collapse;font-size:11px;border-top:1px solid #c2c8d2\">{col_head}<tbody>{}\n       {tail}\n     </tbody></table>\n   {}\n  </div>",
                page_rows.join(""),
                if *last { below.as_str() } else { "" }
            )
        })
        .collect())
}
