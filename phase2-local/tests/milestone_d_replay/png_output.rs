use std::cell::{OnceCell, RefCell};

use alm_phase2_local::confirmation::{
    BalanceMode, EstimateDisplay, EstimateLineDisplay, ImageFile, ImageHandoff,
    ImageHandoffRequest, InvoiceHandoff, InvoiceHandoffRequest, InvoiceImageAttempts,
    PurchaseBusiness, PurchaseDraft, PurchaseLine, PurchaseOrderHandoff, PurchaseOrderSelection,
    PurchaseOutputRequest, PurchaseSupplier, PurchaseWhatsAppRequest, ShareRejected,
    TextHandoffIntent,
};

use super::fixture::{A, B, ITEM_A, ITEM_B, OWNER, SALE, cancelled_fixture, feb, money, qty};
use super::png_encoder::{self as png, Encoder, FALLBACK, FALLBACK_URL, MIME, Mode, PRIMARY};

#[derive(Debug, Eq, PartialEq)]
enum Action {
    EstimateHtml,
    HandDrawnCanvas,
    CanvasToDataUrl(&'static str),
    CanvasDecode,
    PoHtml(u32),
    Probe(ImageFile),
    Download(ImageFile),
    Text(TextHandoffIntent),
}

struct Sink<'a> {
    encoder: &'a Encoder,
    actions: &'a RefCell<Vec<Action>>,
}

impl ImageHandoff for Sink<'_> {
    fn can_share(&mut self, file: &ImageFile) -> bool {
        self.actions.borrow_mut().push(Action::Probe(file.clone()));
        false
    }

    fn share(&mut self, _: &ImageFile, _: &str, _: &str) -> Result<(), ShareRejected> {
        panic!("unexpected share request")
    }

    fn download(&mut self, file: &ImageFile) {
        self.actions
            .borrow_mut()
            .push(Action::Download(file.clone()));
    }

    fn chat(&mut self, _: &str) {
        panic!("unexpected chat request")
    }
}

impl InvoiceHandoff for Sink<'_> {
    fn text(&mut self, intent: TextHandoffIntent) {
        self.actions.borrow_mut().push(Action::Text(intent));
    }
}

impl PurchaseOrderHandoff for Sink<'_> {
    fn render_png(&mut self, html: &str, width: u32) -> Option<Vec<u8>> {
        assert!(html.contains(ITEM_A));
        self.actions.borrow_mut().push(Action::PoHtml(width));
        self.encoder.render().map(|blob| blob.bytes.clone())
    }

    fn text(&mut self, intent: TextHandoffIntent) {
        self.actions.borrow_mut().push(Action::Text(intent));
    }
}

fn file(name: &str, bytes: &[u8]) -> ImageFile {
    ImageFile {
        name: name.into(),
        mime: "image/png",
        bytes: bytes.to_vec(),
    }
}

#[test]
fn estimate_keeps_primary_result_separate_from_canvas_fallback() {
    let fixture = cancelled_fixture();
    let request = InvoiceHandoffRequest {
        image: ImageHandoffRequest {
            role: OWNER,
            draft_id: SALE,
            business_name: "Synthetic Business",
            saved_customer_mobile: None,
        },
        tagline: "Synthetic only",
        balance_mode: BalanceMode::AsOnBillDate,
        display: EstimateDisplay {
            today: feb(21),
            lines: &[
                EstimateLineDisplay {
                    product: A,
                    label: ITEM_A,
                },
                EstimateLineDisplay {
                    product: B,
                    label: ITEM_B,
                },
            ],
        },
    };

    for mode in [Mode::Blob, Mode::Missing, Mode::Null] {
        let encoder = Encoder::new(mode);
        let actions = RefCell::new(Vec::new());
        let primary = OnceCell::new();
        let fallback = OnceCell::new();
        let attempts = InvoiceImageAttempts {
            html: || {
                actions.borrow_mut().push(Action::EstimateHtml);
                primary
                    .get_or_init(|| encoder.render())
                    .as_ref()
                    .map(|blob| blob.bytes.as_slice())
            },
            canvas: || {
                assert_eq!(
                    encoder.events.borrow().last(),
                    Some(&png::Event::Callback(png::Outcome::Null))
                );
                actions.borrow_mut().extend([
                    Action::HandDrawnCanvas,
                    Action::CanvasToDataUrl(MIME),
                    Action::CanvasDecode,
                ]);
                Some(
                    fallback
                        .get_or_init(|| png::decode(FALLBACK_URL))
                        .bytes
                        .as_slice(),
                )
            },
        };
        fixture
            .handoff_invoice(
                request,
                attempts,
                &mut Sink {
                    encoder: &encoder,
                    actions: &actions,
                },
            )
            .unwrap();

        encoder.assert_trace("estimate");
        let estimate = |bytes| file("Estimate-1.png", bytes);
        let expected = match mode {
            Mode::Blob | Mode::Missing => vec![
                Action::EstimateHtml,
                Action::Probe(estimate(PRIMARY)),
                Action::Download(estimate(PRIMARY)),
            ],
            Mode::Null => vec![
                Action::EstimateHtml,
                Action::HandDrawnCanvas,
                Action::CanvasToDataUrl("image/png"),
                Action::CanvasDecode,
                Action::Probe(estimate(FALLBACK)),
                Action::Download(estimate(FALLBACK)),
            ],
        };
        let observed = actions.into_inner();
        println!("estimate {mode:?} caller: {observed:?}");
        assert_eq!(observed, expected);
    }
}

#[test]
fn supplier_po_routes_null_to_text_without_second_render() {
    let mut fixture = cancelled_fixture();
    let order = fixture
        .save_purchase_order(
            OWNER,
            &PurchaseDraft {
                supplier: Some(PurchaseSupplier {
                    id: None,
                    name: "Synthetic Supplier".into(),
                }),
                date: Some(feb(21)),
                note: String::new(),
                items: vec![PurchaseLine {
                    item: ITEM_A.into(),
                    product_id: None,
                    ordered: qty(3),
                    rate: money(100),
                }],
            },
        )
        .unwrap();
    let request = PurchaseWhatsAppRequest {
        output: PurchaseOutputRequest {
            selection: PurchaseOrderSelection {
                role: OWNER,
                id: order,
                number: 1,
            },
            business: PurchaseBusiness {
                name: "Synthetic Business",
                address: "",
                mobile: "",
            },
            products: &[],
        },
        contacts: &[],
    };

    for mode in [Mode::Blob, Mode::Missing, Mode::Null] {
        let encoder = Encoder::new(mode);
        let actions = RefCell::new(Vec::new());
        fixture
            .handoff_purchase_order(
                request,
                &mut Sink {
                    encoder: &encoder,
                    actions: &actions,
                },
            )
            .unwrap();

        encoder.assert_trace("supplier PO");
        let expected = match mode {
            Mode::Blob | Mode::Missing => vec![
                Action::PoHtml(760),
                Action::Probe(file("o.png", PRIMARY)),
                Action::Download(file("Order-PO-001.png", PRIMARY)),
            ],
            Mode::Null => vec![Action::PoHtml(760), Action::Text(TextHandoffIntent {
                url: concat!("https://wa.me/?text=*Order%20%E2%80%94%20Synthetic%20Business*%20(21-Feb-2025)%0A%0A",
                    "Synthetic%20A%20%E2%80%94%203%20cases%0A").into(),
                target: "_blank",
            })],
        };
        let observed = actions.into_inner();
        println!("supplier PO {mode:?} caller: {observed:?}");
        assert_eq!(observed, expected);
    }
}
