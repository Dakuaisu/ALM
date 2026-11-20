use std::num::NonZeroU32;

use alm_phase2_local::confirmation::{ConfirmationError, ConfirmationFixture, Receipt, RequestId};
use alm_phase2_local::{
    BillDate, DraftCustomer, DraftCustomerId, DraftId, DraftLine, Money, ProductId,
    ProvisionalDraft, Role,
};

const OWNER: Role = Role::Owner;
const DRAFT: DraftId = DraftId::new(11);
const REQUEST: RequestId = RequestId::new(21);
const PRODUCT: ProductId = ProductId::new(31);
const CUSTOMER: &str = "l11_01-synthetic-customer";
const STOCK: u32 = 3;
const SELL: Money = Money::from_minor_units(100);
const RECEIVED: Money = Money::from_minor_units(125);

fn draft(second_quantity: u32) -> ProvisionalDraft {
    ProvisionalDraft::new(
        DRAFT,
        vec![
            DraftLine::new(PRODUCT, NonZeroU32::MIN),
            DraftLine::new(
                PRODUCT,
                NonZeroU32::new(second_quantity).expect("positive synthetic quantity"),
            ),
        ],
    )
    .with_customer(DraftCustomer::Selected {
        id: DraftCustomerId::new(CUSTOMER.into()),
        name: "Synthetic Customer".into(),
    })
    .with_bill_date(Some(
        BillDate::new(2025, 1, 1).expect("valid synthetic date"),
    ))
}

#[test]
fn l11_01_local_posting_rejects_shortage_atomically() {
    // Given three available units, each line fits alone but their combined demand is four.
    // The local operational posting path is in memory; no SQLite or remote authority is used.
    let mut posting = ConfirmationFixture::new(PRODUCT, STOCK, SELL).expect("synthetic catalog");
    let original = draft(STOCK);
    posting
        .register_draft(original.clone())
        .expect("register synthetic draft");
    let review = posting
        .review_named(OWNER, DRAFT, RECEIVED)
        .expect("review the over-stock draft with a payment");
    let demand = review.amounts().expect("reviewed amounts").packets;
    assert_eq!(demand, 4);

    // When the Owner tries to post both lines.
    let rejected = posting.confirm(OWNER, REQUEST, &review);

    // Then no stock, financial, posted sale, request, or draft state changes.
    assert_eq!(rejected, Err(ConfirmationError::InsufficientStock));
    let effects = (
        posting.stock(),
        posting.ledger().minor_units(),
        posting.received_ledger().minor_units(),
        posting.customer_due(CUSTOMER).minor_units(),
        posting.posted_count(),
    );
    assert_eq!(effects, (STOCK, 0, 0, 0, 0));
    let posted_items = posting.posted_sale_items(OWNER, DRAFT);
    assert_eq!(posted_items, Err(ConfirmationError::DraftNotFound));
    let retained = posting
        .review_named(OWNER, DRAFT, RECEIVED)
        .expect("review the retained draft");
    assert_eq!(retained, review);
    let draft_unchanged = retained.draft() == &original;
    assert!(draft_unchanged);
    println!(
        "l11_01 local rejection: demand={demand} result={rejected:?} \
         (stock, sales ledger, received ledger, customer due, posted requests)={effects:?} \
         posted_items={posted_items:?} draft_unchanged={draft_unchanged}"
    );

    // Recovery: correct the same draft to exactly available stock and reuse the request.
    posting
        .register_draft(draft(STOCK - 1))
        .expect("register corrected draft with the same ID");
    let corrected = posting
        .review_named(OWNER, DRAFT, RECEIVED)
        .expect("review corrected draft");
    let receipt = posting
        .confirm(OWNER, REQUEST, &corrected)
        .expect("post corrected draft with the same request ID");
    assert_eq!(
        receipt,
        Receipt {
            draft_id: DRAFT,
            invoice: 1,
            quantity: STOCK,
            total: Money::from_minor_units(300),
            received: Some(RECEIVED),
            due: Some(Money::from_minor_units(175)),
            customer_balance: Some(Money::from_minor_units(175)),
        },
    );
    assert_eq!(posting.stock(), 0);
    assert_eq!(posting.ledger(), receipt.total);
    assert_eq!(posting.received_ledger(), RECEIVED);
    assert_eq!(
        Some(posting.customer_due(CUSTOMER)),
        receipt.customer_balance
    );
    assert_eq!(posting.posted_count(), 1);
    println!(
        "l11_01 local recovery: reused request={REQUEST:?} receipt={receipt:?} stock={} posted={}",
        posting.stock(),
        posting.posted_count(),
    );
}
