use std::cell::Cell;
use std::num::NonZeroU32;

use alm_store::confirmation::{
    BalanceMode, ConfirmationFixture, EstimateDisplay, EstimateLineDisplay, EstimateRequest,
    GstBillDraft, GstBillId, GstBillInvoice, GstBillLineInput, GstBillNumber, GstJurisdiction,
    ImageFile, ImageHandoff, ImageHandoffRequest, InvoiceHandoff, InvoiceHandoffRequest,
    InvoiceImageAttempts, RequestId, ShareRejected, TextHandoffIntent,
};
use alm_store::synthetic_read_model::BusinessProfile;
use alm_store::{
    BillDate, DraftCustomer, DraftId, DraftLine, Money, ProductId, ProvisionalDraft, Role,
};

#[derive(Default)]
struct Sink {
    downloaded: bool,
}
impl ImageHandoff for Sink {
    fn can_share(&mut self, _: &ImageFile) -> bool {
        false
    }
    fn share(&mut self, _: &ImageFile, _: &str, _: &str) -> Result<(), ShareRejected> {
        Ok(())
    }
    fn download(&mut self, _: &ImageFile) {
        self.downloaded = true;
    }
    fn chat(&mut self, _: &str) {}
}
impl InvoiceHandoff for Sink {
    fn text(&mut self, _: TextHandoffIntent) {}
}

pub struct Expected<'a> {
    pub fssai: &'a str,
    pub gstin: &'a str,
    pub visible_f: bool,
    pub visible_g: bool,
}
#[derive(Debug, Eq, PartialEq)]
pub struct Outputs {
    pub html: String,
    pub draws: Vec<String>,
    pub gst_header: String,
}

pub fn fixture() -> (ConfirmationFixture, GstBillInvoice) {
    let product = ProductId::new(1);
    let id = DraftId::new(1);
    let date = BillDate::new(2025, 1, 2).unwrap();
    let mut fixture = ConfirmationFixture::new(product, 5, Money::from_minor_units(100)).unwrap();
    fixture
        .register_draft(
            ProvisionalDraft::new(id, vec![DraftLine::new(product, NonZeroU32::MIN)])
                .with_customer(DraftCustomer::Typed("Local - Neighbor".into()))
                .with_bill_date(Some(date)),
        )
        .unwrap();
    let review = fixture.review_walk_in_cash(Role::Owner, id).unwrap();
    fixture
        .confirm(Role::Owner, RequestId::new(1), &review)
        .unwrap();
    let lines = [GstBillLineInput {
        item: "Item",
        quantity: 1,
        inclusive_rate: Money::from_minor_units(100),
        gst_basis_points: 0,
    }];
    let draft = GstBillDraft {
        id: GstBillId(1),
        number: GstBillNumber(1),
        customer: "Buyer",
        date: Some(date),
        gstin: Some("123456789012345"),
        eway_number: None,
        jurisdiction: GstJurisdiction::Intra,
        lines: &lines,
    };
    let mut invoice = None;
    fixture
        .save_gst_bill(Role::Owner, &draft, &mut |value| {
            invoice = Some(value.clone())
        })
        .unwrap();
    (fixture, invoice.unwrap())
}

pub fn capture(
    fixture: &ConfirmationFixture,
    invoice: &GstBillInvoice,
    profile: &BusinessProfile,
    expected: &Expected<'_>,
) -> Outputs {
    let lines = [EstimateLineDisplay {
        product: ProductId::new(1),
        label: "Item",
    }];
    let display = EstimateDisplay {
        today: BillDate::new(2025, 1, 2).unwrap(),
        lines: &lines,
    };
    let request = EstimateRequest {
        role: Role::Owner,
        draft_id: DraftId::new(1),
        balance_mode: BalanceMode::AsOnBillDate,
        display,
    };
    let handoff = InvoiceHandoffRequest {
        image: ImageHandoffRequest {
            role: Role::Owner,
            draft_id: DraftId::new(1),
            business_name: "Synthetic",
            saved_customer_mobile: None,
        },
        tagline: "",
        balance_mode: BalanceMode::AsOnBillDate,
        display,
    };
    let mut html = String::new();
    fixture
        .print_estimate_with_business(request, profile, &mut |doc| html = doc.html)
        .unwrap();
    let f_html = "<div style=\"font-size:12px;color:#000\">FSSAI: F&lt;&amp;</div>";
    let g_html = "<div style=\"font-size:12px;color:#000\">GSTIN: G&lt;&amp;</div>";
    assert_eq!(html.contains(f_html), expected.visible_f);
    assert_eq!(html.contains(g_html), expected.visible_g);
    assert_eq!(html.contains("FSSAI:"), expected.visible_f);
    assert_eq!(html.contains("GSTIN:"), expected.visible_g);
    if expected.visible_f && expected.visible_g {
        assert!(html.contains(&format!("{f_html}{g_html}")));
    }
    let mut draws = Vec::new();
    let mut sink = Sink::default();
    fixture
        .handoff_invoice_with_business(
            handoff,
            profile,
            InvoiceImageAttempts {
                html: || None,
                canvas: || Some(b"canvas".as_slice()),
            },
            &mut |text| draws.push(text.to_owned()),
            &mut sink,
        )
        .unwrap();
    assert!(sink.downloaded);
    let mut expected_draws = Vec::new();
    if expected.visible_f {
        expected_draws.push("FSSAI: F<&".to_owned());
    }
    if expected.visible_g {
        expected_draws.push("GSTIN: G<&".to_owned());
    }
    assert_eq!(draws, expected_draws);
    let fallback_called = Cell::new(false);
    let mut skipped_draws = Vec::new();
    let mut html_sink = Sink::default();
    fixture
        .handoff_invoice_with_business(
            handoff,
            profile,
            InvoiceImageAttempts {
                html: || Some(b"html".as_slice()),
                canvas: || {
                    fallback_called.set(true);
                    Some(b"canvas".as_slice())
                },
            },
            &mut |text| skipped_draws.push(text.to_owned()),
            &mut html_sink,
        )
        .unwrap();
    assert!(html_sink.downloaded && !fallback_called.get() && skipped_draws.is_empty());
    let gst_header = invoice.business_header_html(profile);
    let fssai = if expected.fssai.is_empty() {
        ""
    } else {
        "FSSAI License Number: F&lt;&amp;<br>"
    };
    let gstin = if expected.gstin.is_empty() {
        ""
    } else {
        "G&lt;&amp;"
    };
    assert_eq!(
        gst_header,
        format!("<div>{fssai}GSTIN/UIN: {gstin}<br>State Name : Uttar Pradesh, Code : 09</div>")
    );
    assert_eq!(invoice.record.gstin.as_deref(), Some("123456789012345"));
    assert!(!gst_header.contains("123456789012345"));
    Outputs {
        html,
        draws,
        gst_header,
    }
}
