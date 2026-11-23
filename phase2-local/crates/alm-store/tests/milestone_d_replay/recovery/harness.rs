use std::num::NonZeroU32;

use alm_core::confirmation::{ ConfirmationFixture, CustomerReceiptInput, EditChanges, ReceiptMode, RequestId, };
use alm_core::{BillDate, DraftCustomer, DraftCustomerId, DraftId, DraftLine, Money, ProductId, ProvisionalDraft, Role};

#[path = "../../milestone_c_replay/inventory.rs"]
mod inventory;
#[path = "../recovery.rs"]
mod recovery;

const OWNER: Role = Role::Owner;
const MANAGER: Role = Role::Manager;
const A: ProductId = ProductId::new(1);
const B: ProductId = ProductId::new(2);
const ITEM_A: &str = "Synthetic A";
const ITEM_B: &str = "Synthetic B";
const fn money(value: i64) -> Money {
    Money::from_minor_units(value)
}
const fn qty(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).unwrap()
}
fn feb(day: u8) -> BillDate {
    BillDate::new(2025, 2, day).unwrap()
}

#[test]
fn cancelled_fixture_recovers_into_fresh_read_model() {
    let mut fixture = ConfirmationFixture::new(A, 0, money(11_800)).unwrap();
    fixture.add_product(B, 0, money(5_600)).unwrap();
    inventory::receive_dated_stock(&mut fixture);
    let buyer = DraftCustomerId::new("milestone-c-buyer".into());
    let sale = DraftId::new(21);
    fixture
        .register_draft(
            ProvisionalDraft::new(sale, vec![DraftLine::new(A, qty(2))])
                .with_bill_date(Some(feb(21)))
                .with_customer(DraftCustomer::Selected {
                    id: buyer.clone(),
                    name: "Synthetic Buyer".into(),
                }),
        )
        .unwrap();
    let review = fixture.review_named(OWNER, sale, money(10_000)).unwrap();
    fixture.confirm(OWNER, RequestId::new(1), &review).unwrap();
    fixture
        .record_customer_receipt(
            OWNER,
            CustomerReceiptInput {
                customer_id: buyer,
                date: BillDate::new(2025, 3, 1).unwrap(),
                amount: money(5_000),
                mode: ReceiptMode::Online,
            },
        )
        .unwrap();
    let edit = fixture
        .begin_edit(
            OWNER,
            sale,
            EditChanges {
                retained_price: money(17_700),
                appended_line: DraftLine::new(B, qty(1)),
                appended_price: money(5_600),
            },
        )
        .unwrap();
    fixture.save_edit(OWNER, &edit).unwrap();
    fixture.cancel_sale(OWNER, sale).unwrap();
    inventory::assert_stock(&fixture, inventory::StockCheckpoint::Cancelled);
    let dir = std::env::temp_dir().join(format!("alm-d-recovery-{}", std::process::id()));
    std::fs::create_dir(&dir).unwrap();
    recovery::exercise(&fixture, &dir);
    std::fs::remove_dir_all(dir).unwrap();
}
