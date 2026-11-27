use std::num::NonZeroU32;

use alm_core::confirmation::{
    BalanceMode, ConfirmationError, ConfirmationFixture, EstimateDisplay, EstimateLineDisplay,
    EstimateRequest, ImageFile, ImageHandoff, ImageHandoffRequest, InvoiceHandoff,
    InvoiceHandoffRequest, InvoiceImageAttempts, ShareRejected, StockLotsError, SupplierPriceError,
    TextHandoffIntent,
};
use alm_core::draft_entry::{ActiveBillForm, BillEntryState};
use alm_core::{AccessDenied, DraftCustomer, DraftLine, LocalSales, Money};

use super::{A, B, ITEM_A, ITEM_B, MANAGER, OWNER, SALE};

pub(super) fn exercise(fixture: &ConfirmationFixture) {
    let denied = ConfirmationError::AccessDenied;
    let (saved, _) = fixture.current_sale(OWNER, SALE).unwrap();
    let latest = BalanceMode::Latest;
    let owner_bill = fixture.bill_preview(OWNER, SALE, latest).unwrap();
    assert!(owner_bill.cancelled);
    assert_eq!(owner_bill.balance.minor_units(), -5_000);

    // Reopen only the provisional input in a separate, file-free staff session.
    // The posted receipt/review is not a Manager preview.
    let profit_control = Money::from_minor_units(987_654);
    let mut drafts = LocalSales::new(profit_control);
    assert_eq!(drafts.session(OWNER).total_profit(), Ok(profit_control));
    let mut manager = drafts.session(MANAGER);
    manager.save_draft(saved.draft().clone()).unwrap();
    let reopened = manager.reopen_draft(SALE).unwrap().expect("staff draft");
    let form = ActiveBillForm::reopen(&reopened);
    let mut state = BillEntryState::default();
    state.open(&form);
    let preview = state.active_draft().expect("opened staff draft");
    assert_eq!(preview.id(), SALE);
    assert_eq!(preview.bill_date(), Some(owner_bill.date));
    assert_eq!(
        preview.lines(),
        [
            DraftLine::new(A, NonZeroU32::new(2).unwrap()),
            DraftLine::new(B, NonZeroU32::MIN),
        ]
    );
    let DraftCustomer::Selected { id, name } = preview.customer() else {
        panic!("expected the selected synthetic buyer in the staff draft");
    };
    assert_eq!(
        (id.value(), name.as_str()),
        ("milestone-c-buyer", "Synthetic Buyer")
    );
    assert!(preview.crate_rows().is_empty());
    assert_eq!(state.preview(), None);

    let readback = format!("{preview:?}").to_ascii_lowercase();
    for marker in [
        "cost",
        "profit",
        "fifo",
        "quote",
        "balance",
        "synthetic supplier",
        "987654",
        "27000",
        "6000",
        "7000",
        "2500",
        "-5000",
        "26000",
        "31000",
    ] {
        assert!(!readback.contains(marker), "staff draft leaked {marker:?}");
    }
    assert_eq!(manager.total_profit(), Err(AccessDenied));
    // DEC-034: a Manager opens and previews the posted bill; profit, the ledger, lots and
    // supplier prices stay the Owner's.
    assert_eq!(
        fixture.current_sale(MANAGER, SALE),
        fixture.current_sale(OWNER, SALE)
    );
    assert_eq!(
        fixture.bill_preview(MANAGER, SALE, latest),
        fixture.bill_preview(OWNER, SALE, latest)
    );
    assert_eq!(fixture.gross_profit(MANAGER, SALE), Err(denied));
    assert_eq!(fixture.customer_ledger(MANAGER, id), Err(denied));
    assert_eq!(
        fixture.open_stock_lots(MANAGER, A),
        Err(StockLotsError::AccessDenied)
    );
    assert_eq!(
        fixture.compare_supplier_prices(MANAGER, Some(A)),
        Err(SupplierPriceError::AccessDenied)
    );

    let display = EstimateDisplay {
        today: owner_bill.date,
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
    };
    let request = EstimateRequest {
        role: MANAGER,
        draft_id: SALE,
        balance_mode: latest,
        display,
    };
    // DEC-034: a Manager prints, saves and shares the bill; none of it carries a cost,
    // profit, FIFO, quote or supplier figure.
    let mut documents = Vec::new();
    assert_eq!(
        fixture.print_estimate(request, &mut |doc| documents.push(doc)),
        Ok(())
    );
    assert_eq!(
        fixture.save_estimate_as_pdf(request, &mut |doc| documents.push(doc)),
        Ok(())
    );
    assert_eq!(documents.len(), 2);
    for doc in &documents {
        let html = doc.html.to_ascii_lowercase();
        for marker in [
            "cost",
            "profit",
            "fifo",
            "quote",
            "synthetic supplier",
            "987654",
        ] {
            assert!(!html.contains(marker), "Manager estimate leaked {marker:?}");
        }
    }

    let image = ImageHandoffRequest {
        role: MANAGER,
        draft_id: SALE,
        business_name: "Synthetic ALM",
        saved_customer_mobile: None,
    };
    let mut sinks = RecordedHandoff::default();
    assert_eq!(
        fixture.handoff_invoice_image(image, b"inert", &mut sinks),
        Ok(())
    );
    assert_eq!(
        fixture.handoff_invoice(
            InvoiceHandoffRequest {
                image,
                tagline: "",
                balance_mode: latest,
                display,
            },
            InvoiceImageAttempts {
                html: || -> Option<&[u8]> { None },
                canvas: || -> Option<&[u8]> { None },
            },
            &mut sinks,
        ),
        Ok(())
    );
    assert!(!sinks.0.is_empty());
    for event in &sinks.0 {
        assert!(!event.to_ascii_lowercase().contains("cost"), "{event}");
    }
}

/// Every share event, as text.
#[derive(Default)]
struct RecordedHandoff(Vec<String>);

impl ImageHandoff for RecordedHandoff {
    fn can_share(&mut self, file: &ImageFile) -> bool {
        self.0.push(format!("probe {}", file.name));
        false
    }

    fn share(&mut self, file: &ImageFile, title: &str, text: &str) -> Result<(), ShareRejected> {
        self.0.push(format!("share {} {title} {text}", file.name));
        Ok(())
    }

    fn download(&mut self, file: &ImageFile) {
        self.0.push(format!("download {}", file.name));
    }

    fn chat(&mut self, phone: &str) {
        self.0.push(format!("chat {phone}"));
    }
}

impl InvoiceHandoff for RecordedHandoff {
    fn text(&mut self, intent: TextHandoffIntent) {
        self.0.push(format!("text {}", intent.url));
    }
}
