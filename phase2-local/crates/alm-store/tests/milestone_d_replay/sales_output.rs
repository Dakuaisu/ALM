use std::cell::Cell;

use alm_store::confirmation::{
    BalanceMode, ConfirmationError, EstimateDisplay, EstimateError, EstimateLineDisplay, ImageFile,
    ImageHandoff, InvoiceHandoff, InvoiceImageAttempts, SalesOutput, ShareRejected,
    TextHandoffIntent,
};
use alm_store::{
    BillDate, DraftCustomer, DraftCustomerId, DraftId, DraftLine, ProductId, ProvisionalDraft, Role,
};

use super::fixture::{self, A, BUYER_ID, SALE, feb, money, qty};

const DECOY: DraftId = DraftId::new(22);
const LABELS: [EstimateLineDisplay<'static>; 2] = [
    EstimateLineDisplay {
        product: A,
        label: "Synthetic A",
    },
    EstimateLineDisplay {
        product: ProductId::new(2),
        label: "Synthetic B",
    },
];
const CONTACTS: [(&str, &str); 2] = [("decoy", "8888888888"), (BUYER_ID, "+91 (00000) 00000")];

#[derive(Default)]
struct Capture {
    files: Vec<ImageFile>,
    chats: Vec<String>,
    texts: Vec<TextHandoffIntent>,
    probes: usize,
}
impl ImageHandoff for Capture {
    fn can_share(&mut self, _: &ImageFile) -> bool {
        self.probes += 1;
        false
    }
    fn share(&mut self, _: &ImageFile, _: &str, _: &str) -> Result<(), ShareRejected> {
        panic!("unexpected share")
    }
    fn download(&mut self, file: &ImageFile) {
        self.files.push(file.clone());
    }
    fn chat(&mut self, phone: &str) {
        self.chats.push(phone.into());
    }
}
impl InvoiceHandoff for Capture {
    fn text(&mut self, intent: TextHandoffIntent) {
        self.texts.push(intent);
    }
}

fn output<'a>(contacts: &'a [(&'a str, &'a str)]) -> SalesOutput<'a> {
    SalesOutput {
        business_name: "Synthetic Business",
        tagline: "Synthetic only",
        balance_mode: BalanceMode::Latest,
        display: EstimateDisplay {
            today: BillDate::new(2025, 3, 1).unwrap(),
            lines: &LABELS,
        },
        contacts,
    }
}

#[test]
fn active_row_whatsapp_uses_selected_sale_and_matching_contact() {
    let mut fixture = fixture::cancelled_fixture();
    fixture.undo_cancel_sale(Role::Owner, SALE).unwrap();
    let decoy = ProvisionalDraft::new(DECOY, vec![DraftLine::new(A, qty(1))])
        .with_bill_date(Some(feb(22)))
        .with_customer(DraftCustomer::Selected {
            id: DraftCustomerId::new("decoy".into()),
            name: "Decoy".into(),
        });
    fixture.register_draft(decoy).unwrap();
    let review = fixture
        .review_named(Role::Owner, DECOY, money(100))
        .unwrap();
    fixture
        .confirm(
            Role::Owner,
            alm_store::confirmation::RequestId::new(2),
            &review,
        )
        .unwrap();
    let row = fixture.sales_output_row(Role::Owner, SALE).unwrap();
    for (contacts, expected) in [
        (&CONTACTS[..], vec!["910000000000".to_owned()]),
        (&CONTACTS[..1], vec![]),
    ] {
        let mut capture = Capture::default();
        row.whatsapp(
            output(contacts),
            InvoiceImageAttempts {
                html: || Some(&b"png"[..]),
                canvas: || panic!("canvas"),
            },
            &mut capture,
        )
        .unwrap();
        assert_eq!(
            capture.files,
            [ImageFile {
                name: "Estimate-1.png".into(),
                mime: "image/png",
                bytes: b"png".to_vec()
            }]
        );
        assert_eq!(capture.chats, expected);
        assert_eq!(capture.probes, 1);
        assert!(capture.texts.is_empty());
        println!(
            "L05-05 Sales row {SALE:?}: file={}, resolved recipient={:?}, no text",
            capture.files[0].name,
            capture.chats.first()
        );
    }
}

#[test]
fn cancelled_view_reopens_record_and_preview_whatsapp_falls_back_to_exact_text() {
    let fixture = fixture::cancelled_fixture();
    let row = fixture.sales_output_row(Role::Owner, SALE).unwrap();
    let preview = row.view(output(&CONTACTS)).unwrap();
    assert!(preview.html().contains("<p>CANCELLED</p>"));
    assert!(preview.html().contains("Estimate #1"));
    let attempts = Cell::new(0);
    let mut capture = Capture::default();
    preview
        .whatsapp(
            InvoiceImageAttempts {
                html: || {
                    assert_eq!(attempts.replace(1), 0);
                    None
                },
                canvas: || {
                    assert_eq!(attempts.replace(2), 1);
                    None
                },
            },
            &mut capture,
        )
        .unwrap();
    assert_eq!(attempts.get(), 2);
    assert_eq!(capture.texts, [TextHandoffIntent { target: "_blank", url: concat!(
        "https://wa.me/910000000000?text=*Synthetic%20Business*%0A",
        "Synthetic%20only%0A----------------------%0A*Estimate%20%231*%20%20(21-Feb-2025)%0A",
        "Party%3A%20Synthetic%20Buyer%0A%0ASynthetic%20A%20%20%C3%972%20%3D%20%E2%82%B9354%0A",
        "Synthetic%20B%20%20%C3%971%20%3D%20%E2%82%B956%0ALoading%2FDelivery%3A%20%E2%82%B90%0A----------------------%0A",
        "*Bill%20Amount%3A%20%E2%82%B9410*%0APaid%20on%20this%20bill%3A%20%E2%82%B9100%0A",
        "*Total%20Due%20(as%20on%2001-Mar-2025)%3A%20%E2%82%B9-50*%0A%0AThank%20you!%20%F0%9F%99%8F"
    ).into() }]);
    assert!(capture.files.is_empty() && capture.chats.is_empty());
    assert_eq!(capture.probes, 0);
    println!(
        "L05-05 history row {SALE:?}: Estimate #1 CANCELLED markup; HTML then canvas failed; {:?}",
        capture.texts[0]
    );
}

#[test]
fn denied_routes_invoke_no_callbacks() {
    let fixture = fixture::cancelled_fixture();
    assert!(matches!(
        fixture.sales_output_row(Role::Manager, SALE),
        Err(ConfirmationError::AccessDenied)
    ));
    let row = fixture.sales_output_row(Role::Owner, SALE).unwrap();
    let attempts = Cell::new(0);
    let mut capture = Capture::default();
    let result = row.whatsapp(
        output(&CONTACTS),
        InvoiceImageAttempts {
            html: || {
                attempts.set(1);
                None
            },
            canvas: || {
                attempts.set(2);
                None
            },
        },
        &mut capture,
    );
    assert_eq!(
        result,
        Err(EstimateError::Preview(ConfirmationError::InvalidDraft))
    );
    assert_eq!(attempts.get(), 0);
    assert_eq!(capture.probes, 0);
    assert!(capture.files.is_empty() && capture.chats.is_empty() && capture.texts.is_empty());
}
