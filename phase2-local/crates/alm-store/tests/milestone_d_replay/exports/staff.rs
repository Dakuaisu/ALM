use std::num::NonZeroU32;

use alm_core::confirmation::{
    BalanceMode, ConfirmationError, ConfirmationFixture, EstimateDisplay, EstimateError,
    EstimateLineDisplay, EstimateRequest, ImageFile, ImageHandoff, ImageHandoffRequest,
    InvoiceHandoff, InvoiceHandoffRequest, InvoiceImageAttempts, ShareRejected, StockLotsError,
    SupplierPriceError, TextHandoffIntent,
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
    assert_eq!(preview.crate_row(), None);
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
    assert_eq!(fixture.current_sale(MANAGER, SALE), Err(denied));
    assert_eq!(fixture.bill_preview(MANAGER, SALE, latest), Err(denied));
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
    let mut popup = |_| panic!("Manager estimate invoked its popup sink");
    let denied_output = Err(EstimateError::Preview(denied));
    assert_eq!(fixture.print_estimate(request, &mut popup), denied_output);
    assert_eq!(
        fixture.save_estimate_as_pdf(request, &mut popup),
        denied_output
    );

    let image = ImageHandoffRequest {
        role: MANAGER,
        draft_id: SALE,
        business_name: "Synthetic ALM",
        saved_customer_mobile: None,
    };
    let mut sinks = ForbiddenHandoff;
    assert_eq!(
        fixture.handoff_invoice_image(image, b"inert", &mut sinks),
        Err(denied)
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
                html: || -> Option<&[u8]> { panic!("Manager invoked HTML renderer") },
                canvas: || -> Option<&[u8]> { panic!("Manager invoked canvas renderer") },
            },
            &mut sinks,
        ),
        denied_output
    );
}

struct ForbiddenHandoff;

impl ImageHandoff for ForbiddenHandoff {
    fn can_share(&mut self, _: &ImageFile) -> bool {
        panic!("Manager invoked share detection")
    }

    fn share(&mut self, _: &ImageFile, _: &str, _: &str) -> Result<(), ShareRejected> {
        panic!("Manager invoked share sink")
    }

    fn download(&mut self, _: &ImageFile) {
        panic!("Manager invoked download sink")
    }

    fn chat(&mut self, _: &str) {
        panic!("Manager invoked chat sink")
    }
}

impl InvoiceHandoff for ForbiddenHandoff {
    fn text(&mut self, _: TextHandoffIntent) {
        panic!("Manager invoked text sink")
    }
}
