use alm_core::confirmation::{
    CashBookDay, ConfirmationError, ConfirmationFixture, CustomerLedger, CustomerReceiptInput,
    Receipt, ReceiptMode, RequestId, Review, StockLots,
};
use alm_core::{DraftCustomer, DraftId, DraftLine, Money, ProvisionalDraft, Role};

use super::fixtures::{A_EDITED, A_INITIAL, B_SAVED, DRAFT, PRODUCTS, positive};

const OWNER: Role = Role::Owner;
const OWNER_SALE: DraftId = DraftId::new(5);

pub(super) fn seed() -> ConfirmationFixture {
    let mut fixture =
        ConfirmationFixture::with_ordered_lots(PRODUCTS[0], &[(12, money(700))], money(1_600))
            .unwrap();
    fixture.add_product(PRODUCTS[1], 11, money(2_100)).unwrap();
    let buyer = A_INITIAL.customer_id();
    let other = B_SAVED.customer_id();
    fixture.register_customer(OWNER, &other).unwrap();
    let sale = ProvisionalDraft::new(OWNER_SALE, vec![DraftLine::new(PRODUCTS[0], positive(2))])
        .with_bill_date(Some(A_INITIAL.date()))
        .with_customer(DraftCustomer::Selected {
            id: buyer,
            name: A_INITIAL.name.into(),
        });
    fixture.register_draft(sale).unwrap();
    let approved = fixture
        .review_named(OWNER, OWNER_SALE, money(1_100))
        .unwrap();
    let posted = fixture
        .confirm(OWNER, RequestId::new(71), &approved)
        .unwrap();
    assert_eq!(
        (posted.invoice, posted.total, posted.received),
        (1, money(3_200), Some(money(1_100)))
    );
    fixture
        .record_customer_receipt(
            OWNER,
            CustomerReceiptInput {
                customer_id: other,
                date: B_SAVED.date(),
                amount: money(400),
                mode: ReceiptMode::Online,
            },
        )
        .unwrap();
    let actual = Effects::read(&fixture);
    assert_eq!(actual.totals, [money(3_200), money(1_100)]);
    assert_eq!(actual.posted, 1);
    assert_eq!(actual.profit, Some(money(1_800)));
    assert_eq!(actual.lots.each_ref().map(|lot| lot.on_hand), [10, 11]);
    assert_eq!(actual.books[0].cash, money(1_100));
    assert_eq!(actual.books[2].online, money(400));
    fixture
}

#[derive(Debug, Eq, PartialEq)]
pub(super) struct Effects {
    totals: [Money; 2],
    posted: usize,
    sale: (Review, Receipt),
    profit: Option<Money>,
    lots: [StockLots; 2],
    accounts: [CustomerLedger; 2],
    books: [CashBookDay; 3],
}

impl Effects {
    pub fn read(fixture: &ConfirmationFixture) -> Self {
        assert_eq!(
            fixture.current_sale(OWNER, DRAFT),
            Err(ConfirmationError::DraftNotFound)
        );
        let (review, receipt) = fixture.current_sale(OWNER, OWNER_SALE).unwrap();
        Self {
            totals: [fixture.ledger(), fixture.received_ledger()],
            posted: fixture.posted_count(),
            sale: (review.clone(), receipt.clone()),
            profit: fixture.gross_profit(OWNER, OWNER_SALE).unwrap(),
            lots: PRODUCTS.map(|id| fixture.open_stock_lots(OWNER, id).unwrap()),
            accounts: [&A_INITIAL, &B_SAVED]
                .map(|spec| fixture.customer_ledger(OWNER, &spec.customer_id()).unwrap()),
            books: [&A_INITIAL, &A_EDITED, &B_SAVED]
                .map(|spec| fixture.cashbook_day_close(OWNER, spec.date()).unwrap()),
        }
    }
}

const fn money(value: i64) -> Money {
    Money::from_minor_units(value)
}
