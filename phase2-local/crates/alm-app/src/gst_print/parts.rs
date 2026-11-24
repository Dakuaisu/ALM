//! Invoice fragments: line rows, HSN summary rows, the meta grid, header and tax rows.
use alm_core::Money;
use alm_core::confirmation::{GstBillInvoice, GstHsnSummary, GstInvoiceLine, GstUnit};

use super::format::{esc, fmt_n, inr, percent};
use super::{B, EDGE, Seller, TC};

pub(super) fn row(index: usize, name: &str, line: &GstInvoiceLine) -> String {
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

pub(super) fn hsn_row(index: usize, row: &GstHsnSummary, interstate: bool) -> String {
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

pub(super) fn header(invoice: &GstBillInvoice, seller: &Seller<'_>) -> String {
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

pub(super) fn tax_rows(invoice: &GstBillInvoice, interstate: bool) -> String {
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
