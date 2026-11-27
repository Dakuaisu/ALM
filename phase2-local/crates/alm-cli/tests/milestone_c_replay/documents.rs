use std::time::Duration;

use alm_core::BillDate;
use alm_core::confirmation::{
    BalanceMode, ConfirmationFixture, EstimateDisplay, EstimateLineDisplay, EstimateRequest,
    GstError, GstInvoice, GstJurisdiction, GstRequest, PrintIntent,
};

use super::{A, B, BUYER, DENIED, ITEM_A, ITEM_B, MANAGER, OWNER, SALE, feb};

#[path = "../../../alm-store/tests/common/ordinary_gst_fixture.rs"]
mod ordinary_gst_fixture;
use ordinary_gst_fixture::GST_METADATA;

#[derive(Clone, Copy)]
pub(super) enum BillVersion {
    Original,
    Edited,
}

impl BillVersion {
    const fn line_count(self) -> usize {
        match self {
            Self::Original => 1,
            Self::Edited => 2,
        }
    }
}

const DISPLAY: [EstimateLineDisplay<'static>; 2] = [
    EstimateLineDisplay {
        product: A,
        label: ITEM_A,
    },
    EstimateLineDisplay {
        product: B,
        label: ITEM_B,
    },
];

pub(super) struct EstimateCheckpoint {
    pub(super) today: BillDate,
    pub(super) latest_label: &'static str,
    pub(super) latest_due: &'static str,
    pub(super) cancelled: bool,
}

pub(super) fn assert_estimate(
    fixture: &ConfirmationFixture,
    version: BillVersion,
    checkpoint: EstimateCheckpoint,
) {
    let (total, bill_due, items): (_, _, &[_]) = match version {
        BillVersion::Original => (
            "₹236.00",
            "₹136.00",
            &["<td>Synthetic A</td><td>2</td><td>₹118.00</td><td>₹236.00</td>"],
        ),
        BillVersion::Edited => (
            "₹410.00",
            "₹310.00",
            &[
                "<td>Synthetic A</td><td>2</td><td>₹177.00</td><td>₹354.00</td>",
                "<td>Synthetic B</td><td>1</td><td>₹56.00</td><td>₹56.00</td>",
            ],
        ),
    };

    for (mode, paid_label, due_label, due) in [
        (BalanceMode::AsOnBillDate, "Paid", "Balance Due", bill_due),
        (
            BalanceMode::Latest,
            "Paid on this bill",
            checkpoint.latest_label,
            checkpoint.latest_due,
        ),
    ] {
        let request = EstimateRequest {
            role: OWNER,
            draft_id: SALE,
            balance_mode: mode,
            display: EstimateDisplay {
                today: checkpoint.today,
                lines: &DISPLAY[..version.line_count()],
            },
        };
        let manager = EstimateRequest {
            role: MANAGER,
            ..request
        };
        // DEC-034: a Manager prints the same estimates as the Owner.
        let mut manager_captured = Vec::new();
        fixture
            .print_estimate(manager, &mut |doc| manager_captured.push(doc))
            .unwrap();
        fixture
            .save_estimate_as_pdf(manager, &mut |doc| manager_captured.push(doc))
            .unwrap();
        let mut captured = Vec::new();
        fixture
            .print_estimate(request, &mut |doc| captured.push(doc))
            .unwrap();
        fixture
            .save_estimate_as_pdf(request, &mut |doc| captured.push(doc))
            .unwrap();
        let [print, pdf] = captured.as_slice() else {
            panic!("expected one capture per estimate output");
        };
        assert_eq!(
            (print.target, print.intent),
            ("_print", PrintIntent::OnLoad)
        );
        assert_eq!(
            (pdf.target, pdf.intent),
            ("_pdf", PrintIntent::AfterLoad(Duration::from_millis(350)))
        );
        assert_eq!(print.html, pdf.html);
        assert_eq!(manager_captured, captured);
        let html = &print.html;
        for fragment in [
            "<title>Estimate-1-Synthetic Buyer</title>",
            "<p>Date: 21-Feb-2025</p>",
            "<p>Bill To: Synthetic Buyer</p>",
            &format!("<th>Bill Amount</th><td>{total}</td>"),
            &format!("<th>{paid_label}</th><td>₹100.00</td>"),
            &format!("<th>{due_label}</th><td>{due}</td>"),
        ]
        .into_iter()
        .chain(items.iter().copied())
        {
            assert!(
                html.contains(fragment),
                "missing {fragment} in {mode:?} estimate: {html}"
            );
        }
        let cancelled = html.contains("<p>CANCELLED</p>");
        assert_eq!(cancelled, checkpoint.cancelled);
        println!(
            "estimate #1 {mode:?}: {} {:?}, {} {:?}; total={total}, paid=₹100.00, due={due}, cancelled={cancelled}",
            print.target, print.intent, pdf.target, pdf.intent
        );
    }
}

pub(super) fn assert_gst(
    fixture: &ConfirmationFixture,
    version: BillVersion,
    cancelled: bool,
) -> GstInvoice {
    let request = GstRequest {
        role: OWNER,
        draft_id: SALE,
        jurisdiction: GstJurisdiction::Intra,
        lines: &GST_METADATA[..version.line_count()],
    };
    let mut captured = Vec::new();
    assert_eq!(
        fixture.emit_gst_invoice(
            GstRequest {
                role: MANAGER,
                ..request
            },
            &mut |doc| captured.push(doc)
        ),
        Err(GstError::Preview(DENIED))
    );
    assert!(captured.is_empty(), "denied GST must not reach the sink");
    fixture
        .emit_gst_invoice(request, &mut |doc| captured.push(doc))
        .unwrap();
    let [gst]: [GstInvoice; 1] = captured.try_into().expect("one ordinary-sale GST capture");
    assert_eq!(
        (
            gst.invoice,
            gst.date,
            gst.customer.as_str(),
            gst.cancelled,
            gst.jurisdiction
        ),
        (1, feb(21), BUYER, cancelled, GstJurisdiction::Intra)
    );
    let amounts = match version {
        BillVersion::Original => [23_600, 20_000, 3_600, 1_800, 1_800, 0],
        BillVersion::Edited => [41_000, 35_000, 6_000, 3_000, 3_000, 0],
    };
    assert_eq!(
        [
            gst.total,
            gst.taxable,
            gst.tax,
            gst.cgst,
            gst.sgst,
            gst.igst
        ]
        .map(alm_core::Money::minor_units),
        amounts,
    );
    let groups = gst
        .groups
        .iter()
        .map(|group| {
            (
                group.hsn.as_str(),
                group.combined_rate,
                group.taxable.minor_units(),
                group.tax.minor_units(),
            )
        })
        .collect::<Vec<_>>();
    match version {
        BillVersion::Original => assert_eq!(groups, [("22029920", 1_800, 20_000, 3_600)]),
        BillVersion::Edited => assert_eq!(
            groups,
            [
                ("22029920", 1_800, 30_000, 5_400),
                ("21069099", 1_200, 5_000, 600)
            ]
        ),
    }
    println!(
        "GST invoice={} cancelled={}: total={}, taxable={}, tax={}, CGST={}, SGST={}, IGST={}; groups={groups:?}",
        gst.invoice,
        gst.cancelled,
        gst.total.minor_units(),
        gst.taxable.minor_units(),
        gst.tax.minor_units(),
        gst.cgst.minor_units(),
        gst.sgst.minor_units(),
        gst.igst.minor_units()
    );
    gst
}
