//! L14-01/L14-04, PHASE1_CASES.md:487. Legacy HTML is read as source only.
use std::num::NonZeroU32;

use alm_phase2_local::confirmation::{
    BalanceMode, ConfirmationError, ConfirmationFixture, CreationKey, CustomerLedgerKind,
    CustomerReceiptInput, EstimateDisplay, EstimateLineDisplay, EstimateRequest, OpeningBalance,
    OpeningLedgerKind, ReceiptMode, RequestId,
};
use alm_phase2_local::receive_payment::{Customer, ReceivePayment, SaveError};
use alm_phase2_local::{
    BillDate, DraftCustomer, DraftCustomerId, DraftId, DraftLine, Money, ProductId,
    ProvisionalDraft, Role,
};

#[path = "l14_opening_balance_replay/ledger_keyed.rs"]
mod keyed;
#[path = "l14_opening_balance_replay/ledger_mixed.rs"]
mod mixed;
#[path = "l14_opening_balance_replay/opening_actual.rs"]
mod opening_actual;
#[path = "l14_opening_balance_replay/ledger_source.rs"]
mod source;
#[path = "l14_opening_balance_replay/ledger_ties.rs"]
mod ties;

const FEB9: &str = "2025-02-09";
const FEB10: &str = "2025-02-10";

fn rust_fixture() -> (ConfirmationFixture, DraftCustomerId, DraftId, DraftId) {
    // Given: a zero-start public fixture, without opening data or caller-set timestamps.
    let feb9: BillDate = FEB9.parse().unwrap();
    let feb10: BillDate = FEB10.parse().unwrap();
    let money = Money::from_minor_units;
    let customer_id = DraftCustomerId::new("l14-customer".into());
    let product = ProductId::new(1);
    let b1 = DraftId::new(20);
    let b2 = DraftId::new(10); // Inverted draft IDs distinguish key order from invoice order.
    let mut fixture = ConfirmationFixture::new(product, 3, money(10_000)).unwrap();
    let receipt = |date, amount, mode| CustomerReceiptInput {
        customer_id: customer_id.clone(),
        date,
        amount: money(amount),
        mode,
    };

    // When: post B1, R1, B2, R2, then the backdated R3 through the real APIs.
    for (request, id, paid, payment) in [
        (
            RequestId::new(101),
            b1,
            2_000,
            receipt(feb10, 3_000, ReceiptMode::Cash),
        ),
        (
            RequestId::new(102),
            b2,
            1_000,
            receipt(feb10, 400, ReceiptMode::Online),
        ),
    ] {
        let draft = ProvisionalDraft::new(id, vec![DraftLine::new(product, NonZeroU32::MIN)])
            .with_bill_date(Some(feb10))
            .with_customer(DraftCustomer::Selected {
                id: customer_id.clone(),
                name: "Synthetic customer".into(),
            });
        fixture.register_draft(draft).unwrap();
        let review = fixture.review_named(Role::Owner, id, money(paid)).unwrap();
        fixture.confirm(Role::Owner, request, &review).unwrap();
        fixture
            .record_customer_receipt(Role::Owner, payment)
            .unwrap();
    }
    fixture
        .record_customer_receipt(Role::Owner, receipt(feb9, 1_000, ReceiptMode::Cash))
        .unwrap();

    (fixture, customer_id, b1, b2)
}

#[test]
fn l14_rust_preview_uses_posting_order_when_receipt_is_backdated() {
    let (fixture, customer_id, b1, b2) = rust_fixture();
    let feb9: BillDate = FEB9.parse().unwrap();
    let feb10: BillDate = FEB10.parse().unwrap();
    let money = Money::from_minor_units;

    // Then: the ledger is document-date ordered; the preview is bounded by posting order.
    let ledger = fixture.customer_ledger(Role::Owner, &customer_id).unwrap();
    let rows: Vec<_> = ledger
        .entries
        .iter()
        .map(|entry| {
            (
                entry.date,
                &entry.kind,
                entry.debit.minor_units(),
                entry.credit.minor_units(),
                entry.balance.minor_units(),
            )
        })
        .collect();
    let bill_kind = |draft_id, invoice| CustomerLedgerKind::Bill { draft_id, invoice };
    let cash = CustomerLedgerKind::Receipt {
        mode: ReceiptMode::Cash,
    };
    let online = CustomerLedgerKind::Receipt {
        mode: ReceiptMode::Online,
    };
    assert_eq!(
        rows,
        [
            (feb9, &cash, 0, 1_000, -1_000),
            (feb10, &bill_kind(b1, 1), 10_000, 2_000, 7_000),
            (feb10, &cash, 0, 3_000, 4_000),
            (feb10, &bill_kind(b2, 2), 10_000, 1_000, 13_000),
            (feb10, &online, 0, 400, 12_600),
        ]
    );
    assert_eq!(ledger.closing, money(12_600));
    let [as_on, latest] = [BalanceMode::AsOnBillDate, BalanceMode::Latest]
        .map(|mode| fixture.bill_preview(Role::Owner, b2, mode).unwrap());
    assert_eq!(
        (as_on.invoice, as_on.date, as_on.balance_mode),
        (2, feb10, BalanceMode::AsOnBillDate)
    );
    assert_eq!(
        (latest.invoice, latest.date, latest.balance_mode),
        (2, feb10, BalanceMode::Latest)
    );
    let b2_row = rows[3].4;
    let (as_on, latest) = (as_on.balance.minor_units(), latest.balance.minor_units());
    assert_eq!((as_on, b2_row, latest), (14_000, 13_000, 12_600));
    println!(
        "L14 Rust API ledger (no opening or created timestamps; date, kind, debit p, credit p, running p):"
    );
    for (date, kind, debit, credit, running) in &rows {
        println!("  {date} {kind:?} {debit} {credit} {running}");
    }
    println!("L14 Rust API bill_preview B2 (p): as-on={as_on}, row={b2_row}, latest={latest}");
}
