use alm_core::confirmation::{
    ConfirmationFixture, ImageFile, PurchaseError, PurchaseOrderSelection, PurchaseSupplierContact,
    PurchaseWhatsAppRequest, TextHandoffIntent,
};

use super::capture::{Capture, Event, ROUTING_ONLY, RenderMode, request};
use super::{MANAGER, OWNER, PRODUCTS};

fn file(name: &str) -> ImageFile {
    ImageFile {
        name: name.into(),
        mime: "image/png",
        bytes: ROUTING_ONLY.to_vec(),
    }
}

pub(super) fn exercise(
    fixture: &ConfirmationFixture,
    selection: PurchaseOrderSelection,
    html: &str,
) -> Vec<String> {
    let suppliers = fixture.purchase_suppliers(OWNER).unwrap();
    let [other, saved, third] = suppliers else {
        panic!("expected synthetic supplier directory")
    };
    assert_eq!(
        fixture.purchase_orders(OWNER).unwrap()[0].supplier_id,
        saved.id
    );
    let named = PurchaseSupplierContact {
        id: other.id,
        name: "Synthetic Winning Supplier",
        mobile: "00000 00002",
    };
    let later = PurchaseSupplierContact {
        id: third.id,
        name: named.name,
        mobile: "00000 00003",
    };
    let by_id = PurchaseSupplierContact {
        id: saved.id,
        name: named.name,
        mobile: "+91 (00000) 00001",
    };
    let stale = PurchaseSupplierContact {
        name: "Synthetic Other",
        ..by_id
    };
    let bad = PurchaseSupplierContact {
        mobile: "123",
        ..named
    };
    let bad_id = PurchaseSupplierContact {
        mobile: "123",
        ..by_id
    };
    let wrong_case = PurchaseSupplierContact {
        name: "synthetic winning supplier",
        ..named
    };
    let mut traces = Vec::new();

    for (label, contacts, phone) in [
        (
            "ID beats earlier same-name",
            [named, by_id].as_slice(),
            Some("910000000001"),
        ),
        (
            "stale ID uses first exact name",
            [stale, named, later].as_slice(),
            Some("910000000002"),
        ),
        (
            "missing ID uses first exact name",
            [named, later].as_slice(),
            Some("910000000002"),
        ),
        (
            "no exact name falls back to ID",
            [wrong_case, stale].as_slice(),
            Some("910000000001"),
        ),
        (
            "invalid chosen name does not retry",
            [stale, bad, later].as_slice(),
            None,
        ),
        (
            "invalid chosen ID does not retry",
            [named, bad_id].as_slice(),
            None,
        ),
        (
            "case mismatch with missing ID",
            [wrong_case].as_slice(),
            None,
        ),
        ("no contacts", [].as_slice(), None),
    ] {
        let mut capture = Capture::new(RenderMode::Routing, false, false);
        fixture
            .handoff_purchase_order(
                PurchaseWhatsAppRequest {
                    output: request(selection),
                    contacts,
                },
                &mut capture,
            )
            .unwrap();
        let mut expected = vec![
            Event::Render(760),
            Event::Probe(file("o.png")),
            Event::Download(file("Order-PO-001.png")),
        ];
        expected.extend(phone.map(|number| Event::Chat(number.into())));
        assert_eq!(capture.events, expected, "{label}");
        assert_eq!(capture.html, [html]);
        traces.push(format!(
            "{label}: {:?} [routing-only bytes]",
            capture.events
        ));
    }

    for rejected in [false, true] {
        let mut capture = Capture::new(RenderMode::Routing, true, rejected);
        fixture
            .handoff_purchase_order(
                PurchaseWhatsAppRequest {
                    output: request(selection),
                    contacts: &[by_id],
                },
                &mut capture,
            )
            .unwrap();
        assert_eq!(
            capture.events,
            [
                Event::Render(760),
                Event::Probe(file("o.png")),
                Event::Share(
                    file("Order-PO-001.png"),
                    "Order PO-001".into(),
                    "Synthetic & Co. — order PO-001".into()
                ),
            ]
        );
        assert_eq!(capture.html, [html]);
        traces.push(format!(
            "native share rejected={rejected}: {:?} [routing-only bytes]",
            capture.events
        ));
    }

    for (contacts, phone) in [([by_id].as_slice(), "910000000001"), ([].as_slice(), "")] {
        let mut capture = Capture::new(RenderMode::Missing, true, false);
        fixture
            .handoff_purchase_order(
                PurchaseWhatsAppRequest {
                    output: request(selection),
                    contacts,
                },
                &mut capture,
            )
            .unwrap();
        let text = concat!(
            "*Order%20%E2%80%94%20Synthetic%20%26%20Co.*%20(10-Feb-2025)%0A%0A",
            "Synthetic%20A%20%E2%80%94%2012%20cases%0A",
            "Synthetic%20B%20%E2%80%94%209%20cases%0A"
        );
        assert_eq!(
            capture.events,
            [
                Event::Render(760),
                Event::Text(TextHandoffIntent {
                    url: format!("https://wa.me/{phone}?text={text}"),
                    target: "_blank"
                })
            ]
        );
        assert_eq!(capture.html, [html]);
        traces.push(format!(
            "render unavailable phone={phone:?}: {:?}",
            capture.events
        ));
    }

    let manager = PurchaseOrderSelection {
        role: MANAGER,
        ..selection
    };
    let mut capture = Capture::new(RenderMode::Routing, true, false);
    assert_eq!(
        fixture.view_purchase_order(manager, PRODUCTS),
        Err(PurchaseError::AccessDenied)
    );
    assert_eq!(
        fixture.print_purchase_order(request(manager), &mut |_| panic!(
            "manager reached print sink"
        )),
        Err(PurchaseError::AccessDenied)
    );
    assert_eq!(
        fixture.handoff_purchase_order(
            PurchaseWhatsAppRequest {
                output: request(manager),
                contacts: &[by_id]
            },
            &mut capture
        ),
        Err(PurchaseError::AccessDenied)
    );
    assert!(capture.events.is_empty());
    traces.push("manager: View, print intent, and handoff denied before sinks".into());
    traces
}
