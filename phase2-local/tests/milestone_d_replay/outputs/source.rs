const FROZEN_SOURCE: &str = include_str!("/Users/apple/ALM/ALM_Business_Manager.html");

fn in_order(value: &str, fragments: &[&str]) {
    let mut remaining = value;
    for fragment in fragments {
        let (_, suffix) = remaining
            .split_once(fragment)
            .unwrap_or_else(|| panic!("missing or out-of-order anchor: {fragment}"));
        remaining = suffix;
    }
}

pub(super) fn compare(html: &str, svg: &str) {
    let number_format = FROZEN_SOURCE
        .split_once("const fmtN = n =>")
        .unwrap()
        .1
        .split_once("const money = n =>")
        .unwrap()
        .0;
    assert!(number_format.contains("toLocaleString(\"en-IN\""));
    assert!(number_format.contains("maximumFractionDigits: 2"));
    let invoice = FROZEN_SOURCE
        .split_once("function invoiceHTML(s, simple, live)")
        .unwrap()
        .1
        .split_once("function custBalanceBefore(s)")
        .unwrap()
        .0;
    let template = invoice
        .split_once("return `<div class=\"invoice\"")
        .unwrap()
        .1;
    in_order(
        template,
        &[
            "ESTIMATE</div>",
            "CANCELLED",
            "<b>No:</b>",
            "<b>Date:</b>",
            "Bill To:",
            "<table class=\"estbl\"",
            ">Item</th>",
            ">Pack</th>",
            ">MRP</th>",
            ">Rate</th>",
            ">Pkt</th>",
            ">Amount</th>",
            "Items Total",
            "Loading/Delivery",
            "Bill Amount",
            "${_paidLabel}",
            "${_balLabel}",
            "Go paperless",
            "Proprietor",
        ],
    );
    for anchor in [
        "max-width:720px",
        "padding:26px 30px",
        "display:flex;justify-content:space-between;border-bottom:3px solid #2563eb",
        "display:flex;justify-content:flex-end;margin-top:10px",
        "#1d4ed8",
        "#2563eb",
        "#eef2fb",
        "#b91c1c",
    ] {
        assert!(template.contains(anchor), "legacy layout anchor {anchor}");
    }
    in_order(
        svg,
        &[
            "viewBox=\"0 0 720 470\"",
            "<rect class=\"paper\"",
            "<g id=\"header\">",
            "<line class=\"rule\" x1=\"30\" y1=\"135\"",
            "id=\"bill-to\" x=\"30\" y=\"160\"",
            "<g id=\"items\">",
            "<g id=\"line-1\">",
            "<g id=\"line-2\">",
            "<g id=\"summary\">",
            "<g id=\"footer\">",
        ],
    );
    let print = FROZEN_SOURCE
        .split_once("function printInvoice(s)")
        .unwrap()
        .1
        .split_once("function pdfInvoice(s)")
        .unwrap()
        .0;
    in_order(
        print,
        &[
            "\"_print\"",
            "${invoiceHTML(s)}",
            "window.onload=function(){window.print();}",
        ],
    );
    let pdf = FROZEN_SOURCE
        .split_once("function pdfInvoice(s)")
        .unwrap()
        .1
        .split_once("function waText(s, live)")
        .unwrap()
        .0;
    in_order(
        pdf,
        &["\"_pdf\"", "${invoiceHTML(s)}", "window.print();},350)"],
    );
    let handoff = FROZEN_SOURCE
        .split_once("function waInvoice(s)")
        .unwrap()
        .1
        .split_once("function cleanPhone(p)")
        .unwrap()
        .0;
    in_order(
        handoff,
        &[
            "navigator.canShare",
            "navigator.share",
            "download: file.name",
            "htmlToPNG(invoiceHTML(s, true), 760",
            "billImageCanvas(s)",
        ],
    );
    in_order(
        svg,
        &[
            "Synthetic Business</text>",
            ">ESTIMATE</text>",
            ">CANCELLED</text>",
            ">No: 1</text>",
            ">Date: 21-Feb-2025</text>",
            ">Bill To: Synthetic Buyer</text>",
            ">Item</text>",
            ">Pack</text>",
            ">MRP</text>",
            ">Rate</text>",
            ">Pkt</text>",
            ">Amount</text>",
            ">Synthetic A</text>",
            ">₹177.00</text>",
            ">2</text>",
            ">₹354.00</text>",
            ">Synthetic B</text>",
            ">₹56.00</text>",
            ">1</text>",
            ">₹56.00</text>",
            ">Items Total</text>",
            ">Loading/Delivery</text>",
            ">Bill Amount</text>",
            ">Paid on this bill</text>",
            ">Total Due (as on 01-Mar-2025)</text>",
            ">₹-50.00</text>",
            "Go paperless",
            ">Proprietor</text>",
        ],
    );
    for anchor in ["width=\"720\"", "#1d4ed8", "#2563eb", "#eef2fb", "#b91c1c"] {
        assert!(
            svg.contains(anchor),
            "synthetic vector layout anchor {anchor}"
        );
    }
    in_order(
        html,
        &[
            "<h1>Estimate #1</h1>",
            "<p>Date: 21-Feb-2025</p>",
            "<p>Bill To: Synthetic Buyer</p>",
            "<p>CANCELLED</p>",
            "<th>Item</th><th>Quantity</th><th>Rate</th><th>Amount</th>",
            "<td>Synthetic A</td><td>2</td><td>₹177.00</td><td>₹354.00</td>",
            "<td>Synthetic B</td><td>1</td><td>₹56.00</td><td>₹56.00</td>",
            "Items Total",
            "<th>Loading/Delivery</th><td>₹0.00</td>",
            "<th>Bill Amount</th><td>₹410.00</td>",
            "<th>Paid on this bill</th><td>₹100.00</td>",
            "<th>Total Due (as on 01-Mar-2025)</th><td>₹-50.00</td>",
        ],
    );
    assert!(!html.contains("<script"));
    assert!(!html.contains("class=\"estbl\""));
}
