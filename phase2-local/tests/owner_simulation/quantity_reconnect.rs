use std::num::{NonZeroU32, NonZeroUsize};

use alm_phase2_local::confirmation::{
    CashBookDay, CashBookEntry, CashBookKind, CustomerLedger, CustomerLedgerEntry,
    CustomerLedgerKind, ProposedPayment, ReceiptMode, SalesDetail, SalesHistory, SalesPeriod,
    SalesSource,
};

use super::{
    BillDate, CUSTOMER, CUSTOMER_ID, ConfirmationError, ConfirmationFixture, DRAFT, DraftCustomer,
    DraftCustomerId, DraftLine, LocalSales, Money, PRODUCT, PostingEffects, ProvisionalDraft,
    RECEIVED, Receipt, RequestId, Review, Role, ScratchDir, StockLotRow, StockLots, ZERO,
};
use alm_phase2_local::SqliteSales;

const OWNER: Role = Role::Owner;
const MANAGER: Role = Role::Manager;
const ACCOUNT: &str = "synthetic-manager-quantity-reconnect";
const PRICE: Money = money(11_800);
const TOTAL: Money = money(47_200);
const DUE: Money = money(27_200);

#[test]
fn persisted_quantity_edit_invalidates_retained_owner_review_on_reconnect() {
    let scratch = ScratchDir::new();
    let open = || LocalSales::open_manager_account(&scratch.0, ACCOUNT).unwrap();
    let date = BillDate::new(2025, 2, 21).unwrap();
    let buyer = DraftCustomerId::new(CUSTOMER_ID.into());
    let original = ProvisionalDraft::new(DRAFT, vec![DraftLine::new(PRODUCT, qty(3))])
        .with_customer(DraftCustomer::Selected {
            id: buyer.clone(),
            name: CUSTOMER.into(),
        })
        .with_bill_date(Some(date));
    open().save_draft(original.clone()).unwrap();
    let old = open().reopen_draft(DRAFT).unwrap().expect("saved original");
    assert_eq!(old, original);

    let mut authority = ConfirmationFixture::with_ordered_lots(
        PRODUCT,
        &[(2, money(6_000)), (5, money(7_000))],
        PRICE,
    )
    .unwrap();
    authority.register_customer(OWNER, &buyer).unwrap();
    let before = Readback::observe(&authority, &buyer, date);
    assert_eq!(before.effects.lots.on_hand, 7);
    assert_eq!(before.effects.lots.total_value, Some(money(47_000)));
    assert_eq!(before.effects.posted_count, 0);
    assert_eq!(before.effects.sales_ledger, ZERO);
    assert_eq!(before.effects.received_ledger, ZERO);
    assert_eq!(before.effects.customer_due, ZERO);
    assert_eq!(before.sale, Err(ConfirmationError::DraftNotFound));
    assert!(before.cashbook.entries.is_empty());
    assert!(before.ledger.entries.is_empty());
    assert!(before.history.rows.is_empty());
    assert_eq!(
        authority
            .reconnect_review(OWNER, old.clone())
            .unwrap()
            .draft(),
        &old
    );
    let retained = authority.review_named(OWNER, DRAFT, RECEIVED).unwrap();
    assert_eq!(retained.draft(), &old);
    assert_review(&retained, 3, money(35_400));
    assert_eq!(Readback::observe(&authority, &buyer, date), before);

    let mut edited = old;
    assert_eq!(
        edited.replace_line_quantity(NonZeroUsize::MIN, qty(4)),
        Some(qty(3))
    );
    open().save_draft(edited.clone()).unwrap();
    let reopened = open()
        .reopen_draft(DRAFT)
        .unwrap()
        .expect("saved quantity edit");
    assert_eq!(reopened, edited);
    assert_eq!(reopened.lines(), [DraftLine::new(PRODUCT, qty(4))]);
    assert_eq!(reopened.customer(), original.customer());
    assert_eq!(reopened.bill_date(), original.bill_date());
    assert_eq!(reopened.crate_row(), original.crate_row());

    assert_eq!(
        authority.reconnect_review(MANAGER, reopened.clone()),
        Err(ConfirmationError::AccessDenied)
    );
    assert_eq!(
        authority.review_named(OWNER, DRAFT, RECEIVED),
        Ok(retained.clone())
    );
    assert_eq!(Readback::observe(&authority, &buyer, date), before);

    let refreshed = authority.reconnect_review(OWNER, reopened).unwrap();
    assert_eq!(refreshed.draft(), &edited);
    assert_eq!(refreshed.amounts().unwrap().total, TOTAL);
    assert_eq!(Readback::observe(&authority, &buyer, date), before);
    let request = RequestId::new(1);
    assert_eq!(
        authority.confirm(OWNER, request, &retained),
        Err(ConfirmationError::StaleReview)
    );
    assert_eq!(Readback::observe(&authority, &buyer, date), before);

    let fresh = authority.review_named(OWNER, DRAFT, RECEIVED).unwrap();
    assert_eq!(fresh.draft(), refreshed.draft());
    assert_review(&fresh, 4, TOTAL);
    assert_eq!(Readback::observe(&authority, &buyer, date), before);
    assert_eq!(
        authority.confirm(MANAGER, request, &fresh),
        Err(ConfirmationError::AccessDenied)
    );
    assert_eq!(Readback::observe(&authority, &buyer, date), before);

    let posted = authority.confirm(OWNER, request, &fresh).unwrap();
    assert_eq!(
        posted,
        Receipt {
            draft_id: DRAFT,
            invoice: 1,
            quantity: 4,
            total: TOTAL,
            received: Some(RECEIVED),
            due: Some(DUE),
            customer_balance: Some(DUE),
        }
    );
    let after = Readback::observe(&authority, &buyer, date);
    assert_eq!(after.sale, Ok((fresh, posted.clone())));
    assert_eq!(
        after.effects,
        PostingEffects {
            lots: StockLots {
                on_hand: 3,
                rows: vec![StockLotRow {
                    date: None,
                    remaining: qty(3),
                    unit_cost: Some(money(7_000)),
                    value: Some(money(21_000)),
                }],
                total_value: Some(money(21_000)),
            },
            posted_count: 1,
            sales_ledger: TOTAL,
            received_ledger: RECEIVED,
            customer_due: DUE,
            gross_profit: Ok(Some(money(21_200))),
        }
    );
    assert_eq!(
        after.ledger,
        CustomerLedger {
            entries: vec![CustomerLedgerEntry {
                date,
                kind: CustomerLedgerKind::Bill {
                    draft_id: DRAFT,
                    invoice: 1
                },
                debit: TOTAL,
                credit: RECEIVED,
                balance: DUE,
            }],
            closing: DUE,
        }
    );
    assert_eq!(
        after.cashbook.entries,
        [CashBookEntry {
            kind: CashBookKind::Bill {
                draft_id: DRAFT,
                invoice: 1
            },
            mode: ReceiptMode::Cash,
            amount: RECEIVED,
        }]
    );
    let [row] = after.history.rows.as_slice() else {
        panic!("expected one posted bill")
    };
    assert_eq!((&row.bill, row.date, row.cancelled), (&posted, date, false));
    assert_eq!(
        after.history.collected.details,
        [SalesDetail {
            source: SalesSource::Bill(DRAFT),
            amount: RECEIVED,
            mode: ReceiptMode::Cash,
        }]
    );
    assert_eq!(open().reopen_draft(DRAFT).unwrap(), Some(edited));
    println!(
        "Scoped quantity reconnect | quantity 3 -> 4, old total=35400, stale review and Manager denied; posted={posted:?}; remaining={}",
        after.effects.lots.on_hand
    );
}

