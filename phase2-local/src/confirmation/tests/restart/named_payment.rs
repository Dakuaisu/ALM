use super::*;
use crate::{DraftCustomer, DraftCustomerId};

#[test]
fn named_payment_survives_sqlite_process_restart_and_lost_response() {
    let scratch = Scratch::new();
    let dir = &scratch.0;
    let database = dir.join(DATABASE);
    let order = draft(43, 2).with_customer(DraftCustomer::Selected {
        id: DraftCustomerId::new("customer-a".into()),
        name: "Synthetic".into(),
    });
    let mut authority = ConfirmationFixture::with_ordered_lots(
        ProductId::new(1),
        &[(3, Money::from_minor_units(80))],
        Money::from_minor_units(125),
    )
    .unwrap();
    let mut sales = open_sales(&database);
    sales
        .session(Role::Manager)
        .save_draft(order.clone())
        .unwrap();
    assert_eq!(
        sales
            .session(Role::Manager)
            .reopen_draft(DraftId::new(43))
            .unwrap(),
        Some(order.clone())
    );
    authority.reconnect_review(Role::Owner, order).unwrap();
    let approved = OwnerRequest {
        id: RequestId::new(987),
        review: authority
            .review_named(Role::Owner, DraftId::new(43), Money::from_minor_units(100))
            .unwrap(),
    };
    assert_eq!(
        (
            authority.ledger().minor_units(),
            authority.received_ledger().minor_units(),
            authority.customer_due("customer-a").minor_units()
        ),
        (0, 0, 0)
    );
    let mut sent_by_manager = false;
    assert!(
        sales
            .session(Role::Manager)
            .confirm_pending(approved.id, &approved.review, |_, _| sent_by_manager = true)
            .is_err()
    );
    assert!(!sent_by_manager);
    let mut first = Client::start("sqlite_submit", dir);
    first.write_frame(&approved.encode());
    let sent = first.sent();
    assert_eq!(sent, approved);
    let receipt = authority
        .confirm(Role::Owner, sent.id, &sent.review)
        .unwrap();
    assert_eq!(
        (
            receipt.total.minor_units(),
            receipt.received.unwrap().minor_units(),
            receipt.due.unwrap().minor_units(),
            receipt.customer_balance.unwrap().minor_units()
        ),
        (250, 100, 150, 150)
    );
    assert_eq!(receipt.invoice, 1);
    first.write_frame("CONTINUE");
    assert_eq!(first.finish(), ["REPLY_EOF"]);
    assert_eq!(Client::start("sqlite_manager", dir).finish(), ["DENIED"]);
    let changed = authority
        .review_named(Role::Owner, DraftId::new(43), Money::from_minor_units(101))
        .unwrap();
    assert_eq!(
        authority.confirm(Role::Owner, approved.id, &changed),
        Err(ConfirmationError::RequestConflict)
    );
    assert_eq!(
        authority.confirm(Role::Manager, approved.id, &approved.review),
        Err(ConfirmationError::AccessDenied)
    );
    let mut retry = Client::start("sqlite_retry", dir);
    let resent = retry.sent();
    assert_eq!(resent, approved);
    let replay = authority
        .confirm(Role::Owner, resent.id, &resent.review)
        .unwrap();
    assert_eq!(replay, receipt);
    retry.write_frame(&receipt_frame(&replay));
    assert_eq!(
        retry.finish(),
        [format!("RECEIVED {}", receipt_frame(&receipt))]
    );
    assert_eq!(
        (
            authority.stock(),
            authority.ledger().minor_units(),
            authority.received_ledger().minor_units(),
            authority.customer_due("customer-a").minor_units(),
            authority.posted_count(),
            authority.next_invoice
        ),
        (1, 250, 100, 150, 1, 2)
    );
    assert_eq!(
        authority.requests.get(&approved.id),
        Some(&(approved.review.clone(), receipt.clone()))
    );
    assert_eq!(authority.posted.get(&DraftId::new(43)), Some(&approved.id));
    assert_eq!(
        authority.catalog.allocated_cost(DraftId::new(43)),
        Some(Some(160))
    );
    assert_eq!(
        authority.catalog.allocation_snapshot(),
        vec![(43, 0, 0, 2, Some(160))]
    );
}
