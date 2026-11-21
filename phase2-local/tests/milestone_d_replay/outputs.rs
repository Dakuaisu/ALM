use std::path::Path;
use std::time::Duration;

use alm_phase2_local::confirmation::{
    BalanceMode, ConfirmationError, ConfirmationFixture, EstimateDisplay, EstimateError,
    EstimateLineDisplay, EstimateRequest, PrintIntent,
};
use alm_phase2_local::{BillDate, DraftId, Money, ProductId, Role};

#[path = "outputs/handoff.rs"]
mod handoff;
#[path = "outputs/layout.rs"]
mod layout;
#[path = "outputs/raster.rs"]
mod raster;
#[path = "outputs/source.rs"]
mod source;

const SALE: DraftId = DraftId::new(21);
const LABELS: [EstimateLineDisplay<'static>; 2] = [
    EstimateLineDisplay {
        product: ProductId::new(1),
        label: "Synthetic A",
    },
    EstimateLineDisplay {
        product: ProductId::new(2),
        label: "Synthetic B",
    },
];

pub(super) fn exercise(fixture: &ConfirmationFixture, dir: &Path) {
    let request = EstimateRequest {
        role: Role::Owner,
        draft_id: SALE,
        balance_mode: BalanceMode::Latest,
        display: EstimateDisplay {
            today: BillDate::new(2025, 3, 1).unwrap(),
            lines: &LABELS,
        },
    };
    let denied = EstimateRequest {
        role: Role::Manager,
        ..request
    };
    let denial = Err(EstimateError::Preview(ConfirmationError::AccessDenied));
    let mut documents = Vec::new();
    assert_eq!(
        fixture.print_estimate(denied, &mut |doc| documents.push(doc)),
        denial
    );
    assert_eq!(
        fixture.save_estimate_as_pdf(denied, &mut |doc| documents.push(doc)),
        denial
    );
    assert!(documents.is_empty());
    assert_eq!(
        fixture.bill_preview(Role::Manager, SALE, BalanceMode::Latest),
        Err(ConfirmationError::AccessDenied)
    );

    fixture
        .print_estimate(request, &mut |doc| documents.push(doc))
        .unwrap();
    fixture
        .save_estimate_as_pdf(request, &mut |doc| documents.push(doc))
        .unwrap();
    let [print, pdf] = documents.as_slice() else {
        panic!("expected exactly one print and one PDF intent");
    };
    assert_eq!(
        (print.target, print.intent),
        ("_print", PrintIntent::OnLoad)
    );
    assert_eq!(
        (pdf.target, pdf.intent),
        ("_pdf", PrintIntent::AfterLoad(Duration::from_millis(350)))
    );
    assert_eq!(print.html, pdf.html);

    let preview = fixture
        .bill_preview(Role::Owner, SALE, BalanceMode::Latest)
        .unwrap();
    assert_eq!(preview.invoice, 1);
    assert_eq!(preview.date, BillDate::new(2025, 2, 21).unwrap());
    assert_eq!(preview.customer, "Synthetic Buyer");
    assert!(preview.cancelled);
    assert_eq!(preview.items_quantity, 3);
    assert_eq!(preview.items_total, Money::from_minor_units(41_000));
    assert_eq!(preview.loading, Money::from_minor_units(0));
    assert_eq!(preview.total, Money::from_minor_units(41_000));
    assert_eq!(preview.paid, Money::from_minor_units(10_000));
    assert_eq!(preview.balance, Money::from_minor_units(-5_000));
    assert_eq!(preview.lines.len(), LABELS.len());
    for (line, (label, quantity, price, amount)) in preview
        .lines
        .iter()
        .zip([(LABELS[0], 2, 17_700, 35_400), (LABELS[1], 1, 5_600, 5_600)])
    {
        assert_eq!(line.product, label.product);
        assert_eq!(line.quantity.get(), quantity);
        assert_eq!(line.price, Money::from_minor_units(price));
        assert_eq!(line.amount, Money::from_minor_units(amount));
    }

    let svg = layout::render(&preview, &LABELS, request.display.today);
    source::compare(&print.html, &svg);
    let mut alternate_dates = preview.clone();
    alternate_dates.date = BillDate::new(2024, 12, 31).unwrap();
    let alternate_svg = layout::render(
        &alternate_dates,
        &LABELS,
        BillDate::new(2025, 1, 2).unwrap(),
    );
    assert!(alternate_svg.contains("Date: 31-Dec-2024</text>"));
    assert!(alternate_svg.contains("Total Due (as on 02-Jan-2025)</text>"));
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(dir.join("synthetic-estimate-print.html"), &print.html).unwrap();
    std::fs::write(dir.join("synthetic-cancelled-estimate.svg"), svg).unwrap();
    let raster = raster::render_if_requested(dir);
    handoff::exercise(fixture, request, raster.as_deref());
    let raster_note = if raster.is_some() {
        "An opted-in, cropped Quick Look PNG of the synthetic SVG was used in the inert handoff.\n"
    } else {
        "Raster blocked/not requested in the normal tool-independent replay; handoff used a 1x1 PNG sentinel.\n"
    };
    std::fs::write(
        dir.join("output-replay-evidence.txt"),
        format!(
            concat!(
                "Synthetic reconstruction from the local cancelled invoice projection.\n",
                "The SVG is an actual vector image, not a legacy PNG or a pixel-parity result.\n",
                "The adjacent HTML is the actual inert Rust print/PDF projection. It lacks the ",
                "legacy business header, simplified table columns, visual styling and footer; ",
                "its cancellation marker also follows Bill To rather than the legacy header.\n",
                "The HTML and SVG use fixed two-decimal amounts; legacy fmtN uses en-IN ",
                "grouping and up to two fractional digits. Rounding parity is not established.\n",
                "{}",
                "Real sharing, printing and PDF creation were not performed.\n"
            ),
            raster_note,
        ),
    )
    .unwrap();
}
