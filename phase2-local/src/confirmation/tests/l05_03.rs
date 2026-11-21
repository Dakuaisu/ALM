// L05-03: waInvoice, ALM_Business_Manager.html:4768–4805.
use super::*;
use crate::DraftCustomerId;

// A complete 1×1 transparent RGBA PNG, not a rendered estimate.
pub(super) const PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4,
    0x89, 0x00, 0x00, 0x00, 0x0b, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0x60, 0x00, 0x02, 0x00,
    0x00, 0x05, 0x00, 0x01, 0x7a, 0x5e, 0xab, 0x3f, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44,
    0xae, 0x42, 0x60, 0x82,
];
pub(super) const MOBILE: (&str, &str) = ("customer-saved", "+91 (00000) 00000");

pub(super) fn fixture() -> ConfirmationFixture {
    let mut fixture = l05_01::preview_fixture(5);
    fixture
        .register_draft(draft(41, 2).with_customer(DraftCustomer::Selected {
            id: DraftCustomerId::new(MOBILE.0.into()),
            name: "Synthetic Buyer".into(),
        }))
        .unwrap();
    let review = fixture
        .review_named(
            Role::Owner,
            DraftId::new(41),
            Money::from_minor_units(10_000),
        )
        .unwrap();
    fixture
        .confirm(Role::Owner, RequestId::new(400), &review)
        .unwrap();
    fixture.preview_payments.push(PreviewPayment {
        customer_id: MOBILE.0.into(),
        amount: Money::from_minor_units(6_000),
        date: BillDate::new(2024, 3, 1).unwrap(),
        after_invoice: 1,
        mode: None,
    });
    fixture
}

#[derive(Debug, PartialEq, Eq)]
enum Event {
    Probe(ImageFile),
    Share(ImageFile, String, String),
    Download(ImageFile),
    Chat(String),
}

struct Capture {
    supported: bool,
    share_result: Result<(), ShareRejected>,
    events: Vec<Event>,
}

impl ImageHandoff for Capture {
    fn can_share(&mut self, file: &ImageFile) -> bool {
        self.events.push(Event::Probe(file.clone()));
        self.supported
    }

    fn share(&mut self, file: &ImageFile, title: &str, text: &str) -> Result<(), ShareRejected> {
        self.events
            .push(Event::Share(file.clone(), title.into(), text.into()));
        self.share_result
    }

    fn download(&mut self, file: &ImageFile) {
        self.events.push(Event::Download(file.clone()));
    }

    fn chat(&mut self, phone: &str) {
        self.events.push(Event::Chat(phone.into()));
    }
}

fn run_case(role: Role, mobile: Option<(&str, &str)>, mut sink: Capture) -> Vec<Event> {
    let fixture = fixture();
    let before = fixture.clone();
    let result = fixture.handoff_invoice_image(
        ImageHandoffRequest {
            role,
            draft_id: DraftId::new(41),
            business_name: "Synthetic business",
            saved_customer_mobile: mobile,
        },
        PNG,
        &mut sink,
    );
    assert_eq!(
        result,
        match role {
            Role::Owner => Ok(()),
            Role::Manager => Err(ConfirmationError::AccessDenied),
        }
    );
    assert_eq!(fixture, before);
    sink.events
}

fn expected_file() -> ImageFile {
    ImageFile {
        name: "Estimate-1.png".into(),
        mime: "image/png",
        bytes: PNG.to_vec(),
    }
}

#[test]
fn l05_03_supported_share_is_terminal_even_when_rejected() {
    for share_result in [Ok(()), Err(ShareRejected)] {
        let events = run_case(
            Role::Owner,
            Some(MOBILE),
            Capture {
                supported: true,
                share_result,
                events: Vec::new(),
            },
        );
        assert_eq!(
            events,
            [
                Event::Probe(expected_file()),
                Event::Share(
                    expected_file(),
                    "Estimate #1".into(),
                    "Synthetic business — Estimate #1".into(),
                )
            ]
        );
    }
}

#[test]
fn l05_03_unsupported_downloads_then_requests_the_resolved_chat() {
    let events = run_case(
        Role::Owner,
        Some(MOBILE),
        Capture {
            supported: false,
            share_result: Ok(()),
            events: Vec::new(),
        },
    );
    assert_eq!(
        events,
        [
            Event::Probe(expected_file()),
            Event::Download(expected_file()),
            Event::Chat("910000000000".into())
        ]
    );
}

#[test]
fn l05_03_unsupported_stops_after_download_without_a_resolved_phone() {
    for mobile in [
        None,
        Some(("other-id", MOBILE.1)),
        Some((MOBILE.0, "123-456")),
    ] {
        let events = run_case(
            Role::Owner,
            mobile,
            Capture {
                supported: false,
                share_result: Ok(()),
                events: Vec::new(),
            },
        );
        assert_eq!(
            events,
            [
                Event::Probe(expected_file()),
                Event::Download(expected_file())
            ]
        );
    }
}

#[test]
fn l05_03_manager_cannot_probe_or_emit_an_action() {
    let events = run_case(
        Role::Manager,
        Some(MOBILE),
        Capture {
            supported: true,
            share_result: Ok(()),
            events: Vec::new(),
        },
    );
    assert!(events.is_empty());
}
