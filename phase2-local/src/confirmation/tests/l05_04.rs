// L05-04: waInvoice and waText, ALM_Business_Manager.html:4459–4476, 4768–4817.
use std::cell::RefCell;

use super::*;
use crate::DraftCustomerId;
use l05_03::{MOBILE, PNG};

const LINES: &[EstimateLineDisplay<'_>] = &[EstimateLineDisplay {
    product: ProductId::new(1),
    label: "नमूना / 50%",
}];

fn fixture() -> ConfirmationFixture {
    let mut fixture = l05_03::fixture();
    fixture
        .set_unit_price(Money::from_minor_units(12_345_670))
        .unwrap();
    fixture
        .register_draft(draft(42, 1).with_customer(DraftCustomer::Selected {
            id: DraftCustomerId::new(MOBILE.0.into()),
            name: "Buyer + मित्र".into(),
        }))
        .unwrap();
    let review = fixture
        .review_named(Role::Owner, DraftId::new(42), Money::from_minor_units(5))
        .unwrap();
    fixture
        .confirm(Role::Owner, RequestId::new(401), &review)
        .unwrap();
    fixture.preview_payments.push(PreviewPayment {
        customer_id: MOBILE.0.into(),
        amount: Money::from_minor_units(1_000),
        date: BillDate::new(2024, 3, 2).unwrap(),
        after_invoice: 2,
        mode: None,
    });
    fixture
}

fn request(mobile: Option<(&'static str, &'static str)>) -> InvoiceHandoffRequest<'static> {
    InvoiceHandoffRequest {
        image: ImageHandoffRequest {
            role: Role::Owner,
            draft_id: DraftId::new(42),
            business_name: "ALM & Co.",
            saved_customer_mobile: mobile,
        },
        tagline: "Ready? #1 + tax=₹ (synthetic!)",
        balance_mode: BalanceMode::AsOnBillDate,
        display: EstimateDisplay {
            today: BillDate::new(2024, 3, 5).unwrap(),
            lines: LINES,
        },
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Call {
    Html,
    Canvas,
    Probe(ImageFile),
    Download(ImageFile),
    Chat(String),
    Text(TextHandoffIntent),
}

struct Capture<'a>(&'a RefCell<Vec<Call>>);

impl ImageHandoff for Capture<'_> {
    fn can_share(&mut self, file: &ImageFile) -> bool {
        self.0.borrow_mut().push(Call::Probe(file.clone()));
        false
    }

    fn share(&mut self, _: &ImageFile, _: &str, _: &str) -> Result<(), ShareRejected> {
        panic!("native share was not supported");
    }

    fn download(&mut self, file: &ImageFile) {
        self.0.borrow_mut().push(Call::Download(file.clone()));
    }

    fn chat(&mut self, phone: &str) {
        self.0.borrow_mut().push(Call::Chat(phone.into()));
    }
}

impl InvoiceHandoff for Capture<'_> {
    fn text(&mut self, intent: TextHandoffIntent) {
        self.0.borrow_mut().push(Call::Text(intent));
    }
}

fn run_case(
    request: InvoiceHandoffRequest<'_>,
    [html, canvas]: [Option<&[u8]>; 2],
) -> (Result<(), EstimateError>, Vec<Call>) {
    let fixture = fixture();
    let before = fixture.clone();
    let calls = RefCell::new(Vec::new());

    let result = fixture.handoff_invoice(
        request,
        InvoiceImageAttempts {
            html: || {
                calls.borrow_mut().push(Call::Html);
                html
            },
            canvas: || {
                calls.borrow_mut().push(Call::Canvas);
                canvas
            },
        },
        &mut Capture(&calls),
    );

    assert_eq!(fixture, before);
    (result, calls.into_inner())
}

fn expected_file() -> ImageFile {
    ImageFile {
        name: "Estimate-2.png".into(),
        mime: "image/png",
        bytes: PNG.to_vec(),
    }
}

