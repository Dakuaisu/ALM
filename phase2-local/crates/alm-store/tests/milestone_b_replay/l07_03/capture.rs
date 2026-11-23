use std::path::PathBuf;
use std::time::Duration;

use alm_store::confirmation::{
    ConfirmationFixture, ImageFile, ImageHandoff, PrintIntent, PurchaseBusiness,
    PurchaseOrderHandoff, PurchaseOrderSelection, PurchaseOutputRequest, PurchaseWhatsAppRequest,
    ShareRejected, TextHandoffIntent,
};

use super::{PRODUCTS, artifact};

pub(super) const EXPECTED_HTML: &str = include_str!("expected_supplier.html");
pub(super) const ROUTING_ONLY: &[u8] = b"routing-only, not a PNG";
const BUSINESS: PurchaseBusiness<'_> = PurchaseBusiness {
    name: "Synthetic & Co.",
    address: "Fixture <Lane>",
    mobile: "00000 00000",
};

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Event {
    Render(u32),
    Probe(ImageFile),
    Share(ImageFile, String, String),
    Download(ImageFile),
    Chat(String),
    Text(TextHandoffIntent),
}

pub(super) enum RenderMode {
    Routing,
    Missing,
    Artifact,
}

pub(super) struct Capture {
    pub mode: RenderMode,
    pub supported: bool,
    pub rejected: bool,
    pub events: Vec<Event>,
    pub html: Vec<String>,
    pub artifact_dir: Option<PathBuf>,
}

impl Capture {
    pub fn new(mode: RenderMode, supported: bool, rejected: bool) -> Self {
        Self {
            mode,
            supported,
            rejected,
            events: Vec::new(),
            html: Vec::new(),
            artifact_dir: None,
        }
    }
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
        self.events.push(Event::Render(width));
        self.html.push(html.into());
        match self.mode {
            RenderMode::Routing => Some(ROUTING_ONLY.to_vec()),
            RenderMode::Missing => None,
            RenderMode::Artifact => {
                let (dir, png) = artifact::render(html, width);
                self.artifact_dir = Some(dir);
                Some(png)
            }
        }
    }

    fn text(&mut self, intent: TextHandoffIntent) {
        self.events.push(Event::Text(intent));
    }
}

pub(super) fn request(selection: PurchaseOrderSelection) -> PurchaseOutputRequest<'static> {
    PurchaseOutputRequest {
        selection,
        business: BUSINESS,
        products: PRODUCTS,
    }
}

pub(super) struct Output {
    pub html: String,
    pub artifact_dir: Option<PathBuf>,
    pub artifact_trace: Option<String>,
}

pub(super) fn capture_outputs(
    fixture: &ConfirmationFixture,
    selection: PurchaseOrderSelection,
    artifact_enabled: bool,
) -> Output {
    let output = request(selection);
    let mut printed = Vec::new();
    fixture
        .print_purchase_order(output, &mut |doc| printed.push(doc))
        .unwrap();
    let [print] = printed.as_slice() else {
        panic!("expected one captured print intent")
    };
    assert_eq!(print.target, "_p");
    assert_eq!(
        print.intent,
        PrintIntent::AfterLoad(Duration::from_millis(350))
    );
    let mode = if artifact_enabled && std::env::var_os("ALM_L07_03_PNG").is_some() {
        assert_eq!(std::env::var_os("ALM_L07_03_PNG").unwrap(), "1");
        RenderMode::Artifact
    } else {
        RenderMode::Missing
    };
    let mut capture = Capture::new(mode, true, false);
    fixture
        .handoff_purchase_order(
            PurchaseWhatsAppRequest {
                output,
                contacts: &[],
            },
            &mut capture,
        )
        .unwrap();
    let [html] = capture.html.as_slice() else {
        panic!("expected one PNG render input")
    };
    assert_eq!(html, EXPECTED_HTML.trim_end_matches('\n'));
    let (head, body) = print.html.split_once("<body style=\"margin:0\">").unwrap();
    assert!(head.contains("<title>Order-PO-001-Synthetic Winning Supplier</title>"));
    assert_eq!(body.strip_suffix("</body></html>"), Some(html.as_str()));
    for forbidden in [
        "₹1<",
        "₹1.8<",
        "₹28.2",
        "Received",
        "Cancelled",
        "Remaining",
        "Renamed Synthetic A",
        "₹71.75",
    ] {
        assert!(
            !html.contains(forbidden),
            "unexpected supplier output: {forbidden}"
        );
    }
    let artifact_trace = if let Some(dir) = &capture.artifact_dir {
        std::fs::write(dir.join("print-intent.html"), &print.html).unwrap();
        let [
            Event::Render(760),
            Event::Probe(probe),
            Event::Share(file, title, text),
        ] = capture.events.as_slice()
        else {
            panic!("actual PNG must follow the native-share capture branch")
        };
        assert_eq!(
            (probe.name.as_str(), file.name.as_str(), file.mime),
            ("o.png", "Order-PO-001.png", "image/png")
        );
        assert_eq!(
            (&probe.bytes, title.as_str(), text.as_str()),
            (
                &file.bytes,
                "Order PO-001",
                "Synthetic & Co. — order PO-001"
            )
        );
        assert_ne!(file.bytes, ROUTING_ONLY);
        Some(format!(
            "actual decoded Quick Look PNG: Render(760) -> Probe(o.png) -> Share(Order-PO-001.png), {} bytes; no chat",
            file.bytes.len()
        ))
    } else {
        None
    };
    Output {
        html: html.clone(),
        artifact_dir: capture.artifact_dir,
        artifact_trace,
    }
}
