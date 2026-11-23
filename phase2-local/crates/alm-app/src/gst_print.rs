//! Port of the frozen HTML's `gstInvoiceHTML` (L4989-5134) over `GstBillInvoice` data.
//! Markup and inline styles are copied; every number comes from `alm-core`.
use alm_core::Money;
use alm_core::confirmation::{
    GstBillInvoice, GstHsnSummary, GstInvoiceLine, GstJurisdiction, GstUnit,
};

/// The seller block (`DB.meta`); synthetic until the business profile is ported.
pub struct Seller<'a> {
    pub name: &'a str,
    pub address: &'a str,
    pub fssai: &'a str,
    pub gstin: &'a str,
}

const B: &str = "border:1px solid #c2c8d2";
const TC: &str = "padding:3px 7px;border-bottom:1px solid #eaedf1";
const TH: &str = "padding:4px 7px;border-bottom:1.5px solid #c2c8d2";
const EDGE: &str = "border-top:1.5px solid #c2c8d2;border-bottom:1.5px solid #c2c8d2";

/// `esc`: the HTML escapes `& < > "` only.
pub fn esc(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub(crate) fn indian_digits(mut whole: u64) -> String {
    let mut groups = vec![format!("{:03}", whole % 1000)];
    whole /= 1000;
    while whole > 0 {
        groups.push(format!("{:02}", whole % 100));
        whole /= 100;
    }
    let mut text = groups.into_iter().rev().collect::<Vec<_>>().join(",");
    while text.len() > 1 && (text.starts_with('0') || text.starts_with(',')) {
        text.remove(0);
    }
    text
}

/// `inr`: en-IN grouping with exactly two decimals.
pub fn inr(money: Money) -> String {
    let minor = money.minor_units();
    let sign = if minor < 0 { "-" } else { "" };
    let magnitude = minor.unsigned_abs();
    format!(
        "{sign}{}.{:02}",
        indian_digits(magnitude / 100),
        magnitude % 100
    )
}

/// `fmtN` for whole numbers.
fn fmt_n(value: u64) -> String {
    indian_digits(value)
}

/// A rate in basis points as the HTML prints a JS number: `20`, `2.5`, `9`.
fn percent(basis_points: u32) -> String {
    let text = format!("{}.{:02}", basis_points / 100, basis_points % 100);
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}

/// `numToWords` (L4843-4862).
fn words(mut n: u64) -> String {
    const A: [&str; 20] = [
        "",
        "One",
        "Two",
        "Three",
        "Four",
        "Five",
        "Six",
        "Seven",
        "Eight",
        "Nine",
        "Ten",
        "Eleven",
        "Twelve",
        "Thirteen",
        "Fourteen",
        "Fifteen",
        "Sixteen",
        "Seventeen",
        "Eighteen",
        "Nineteen",
    ];
    const T: [&str; 10] = [
        "", "", "Twenty", "Thirty", "Forty", "Fifty", "Sixty", "Seventy", "Eighty", "Ninety",
    ];
    let two = |x: u64| -> String {
        if x < 20 {
            A[x as usize].to_owned()
        } else if x.is_multiple_of(10) {
            T[(x / 10) as usize].to_owned()
        } else {
            format!("{} {}", T[(x / 10) as usize], A[(x % 10) as usize])
        }
    };
    let three = |x: u64| -> String {
        let hundreds = if x >= 100 {
            format!(
                "{} Hundred{}",
                A[(x / 100) as usize],
                if !x.is_multiple_of(100) { " " } else { "" }
            )
        } else {
            String::new()
        };
        let rest = if !x.is_multiple_of(100) {
            two(x % 100)
        } else {
            String::new()
        };
        hundreds + &rest
    };
    if n == 0 {
        return "Zero".into();
    }
    let mut text = String::new();
    let crore = n / 10_000_000;
    n %= 10_000_000;
    let lakh = n / 100_000;
    n %= 100_000;
    let thousand = n / 1000;
    n %= 1000;
    if crore > 0 {
        text += &format!("{} Crore ", three(crore));
    }
    if lakh > 0 {
        text += &format!("{} Lakh ", two(lakh));
    }
    if thousand > 0 {
        text += &format!("{} Thousand ", two(thousand));
    }
    if n > 0 {
        text += &three(n);
    }
    text.trim().to_owned()
}

/// `rupeesWords` (L4864-4870).
fn rupees_words(money: Money) -> String {
    let minor = money.minor_units().unsigned_abs();
    let mut text = format!("INR {}", words(minor / 100));
    if !minor.is_multiple_of(100) {
        text += &format!(" and {} paise", words(minor % 100));
    }
    text + " Only"
}

fn row(index: usize, name: &str, line: &GstInvoiceLine) -> String {
    let (qcell, per) = match (line.unit, line.pieces) {
        (GstUnit::Piece, _) | (GstUnit::Case, None) => {
            (format!("{} PCS", fmt_n(line.quantity.into())), "PCS")
        }
        (GstUnit::Case, Some(pieces)) => (
            format!(
                "{} CASE <span style=\"font-size:9px;color:#666\">({} PCS)</span>",
                fmt_n(line.quantity.into()),
                fmt_n(pieces)
            ),
            "CASE",
        ),
    };
    let bg = if index % 2 == 1 {
        "background:#f7f9fc"
    } else {
        ""
    };
    format!(
        "<tr style=\"{bg}\"><td style=\"{TC};text-align:center\">{}</td><td style=\"{TC}\">{}</td><td style=\"{TC};text-align:center\">{}</td><td style=\"{TC};text-align:right;white-space:nowrap\">{qcell}</td><td style=\"{TC};text-align:right\">{}</td><td style=\"{TC};text-align:right\">{}</td><td style=\"{TC};text-align:center\">{per}</td><td style=\"{TC};text-align:right\">{}</td></tr>",
        index + 1,
        esc(name),
        esc(&line.hsn),
        inr(line.inclusive_rate),
        inr(line.taxable_rate),
        inr(line.taxable_amount),
    )
}

fn hsn_row(index: usize, row: &GstHsnSummary, interstate: bool) -> String {
    let bg = if index % 2 == 1 {
        "background:#f7f9fc"
    } else {
        ""
    };
    let cell = |align: &str, text: String| {
        format!("<td style=\"{B};padding:5px;text-align:{align}\">{text}</td>")
    };
    if interstate {
        format!(
            "<tr style=\"{bg}\">{}{}{}{}{}</tr>",
            cell("center", esc(&row.hsn)),
            cell("right", inr(row.taxable)),
            cell("center", format!("{}%", percent(row.gst_basis_points))),
            cell("right", inr(row.integrated_tax)),
            cell("right", inr(row.integrated_tax)),
        )
    } else {
        let half = format!("{}%", percent(row.gst_basis_points / 2));
        format!(
            "<tr style=\"{bg}\">{}{}{}{}{}{}{}</tr>",
            cell("center", esc(&row.hsn)),
            cell("right", inr(row.taxable)),
            cell("center", half.clone()),
            cell("right", inr(row.half_tax)),
            cell("center", half),
            cell("right", inr(row.half_tax)),
            cell("right", inr(row.total_tax)),
        )
    }
}

fn meta_cell(l1: &str, v1: &str, l2: &str, v2: &str) -> String {
    let value = |v: &str| {
        if v.is_empty() {
            "&nbsp;".to_owned()
        } else {
            v.to_owned()
        }
    };
    format!(
        "<div style=\"display:flex\"><div style=\"width:50%;border-right:1px solid #c2c8d2;border-bottom:1px solid #c2c8d2;padding:2px 5px;min-height:24px\"><span style=\"font-size:8.5px;color:#555\">{l1}</span><br><b>{}</b></div><div style=\"width:50%;border-bottom:1px solid #c2c8d2;padding:2px 5px\"><span style=\"font-size:8.5px;color:#555\">{l2}</span><br><b>{}</b></div></div>",
        value(v1),
        value(v2)
    )
}

fn header(invoice: &GstBillInvoice, seller: &Seller<'_>) -> String {
    let bill = &invoice.record;
    let party = invoice.party(None);
    let mut pairs = vec![
        ("Invoice No.".to_owned(), esc(&invoice.invoice_number())),
        ("Dated".to_owned(), esc(&invoice.dated())),
    ];
    pairs.extend(
        invoice
            .meta()
            .into_iter()
            .map(|(label, value)| (label.to_owned(), esc(&value))),
    );
    let mut grid = String::new();
    for pair in pairs.chunks(2) {
        let (l2, v2) = pair.get(1).cloned().unwrap_or_default();
        grid += &meta_cell(&pair[0].0, &pair[0].1, &l2, &v2);
    }
    let mut buyer = format!("<b>{}</b>", esc(&party.name));
    for (prefix, value) in [
        ("", &party.address),
        ("Ph: ", &party.phone),
        ("Email: ", &party.email),
        ("GSTIN/UIN : ", &party.gstin),
    ] {
        if !value.is_empty() {
            buyer += &format!("<br>{prefix}{}", esc(value));
        }
    }
    buyer += &format!(
        "<br>State Name : {}, Code : {}",
        esc(&party.state),
        esc(&party.state_code)
    );
    let fssai = if seller.fssai.is_empty() {
        String::new()
    } else {
        format!("FSSAI License Number: {}<br>", esc(seller.fssai))
    };
    let terms = if bill.terms_delivery.is_empty() {
        "&nbsp;".to_owned()
    } else {
        esc(&bill.terms_delivery)
    };
    format!(
        "<div style=\"display:flex\"><div style=\"width:56%;border-right:1px solid #c2c8d2\"><div style=\"padding:6px;border-bottom:1px solid #c2c8d2\"><b style=\"font-size:13px\">{}</b><br>{}<br>{fssai}GSTIN/UIN: {}<br>State Name : Uttar Pradesh, Code : 09</div><div style=\"padding:6px\"><span style=\"font-size:8.5px;color:#555\">Buyer (Bill to)</span><br>{buyer}</div></div><div style=\"width:44%;display:flex;flex-direction:column\">{grid}<div style=\"flex:1;border-top:1px solid #c2c8d2;padding:3px 7px;min-height:30px\"><span style=\"font-size:8.5px;color:#555\">Terms of Delivery</span><br><b>{terms}</b></div></div></div>",
        esc(seller.name),
        esc(seller.address),
        esc(seller.gstin),
    )
}

fn tax_rows(invoice: &GstBillInvoice, interstate: bool) -> String {
    let totals = invoice.totals;
    let tax = |label: &str, money: Money| {
        format!(
            "<tr><td style=\"{TC};text-align:right\" colspan=\"7\">{label}</td><td style=\"{TC};text-align:right\">{}</td></tr>",
            inr(money)
        )
    };
    let rows = if interstate {
        tax("IGST", totals.igst)
    } else {
        format!(
            "{}\n       {}",
            tax("CGST", totals.cgst),
            tax("SGST", totals.sgst)
        )
    };
    format!(
        "{rows}\n       <tr style=\"font-weight:800;background:#eef1f6\"><td style=\"padding:6px 7px;text-align:right;{EDGE}\" colspan=\"3\">Grand Total</td><td style=\"padding:6px 7px;text-align:right;{EDGE}\">{} CASE</td><td style=\"{EDGE}\" colspan=\"3\"></td><td style=\"padding:6px 7px;text-align:right;font-size:12px;{EDGE}\">&#8377; {}</td></tr>",
        fmt_n(invoice.total_quantity()),
        inr(totals.total)
    )
}

fn below_table(
    invoice: &GstBillInvoice,
    seller: &Seller<'_>,
    interstate: bool,
) -> Result<String, String> {
    let totals = invoice.totals;
    let summary = invoice.hsn_summary().map_err(|e| format!("{e:?}"))?;
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
    let lines = invoice.lines().map_err(|e| format!("{e:?}"))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn number_formats_follow_the_html_helpers() {
        assert_eq!(inr(Money::from_minor_units(12_345_678)), "1,23,456.78");
        assert_eq!(inr(Money::from_minor_units(5)), "0.05");
        assert_eq!(percent(2_000), "20");
        assert_eq!(percent(250), "2.5");
        assert_eq!(
            rupees_words(Money::from_minor_units(52_500)),
            "INR Five Hundred Twenty Five Only"
        );
        assert_eq!(
            rupees_words(Money::from_minor_units(12_525_050)),
            "INR One Lakh Twenty Five Thousand Two Hundred Fifty and Fifty paise Only"
        );
        assert_eq!(esc("<a & \"b\">"), "&lt;a &amp; &quot;b&quot;&gt;");
    }
}