#[test]
fn l05_04_html_png_bypasses_canvas_and_text() {
    let (result, calls) = run_case(request(Some(MOBILE)), [Some(PNG), Some(PNG)]);
    assert_eq!(result, Ok(()));
    assert_eq!(
        calls,
        [
            Call::Html,
            Call::Probe(expected_file()),
            Call::Download(expected_file()),
            Call::Chat("910000000000".into()),
        ]
    );
}

#[test]
fn l05_04_canvas_png_dispatches_once_when_html_is_absent() {
    let (result, calls) = run_case(request(Some(MOBILE)), [None, Some(PNG)]);
    assert_eq!(result, Ok(()));
    assert_eq!(
        calls,
        [
            Call::Html,
            Call::Canvas,
            Call::Probe(expected_file()),
            Call::Download(expected_file()),
            Call::Chat("910000000000".into()),
        ]
    );
}

// Decode the emitted component independently; require escaped delimiters, not form encoding.
fn decode_component(encoded: &str) -> String {
    let mut bytes = Vec::new();
    let mut chars = encoded.bytes();
    while let Some(byte) = chars.next() {
        if byte == b'%' {
            let pair: String = chars.by_ref().take(2).map(char::from).collect();
            assert_eq!(pair.len(), 2);
            bytes.push(u8::from_str_radix(&pair, 16).unwrap());
        } else {
            assert!(byte.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&byte));
            bytes.push(byte);
        }
    }
    String::from_utf8(bytes).unwrap()
}

const MESSAGE_HEAD: &str = "*ALM & Co.*\n\
Ready? #1 + tax=₹ (synthetic!)\n\
----------------------\n\
*Estimate #2*  (29-Feb-2024)\n\
Party: Buyer + मित्र\n\n\
नमूना / 50%  ×1 = ₹1,23,456.7\n\
Loading/Delivery: ₹0\n\
----------------------\n\
*Bill Amount: ₹1,23,456.7*\n";

fn text_only(request: InvoiceHandoffRequest<'_>, expected_prefix: &str) -> String {
    let (result, calls) = run_case(request, [None, None]);
    assert_eq!(result, Ok(()));
    let [Call::Html, Call::Canvas, Call::Text(intent)] = calls.as_slice() else {
        panic!("expected only the text intent after both render attempts: {calls:?}");
    };
    assert_eq!(intent.target, "_blank");
    let encoded = intent.url.strip_prefix(expected_prefix).unwrap();
    assert!(encoded.starts_with("*ALM%20%26%20Co.*%0A"));
    decode_component(encoded)
}

#[test]
fn l05_04_both_absent_emit_only_source_text_with_or_without_matching_phone() {
    for (mobile, prefix) in [
        (Some(MOBILE), "https://wa.me/910000000000?text="),
        (None, "https://wa.me/?text="),
        (Some(("other-id", MOBILE.1)), "https://wa.me/?text="),
        (Some((MOBILE.0, "123-456")), "https://wa.me/?text="),
    ] {
        assert_eq!(
            text_only(request(mobile), prefix),
            format!(
                "{MESSAGE_HEAD}Paid: ₹0.05\nOld Balance: ₹90\n*Balance Due: ₹1,23,546.65*\n\nThank you! 🙏"
            )
        );
    }
}

#[test]
fn l05_04_text_uses_latest_balance_when_selected() {
    let mut request = request(Some(MOBILE));
    request.balance_mode = BalanceMode::Latest;
    assert_eq!(
        text_only(request, "https://wa.me/910000000000?text="),
        format!(
            "{MESSAGE_HEAD}Paid on this bill: ₹0.05\n*Total Due (as on 05-Mar-2024): ₹1,23,536.65*\n\nThank you! 🙏"
        )
    );
}

#[test]
fn l05_04_manager_is_denied_before_any_callback() {
    let mut request = request(Some(MOBILE));
    request.image.role = Role::Manager;
    let (result, calls) = run_case(request, [None, Some(PNG)]);
    assert_eq!(
        result,
        Err(EstimateError::Preview(ConfirmationError::AccessDenied))
    );
    assert!(calls.is_empty());
}
