use std::cell::Cell;

use alm_core::Role;
use alm_core::confirmation::{
    ConfirmationFixture, EstimateRequest, ImageFile, ImageHandoff, ImageHandoffRequest,
    InvoiceHandoff, InvoiceHandoffRequest, InvoiceImageAttempts, ShareRejected, TextHandoffIntent,
};

// A valid transparent PNG sentinel tests routing only; it is not the reconstructed bill image.
const PNG_SENTINEL: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4,
    0x89, 0x00, 0x00, 0x00, 0x0b, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0x60, 0x00, 0x02, 0x00,
    0x00, 0x05, 0x00, 0x01, 0x7a, 0x5e, 0xab, 0x3f, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44,
    0xae, 0x42, 0x60, 0x82,
];

#[derive(Debug, Eq, PartialEq)]
enum Event {
    Probe(ImageFile),
    Share(ImageFile, String, String),
    Download(ImageFile),
    Chat(String),
    Text(TextHandoffIntent),
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

impl InvoiceHandoff for Capture {
    fn text(&mut self, intent: TextHandoffIntent) {
        self.events.push(Event::Text(intent));
    }
}

pub(super) fn exercise(
    fixture: &ConfirmationFixture,
    estimate: EstimateRequest<'_>,
    rendered_png: Option<&[u8]>,
) {
    let png = rendered_png.unwrap_or(PNG_SENTINEL);
    let image = ImageHandoffRequest {
        role: Role::Owner,
        draft_id: estimate.draft_id,
        business_name: "Synthetic Business",
        saved_customer_mobile: Some(("milestone-c-buyer", "+91 (00000) 00000")),
    };
    let file = ImageFile {
        name: "Estimate-1.png".into(),
        mime: "image/png",
        bytes: png.into(),
    };
    for share_result in [Ok(()), Err(ShareRejected)] {
        let mut capture = Capture {
            supported: true,
            share_result,
            events: Vec::new(),
        };
        fixture
            .handoff_invoice_image(image, png, &mut capture)
            .unwrap();
        assert_eq!(
            capture.events,
            [
                Event::Probe(file.clone()),
                Event::Share(
                    file.clone(),
                    "Estimate #1".into(),
                    "Synthetic Business — Estimate #1".into(),
                ),
            ]
        );
    }
    let mut capture = Capture {
        supported: false,
        share_result: Ok(()),
        events: Vec::new(),
    };
    fixture
        .handoff_invoice_image(image, png, &mut capture)
        .unwrap();
    assert_eq!(
        capture.events,
        [
            Event::Probe(file.clone()),
            Event::Download(file.clone()),
            Event::Chat("910000000000".into()),
        ]
    );

    let request = InvoiceHandoffRequest {
        image,
        tagline: "Synthetic only",
        balance_mode: estimate.balance_mode,
        display: estimate.display,
    };
    let attempts = Cell::new(0);
    // DEC-034: a Manager's handoff of the same bill emits what the Owner's does.
    let owner_events = std::mem::take(&mut capture.events);
    assert_eq!(
        fixture.handoff_invoice_image(
            ImageHandoffRequest {
                role: Role::Manager,
                ..image
            },
            png,
            &mut capture,
        ),
        Ok(())
    );
    assert_eq!(std::mem::take(&mut capture.events), owner_events);
    assert_eq!(
        fixture.handoff_invoice(
            InvoiceHandoffRequest {
                image: ImageHandoffRequest {
                    role: Role::Manager,
                    ..image
                },
                ..request
            },
            InvoiceImageAttempts {
                html: || {
                    attempts.set(attempts.get() + 1);
                    Some(png)
                },
                canvas: || {
                    attempts.set(attempts.get() + 1);
                    Some(png)
                },
            },
            &mut capture,
        ),
        Ok(())
    );
    assert_eq!(attempts.replace(0), 1, "the first rendering is used");
    assert_eq!(std::mem::take(&mut capture.events), owner_events);

    fixture
        .handoff_invoice(
            request,
            InvoiceImageAttempts {
                html: || {
                    assert_eq!(attempts.replace(1), 0);
                    None
                },
                canvas: || {
                    assert_eq!(attempts.replace(2), 1);
                    Some(png)
                },
            },
            &mut capture,
        )
        .unwrap();
    assert_eq!(attempts.get(), 2);
    assert_eq!(
        capture.events,
        [
            Event::Probe(file.clone()),
            Event::Download(file),
            Event::Chat("910000000000".into()),
        ]
    );
    capture.events.clear();
    fixture
        .handoff_invoice(
            request,
            InvoiceImageAttempts {
                html: || None,
                canvas: || None,
            },
            &mut capture,
        )
        .unwrap();
    let [Event::Text(intent)] = capture.events.as_slice() else {
        panic!("failed image attempts must capture only a text intent");
    };
    assert_eq!(intent.target, "_blank");
    assert_eq!(
        intent.url,
        concat!(
            "https://wa.me/910000000000?text=*Synthetic%20Business*%0A",
            "Synthetic%20only%0A----------------------%0A",
            "*Estimate%20%231*%20%20(21-Feb-2025)%0A",
            "Party%3A%20Synthetic%20Buyer%0A%0A",
            "Synthetic%20A%20%20%C3%972%20%3D%20%E2%82%B9354%0A",
            "Synthetic%20B%20%20%C3%971%20%3D%20%E2%82%B956%0A",
            "Loading%2FDelivery%3A%20%E2%82%B90%0A----------------------%0A",
            "*Bill%20Amount%3A%20%E2%82%B9410*%0A",
            "Paid%20on%20this%20bill%3A%20%E2%82%B9100%0A",
            "*Total%20Due%20(as%20on%2001-Mar-2025)%3A%20%E2%82%B9-50*",
            "%0A%0AThank%20you!%20%F0%9F%99%8F",
        )
    );
}
