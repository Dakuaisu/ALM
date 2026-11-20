use super::{OWNER, PRODUCT, PurchaseOrderId, fixture_with_prior_receipt};
use crate::confirmation::{
    ConfirmationFixture, ImageFile, ImageHandoff, PopupDocument, PrintIntent, PurchaseBusiness,
    PurchaseError, PurchaseOrderHandoff, PurchaseOrderSelection, PurchaseOrderStatus,
    PurchaseOutputRequest, PurchaseProductDisplay, PurchaseSupplierContact,
    PurchaseWhatsAppRequest, ShareRejected, TextHandoffIntent,
};
use crate::{BillDate, Money, ProductId, Role};
use std::time::Duration;

const PRODUCTS: &[PurchaseProductDisplay<'_>] = &[
    PurchaseProductDisplay {
        id: ProductId::new(9),
        item: "Synthetic Orange",
        pack_size: "12",
        uom: "bottles",
        mrp: Some(Money::from_minor_units(6_750)),
    },
    PurchaseProductDisplay {
        id: PRODUCT,
        item: "Renamed Orange",
        pack_size: "24",
        uom: "tins & jars",
        mrp: Some(Money::from_minor_units(8_950)),
    },
];
const BUSINESS: PurchaseBusiness<'_> = PurchaseBusiness {
    name: "Synthetic & Co.",
    address: "Fixture <Lane>",
    mobile: "00000 00000",
};
const PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4,
    0x89, 0x00, 0x00, 0x00, 0x0b, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0x60, 0x00, 0x02, 0x00,
    0x00, 0x05, 0x00, 0x01, 0x7a, 0x5e, 0xab, 0x3f, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44,
    0xae, 0x42, 0x60, 0x82,
];

fn output_request(fixture: &ConfirmationFixture) -> PurchaseOutputRequest<'static> {
    PurchaseOutputRequest {
        selection: PurchaseOrderSelection {
            role: OWNER,
            id: fixture.purchase_orders(OWNER).unwrap()[0].id,
            number: 42,
        },
        business: BUSINESS,
        products: PRODUCTS,
    }
}

#[derive(Debug, Eq, PartialEq)]
enum Event {
    Render,
    Probe(ImageFile),
    Share(ImageFile, String, String),
    Download(ImageFile),
    Chat(String),
    Text(TextHandoffIntent),
}

#[derive(Default)]
struct Capture {
    png: Option<&'static [u8]>,
    supported: bool,
    rejected: bool,
    events: Vec<Event>,
    renders: Vec<(String, u32)>,
}

impl ImageHandoff for Capture {
    fn can_share(&mut self, file: &ImageFile) -> bool {
        self.events.push(Event::Probe(file.clone()));
        self.supported
    }

    fn share(&mut self, file: &ImageFile, title: &str, text: &str) -> Result<(), ShareRejected> {
        self.events
            .push(Event::Share(file.clone(), title.into(), text.into()));
        if self.rejected {
            Err(ShareRejected)
        } else {
            Ok(())
        }
    }

    fn download(&mut self, file: &ImageFile) {
        self.events.push(Event::Download(file.clone()));
    }

    fn chat(&mut self, phone: &str) {
        self.events.push(Event::Chat(phone.into()));
    }
}

impl PurchaseOrderHandoff for Capture {
    fn render_png(&mut self, html: &str, width: u32) -> Option<Vec<u8>> {
        self.events.push(Event::Render);
        self.renders.push((html.into(), width));
        self.png.map(<[u8]>::to_vec)
    }

    fn text(&mut self, intent: TextHandoffIntent) {
        self.events.push(Event::Text(intent));
    }
}

fn capture_outputs(fixture: &ConfirmationFixture) -> (PopupDocument, String) {
    let output = output_request(fixture);
    let mut printed = Vec::new();
    let mut handoff = Capture::default();
    fixture
        .print_purchase_order(output, &mut |doc| printed.push(doc))
        .unwrap();
    fixture
        .handoff_purchase_order(
            PurchaseWhatsAppRequest {
                output,
                contacts: &[],
            },
            &mut handoff,
        )
        .unwrap();
    let [print]: [_; 1] = printed.try_into().unwrap();
    let [(html, width)]: [_; 1] = handoff.renders.try_into().unwrap();
    assert_eq!(width, 760);
    (print, html)
}

