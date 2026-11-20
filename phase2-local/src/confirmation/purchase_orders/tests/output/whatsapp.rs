use super::{
    Capture, Event, ImageFile, PNG, PurchaseSupplierContact, PurchaseWhatsAppRequest,
    TextHandoffIntent, fixture_with_prior_receipt, output_request,
};
use crate::confirmation::purchase_orders::PurchaseSupplierId;

const ORDER_FILENAME: &str = "Order-PO-042.png";
const SAVED: PurchaseSupplierId = PurchaseSupplierId(0);
const OTHER: PurchaseSupplierId = PurchaseSupplierId(1);
const THIRD: PurchaseSupplierId = PurchaseSupplierId(2);
const NAME: &str = "Synthetic Grove";

fn png_file(name: &str) -> ImageFile {
    ImageFile {
        name: name.into(),
        mime: "image/png",
        bytes: PNG.to_vec(),
    }
}

#[test]
fn l07_03_native_share_probes_o_png_and_is_terminal_even_on_rejection() {
    let (fixture, _) = fixture_with_prior_receipt();
    let before = fixture.clone();
    let output = output_request(&fixture);
    let contacts = [PurchaseSupplierContact {
        id: SAVED,
        name: NAME,
        mobile: "00000 00001",
    }];
    for rejected in [false, true] {
        let mut capture = Capture {
            png: Some(PNG),
            supported: true,
            rejected,
            ..Capture::default()
        };
        fixture
            .handoff_purchase_order(
                PurchaseWhatsAppRequest {
                    output,
                    contacts: &contacts,
                },
                &mut capture,
            )
            .unwrap();
        assert_eq!(
            capture.events,
            [
                Event::Render,
                Event::Probe(png_file("o.png")),
                Event::Share(
                    png_file(ORDER_FILENAME),
                    "Order PO-042".into(),
                    "Synthetic & Co. — order PO-042".into()
                ),
            ]
        );
    }
    assert_eq!(fixture, before);
}

#[test]
fn l07_03_unsupported_share_downloads_then_chats_only_with_the_source_recipient() {
    let (fixture, _) = fixture_with_prior_receipt();
    let before = fixture.clone();
    let output = output_request(&fixture);
    let named_first = PurchaseSupplierContact {
        id: OTHER,
        name: NAME,
        mobile: "00000 00002",
    };
    let named_second = PurchaseSupplierContact {
        id: THIRD,
        name: NAME,
        mobile: "00000 00003",
    };
    let by_id = PurchaseSupplierContact {
        id: SAVED,
        name: NAME,
        mobile: "+91 (00000) 00001",
    };
    let stale_id = PurchaseSupplierContact {
        name: "Synthetic Other",
        ..by_id
    };
    let bad_named = PurchaseSupplierContact {
        mobile: "123",
        ..named_first
    };
    let case_mismatch = PurchaseSupplierContact {
        name: "synthetic grove",
        ..named_first
    };

    for (contacts, expected_phone) in [
        ([named_first, by_id].as_slice(), Some("910000000001")),
        (
            [stale_id, named_first, named_second].as_slice(),
            Some("910000000002"),
        ),
        ([named_first].as_slice(), Some("910000000002")),
        ([case_mismatch, stale_id].as_slice(), Some("910000000001")),
        ([stale_id, bad_named, named_second].as_slice(), None),
        (&[], None),
    ] {
        let mut capture = Capture {
            png: Some(PNG),
            ..Capture::default()
        };
        fixture
            .handoff_purchase_order(PurchaseWhatsAppRequest { output, contacts }, &mut capture)
            .unwrap();
        let mut expected = vec![
            Event::Render,
            Event::Probe(png_file("o.png")),
            Event::Download(png_file(ORDER_FILENAME)),
        ];
        expected.extend(expected_phone.map(|phone| Event::Chat(phone.into())));
        assert_eq!(capture.events, expected);
    }
    assert_eq!(fixture, before);
}

#[test]
fn l07_03_without_png_only_encoded_text_uses_the_original_ordered_cases() {
    let (fixture, _) = fixture_with_prior_receipt();
    let before = fixture.clone();
    let output = output_request(&fixture);
    let contact = PurchaseSupplierContact {
        id: SAVED,
        name: NAME,
        mobile: "00000 00001",
    };
    for (contacts, phone) in [([contact].as_slice(), "910000000001"), (&[], "")] {
        let mut capture = Capture {
            supported: true,
            ..Capture::default()
        };
        fixture
            .handoff_purchase_order(PurchaseWhatsAppRequest { output, contacts }, &mut capture)
            .unwrap();
        assert_eq!(
            capture.events,
            [
                Event::Render,
                Event::Text(TextHandoffIntent {
                    url: format!(
                        "https://wa.me/{phone}?text={}",
                        concat!(
                            "*Order%20%E2%80%94%20Synthetic%20%26%20Co.*%20(02-Jan-2025)%0A%0A",
                            "Synthetic%20Orange%20%E2%80%94%204%20cases%0A",
                        )
                    ),
                    target: "_blank",
                })
            ]
        );
    }
    assert_eq!(fixture, before);
}