fn assert_review(review: &Review, packets: u32, total: Money) {
    let amounts = review.amounts().unwrap();
    assert_eq!((amounts.packets, amounts.total), (packets, total));
    assert_eq!(review.unit_prices(), [PRICE]);
    assert_eq!(review.payment(), ProposedPayment::Named(RECEIVED));
}

#[derive(Debug, Eq, PartialEq)]
struct Readback {
    effects: PostingEffects,
    sale: Result<(Review, Receipt), ConfirmationError>,
    cashbook: CashBookDay,
    ledger: CustomerLedger,
    history: SalesHistory,
}

impl Readback {
    fn observe(fixture: &ConfirmationFixture, buyer: &DraftCustomerId, date: BillDate) -> Self {
        Self {
            effects: PostingEffects::observe(fixture),
            sale: fixture
                .current_sale(OWNER, DRAFT)
                .map(|(review, receipt)| (review.clone(), receipt.clone())),
            cashbook: fixture.cashbook_day_close(OWNER, date).unwrap(),
            ledger: fixture.customer_ledger(OWNER, buyer).unwrap(),
            history: fixture
                .sales_history(OWNER, date, SalesPeriod::All, "")
                .unwrap(),
        }
    }
}

const fn qty(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).expect("positive fixture quantity")
}

const fn money(value: i64) -> Money {
    Money::from_minor_units(value)
}