#[test]
fn l07_03_view_progress_differs_from_the_shared_supplier_html() {
    let (fixture, _) = fixture_with_prior_receipt();
    let before = fixture.clone();
    let view = fixture
        .view_purchase_order(output_request(&fixture).selection, PRODUCTS)
        .unwrap();
    let (print, html) = capture_outputs(&fixture);

    assert_eq!(view.number, "PO-042");
    assert_eq!(view.supplier, "Synthetic Grove");
    assert_eq!(view.date, BillDate::new(2025, 1, 2).unwrap());
    assert_eq!(view.status, PurchaseOrderStatus::PartiallyReceived);
    assert!(view.status.can_receive());
    let [line] = view.lines.as_slice() else {
        panic!("expected one progress row")
    };
    assert_eq!(line.item, "Synthetic Orange");
    assert_eq!(line.pack.as_deref(), Some("12 bottles"));
    assert_eq!(line.mrp, Some(Money::from_minor_units(6_750)));
    assert_eq!(
        (line.ordered, line.received, line.cancelled, line.remaining),
        (4, 1, 0, 3)
    );

    assert_eq!(print.target, "_p");
    assert_eq!(
        print.intent,
        PrintIntent::AfterLoad(Duration::from_millis(350))
    );
    let (head, body) = print.html.split_once(r#"<body style="margin:0">"#).unwrap();
    assert!(head.contains("<title>Order-PO-042-Synthetic Grove</title>"));
    assert_eq!(body.strip_suffix("</body></html>"), Some(html.as_str()));
    for included in [
        "Synthetic &amp; Co.",
        "Fixture &lt;Lane&gt;",
        "Mob: 00000 00000",
        "PURCHASE ORDER",
        "PO-042",
        "02-Jan-2025",
        "Synthetic Grove",
        "Synthetic Orange",
        "24 tins &amp; jars",
        "₹89.5",
        "<b>4</b>",
        "4 cases",
        "Note: Prior PO",
        "Authorised Signatory",
    ] {
        assert!(html.contains(included), "missing {included:?}");
    }
    for excluded in [
        "Partially Received",
        "Cancelled",
        "Remaining",
        "06-Jan-2025",
        "Renamed Orange",
        "₹67.5",
        "₹3",
        "₹12",
    ] {
        assert!(!html.contains(excluded), "leaked {excluded:?}");
    }
    assert_eq!(fixture, before);
}

#[test]
fn l07_03_private_cost_changes_preserve_both_full_html_payloads() {
    let (fixture, _) = fixture_with_prior_receipt();
    let mut changed = fixture.clone();
    let line = &mut changed.purchases.orders[0].items[0];
    line.line.rate = Money::from_minor_units(2_222);
    let before = changed.clone();

    assert_eq!(capture_outputs(&fixture), capture_outputs(&changed));
    assert_eq!(changed, before);
}

#[test]
fn l07_03_status_and_remaining_follow_receipt_and_cancellation_priority() {
    let (fixture, _) = fixture_with_prior_receipt();
    for (received, cancelled, order_cancelled, remaining, status, can_receive) in [
        (0, 0, false, 4, PurchaseOrderStatus::Pending, true),
        (0, 2, false, 2, PurchaseOrderStatus::PartiallyReceived, true),
        (3, 1, false, 0, PurchaseOrderStatus::Completed, false),
        (5, 1, false, 0, PurchaseOrderStatus::Completed, false),
        (4, 0, true, 0, PurchaseOrderStatus::Cancelled, false),
        (1, 0, true, 3, PurchaseOrderStatus::Cancelled, false),
    ] {
        let mut seeded = fixture.clone();
        let order = &mut seeded.purchases.orders[0];
        order.cancelled = order_cancelled;
        order.items[0].received = received;
        order.items[0].cancelled = cancelled;
        let before = seeded.clone();
        let view = seeded
            .view_purchase_order(output_request(&seeded).selection, &[])
            .unwrap();
        let line = &view.lines[0];
        assert_eq!((line.cancelled, line.remaining), (cancelled, remaining));
        assert_eq!((line.pack.as_deref(), line.mrp), (None, None));
        assert_eq!(
            (view.status, view.status.can_receive()),
            (status, can_receive)
        );
        assert_eq!(seeded, before);
    }
}

#[test]
fn l07_03_manager_and_missing_order_do_not_reach_any_sink() {
    let (fixture, _) = fixture_with_prior_receipt();
    let before = fixture.clone();
    let valid = output_request(&fixture);
    let missing = PurchaseOrderId(usize::MAX);
    for (selection, error) in [
        (
            PurchaseOrderSelection {
                role: Role::Manager,
                ..valid.selection
            },
            PurchaseError::AccessDenied,
        ),
        (
            PurchaseOrderSelection {
                role: Role::Manager,
                id: missing,
                ..valid.selection
            },
            PurchaseError::AccessDenied,
        ),
        (
            PurchaseOrderSelection {
                role: OWNER,
                id: missing,
                ..valid.selection
            },
            PurchaseError::OrderNotFound,
        ),
    ] {
        let output = PurchaseOutputRequest { selection, ..valid };
        let mut capture = Capture::default();
        assert_eq!(fixture.view_purchase_order(selection, PRODUCTS), Err(error));
        assert_eq!(
            fixture.print_purchase_order(output, &mut |_| panic!("unexpected print")),
            Err(error)
        );
        assert_eq!(
            fixture.handoff_purchase_order(
                PurchaseWhatsAppRequest {
                    output,
                    contacts: &[]
                },
                &mut capture
            ),
            Err(error)
        );
        assert!(capture.events.is_empty());
    }
    assert_eq!(fixture, before);
}

mod whatsapp;
