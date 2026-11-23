use std::num::NonZeroU32;

use alm_store::confirmation::{
    ConfirmationError, ConfirmationFixture, CustomerReceiptInput, EditChanges, Receipt,
    ReceiptMode, RequestId, StockLots,
};
use alm_store::{
    BillDate, DraftCustomer, DraftCustomerId, DraftId, DraftLine, Money, ProductId,
    ProvisionalDraft, Role,
};

#[path = "../../../../tests/milestone_c_replay/accounts.rs"]
mod accounts;
#[path = "../../../../tests/milestone_c_replay/inventory.rs"]
mod inventory;

use accounts::{AccountCheckpoint, assert_accounts};
use inventory::{StockCheckpoint, assert_stock, receive_dated_stock};

#[path = "../common/partial_po.rs"]
pub mod partial_po;

pub(super) const OWNER: Role = Role::Owner;
pub(super) const MANAGER: Role = Role::Manager;
pub(super) const DENIED: ConfirmationError = ConfirmationError::AccessDenied;
pub(super) const A: ProductId = ProductId::new(1);
pub(super) const B: ProductId = ProductId::new(2);
pub(super) const ITEM_A: &str = "Synthetic A";
pub(super) const ITEM_B: &str = "Synthetic B";
pub(super) const SALE: DraftId = DraftId::new(21);
pub(super) const BUYER: &str = "Synthetic Buyer";
pub(super) const BUYER_ID: &str = "milestone-c-buyer";
pub(super) const PAID: Money = money(10_000);

const POSTED: Receipt = Receipt {
    draft_id: SALE,
    invoice: 1,
    quantity: 2,
    total: money(23_600),
    received: Some(PAID),
    due: Some(money(13_600)),
    customer_balance: Some(money(13_600)),
};
const EDITED: Receipt = Receipt {
    quantity: 3,
    total: money(41_000),
    due: Some(money(31_000)),
    customer_balance: Some(money(26_000)),
    ..POSTED
};

pub(super) const fn money(value: i64) -> Money {
    Money::from_minor_units(value)
}

pub(super) const fn qty(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).expect("positive synthetic packet count")
}

pub(super) fn feb(day: u8) -> BillDate {
    BillDate::new(2025, 2, day).expect("valid February fixture date")
}

pub(super) fn stock(fixture: &ConfirmationFixture) -> [u32; 2] {
    [A, B].map(|id| fixture.product_stock(id).unwrap())
}

pub(super) fn fifo(fixture: &ConfirmationFixture) -> [StockLots; 2] {
    [A, B].map(|id| fixture.open_stock_lots(OWNER, id).unwrap())
}

pub(super) fn fifo_value(lots: &[StockLots; 2]) -> i64 {
    lots.iter()
        .map(|product| product.total_value.unwrap().minor_units())
        .sum()
}

pub(super) fn march_receipt_date() -> BillDate {
    accounts::march_receipt_date()
}

pub(super) fn cancelled_fixture() -> ConfirmationFixture {
    let mut fixture = ConfirmationFixture::new(A, 0, money(11_800)).unwrap();
    fixture.add_product(B, 0, money(5_600)).unwrap();
    receive_dated_stock(&mut fixture);
    assert_stock(&fixture, StockCheckpoint::Received);

    let buyer = DraftCustomerId::new(BUYER_ID.into());
    let bill = ProvisionalDraft::new(SALE, vec![DraftLine::new(A, qty(2))])
        .with_bill_date(Some(feb(21)))
        .with_customer(DraftCustomer::Selected {
            id: buyer.clone(),
            name: BUYER.into(),
        });
    fixture.register_draft(bill).unwrap();
    assert_accounts(&fixture, &buyer, AccountCheckpoint::BeforePosting);
    let review = fixture.review_named(OWNER, SALE, PAID).unwrap();
    assert_eq!(
        fixture.confirm(OWNER, RequestId::new(1), &review),
        Ok(POSTED)
    );
    assert_accounts(&fixture, &buyer, AccountCheckpoint::Posted);
    assert_stock(&fixture, StockCheckpoint::Posted);

    fixture
        .record_customer_receipt(
            OWNER,
            CustomerReceiptInput {
                customer_id: buyer.clone(),
                date: march_receipt_date(),
                amount: money(5_000),
                mode: ReceiptMode::Online,
            },
        )
        .unwrap();
    assert_accounts(&fixture, &buyer, AccountCheckpoint::OnlineReceipt);

    let changes = EditChanges {
        retained_price: money(17_700),
        appended_line: DraftLine::new(B, qty(1)),
        appended_price: money(5_600),
    };
    let edit = fixture.begin_edit(OWNER, SALE, changes).unwrap();
    assert_eq!(fixture.save_edit(OWNER, &edit), Ok(EDITED));
    assert_accounts(&fixture, &buyer, AccountCheckpoint::Edited);
    assert_stock(&fixture, StockCheckpoint::Edited);

    fixture.cancel_sale(OWNER, SALE).unwrap();
    let (saved, receipt) = fixture.current_sale(OWNER, SALE).unwrap();
    assert_eq!(receipt, &EDITED);
    assert_eq!(
        saved.draft().lines(),
        [DraftLine::new(A, qty(2)), DraftLine::new(B, qty(1))]
    );
    assert_eq!(saved.unit_prices(), [money(17_700), money(5_600)]);
    assert_eq!(fixture.posted_count(), 1);
    assert_accounts(&fixture, &buyer, AccountCheckpoint::Cancelled);
    assert_stock(&fixture, StockCheckpoint::Cancelled);
    fixture
}
