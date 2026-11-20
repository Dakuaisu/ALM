use super::*;
use crate::{BillDate, DraftCustomer, DraftLine};
use std::num::NonZeroU32;

mod bill_guard;
mod cancellation;
mod checkout_guard;
mod edit;
mod fifo;
mod l05_01;
mod l05_02;
mod l05_02_display;
mod l05_02_privacy;
mod l05_03;
mod l05_04;
mod l06_01;
mod l06_01_hsn;
mod l06_02;
mod multiline;
mod named_payment;
mod pending;
mod restart;
mod walk_in;

fn draft(id: u64, quantity: u32) -> ProvisionalDraft {
    dated_named_draft(
        DraftId::new(id),
        vec![DraftLine::new(
            ProductId::new(1),
            NonZeroU32::new(quantity).unwrap(),
        )],
    )
}

fn test_bill_date() -> Option<BillDate> {
    Some(BillDate::new(2024, 2, 29).unwrap())
}

fn dated_named_draft(id: DraftId, lines: Vec<DraftLine>) -> ProvisionalDraft {
    ProvisionalDraft::new(id, lines)
        .with_customer(DraftCustomer::Typed("Synthetic".into()))
        .with_bill_date(test_bill_date())
}

fn fixture(stock: u32) -> ConfirmationFixture {
    let mut f =
        ConfirmationFixture::new(ProductId::new(1), stock, Money::from_minor_units(125)).unwrap();
    f.register_draft(draft(1, 2)).unwrap();
    f
}

#[test]
fn successful_sale_and_lost_response_replay_have_one_effect() {
    let mut f = fixture(5);
    let review = f.review(Role::Owner, DraftId::new(1)).unwrap();
    assert_eq!(review.draft(), &draft(1, 2));
    assert_eq!(review.unit_price(), Money::from_minor_units(125));
    assert_eq!((f.stock(), f.ledger().minor_units()), (5, 0));
    let receipt = f.confirm(Role::Owner, RequestId::new(1), &review).unwrap();
    assert_eq!(
        (
            f.stock(),
            f.ledger().minor_units(),
            receipt.quantity,
            receipt.total.minor_units()
        ),
        (3, 250, 2, 250)
    );
    f.set_unit_price(Money::from_minor_units(400)).unwrap();
    assert_eq!(
        f.confirm(Role::Owner, RequestId::new(1), &review),
        Ok(receipt)
    );
    assert_eq!(
        (f.stock(), f.ledger().minor_units(), f.requests.len()),
        (3, 250, 1)
    );
    assert_eq!(
        f.confirm(Role::Owner, RequestId::new(2), &review),
        Err(ConfirmationError::DraftAlreadyPosted)
    );
}

#[test]
fn reused_request_with_different_review_conflicts() {
    let mut f = fixture(5);
    let first = f.review(Role::Owner, DraftId::new(1)).unwrap();
    f.confirm(Role::Owner, RequestId::new(1), &first).unwrap();
    f.register_draft(draft(2, 1)).unwrap();
    let second = f.review(Role::Owner, DraftId::new(2)).unwrap();
    assert_eq!(
        f.confirm(Role::Owner, RequestId::new(1), &second),
        Err(ConfirmationError::RequestConflict)
    );
}

#[test]
fn changed_draft_or_terms_rejects_stale_review() {
    let mut f = fixture(5);
    let old = f.review(Role::Owner, DraftId::new(1)).unwrap();
    f.register_draft(draft(1, 3)).unwrap();
    assert_eq!(
        f.confirm(Role::Owner, RequestId::new(1), &old),
        Err(ConfirmationError::StaleReview)
    );
    let old_terms = f.review(Role::Owner, DraftId::new(1)).unwrap();
    f.set_unit_price(Money::from_minor_units(200)).unwrap();
    assert_eq!(
        f.confirm(Role::Owner, RequestId::new(2), &old_terms),
        Err(ConfirmationError::StaleReview)
    );
    assert_eq!((f.stock(), f.ledger().minor_units()), (5, 0));
}

#[test]
fn customer_only_edit_invalidates_the_previous_review_without_posting() {
    let mut f = fixture(5);
    let original = f.review(Role::Owner, DraftId::new(1)).unwrap();
    let edited = draft(1, 2).with_customer(DraftCustomer::Typed("Synthetic customer".into()));

    let updated = f.reconnect_review(Role::Owner, edited.clone()).unwrap();
    let stale = f.confirm(Role::Owner, RequestId::new(1), &original);

    assert_eq!(updated.draft(), &edited);
    assert_eq!(stale, Err(ConfirmationError::StaleReview));
    assert_eq!(
        (
            f.stock(),
            f.ledger().minor_units(),
            f.next_invoice,
            f.posted_count()
        ),
        (5, 0, 1, 0)
    );
}

#[test]
fn manager_is_denied_before_replay() {
    let mut f = fixture(5);
    assert_eq!(
        f.review(Role::Manager, DraftId::new(1)),
        Err(ConfirmationError::AccessDenied)
    );
    let review = f.review(Role::Owner, DraftId::new(1)).unwrap();
    f.confirm(Role::Owner, RequestId::new(1), &review).unwrap();
    assert_eq!(
        f.confirm(Role::Manager, RequestId::new(1), &review),
        Err(ConfirmationError::AccessDenied)
    );
}

#[test]
fn insufficient_stock_does_not_mutate_any_effect() {
    let mut f = fixture(1);
    let review = f.review(Role::Owner, DraftId::new(1)).unwrap();
    assert_eq!(
        f.confirm(Role::Owner, RequestId::new(1), &review),
        Err(ConfirmationError::InsufficientStock)
    );
    assert_eq!(
        (
            f.stock(),
            f.ledger().minor_units(),
            f.next_invoice,
            f.requests.len(),
            f.posted.len()
        ),
        (1, 0, 1, 0, 0)
    );
}

#[test]
fn two_reviewed_drafts_compete_for_the_same_stock() {
    let mut f = fixture(2);
    f.register_draft(draft(2, 2)).unwrap();
    let first = f.review(Role::Owner, DraftId::new(1)).unwrap();
    let second = f.review(Role::Owner, DraftId::new(2)).unwrap();

    f.confirm(Role::Owner, RequestId::new(1), &first).unwrap();
    assert_eq!(
        f.confirm(Role::Owner, RequestId::new(2), &second),
        Err(ConfirmationError::InsufficientStock)
    );
    assert_eq!(
        (
            f.stock(),
            f.ledger().minor_units(),
            f.next_invoice,
            f.requests.len(),
            f.posted.len()
        ),
        (0, 250, 2, 1, 1)
    );
}

#[test]
fn total_ledger_and_invoice_overflow_are_atomic() {
    let mut f = fixture(5);
    f.set_unit_price(Money::from_minor_units(i64::MAX)).unwrap();
    let review = f.review(Role::Owner, DraftId::new(1)).unwrap();
    assert_eq!(
        f.confirm(Role::Owner, RequestId::new(1), &review),
        Err(ConfirmationError::Overflow)
    );
    f.set_unit_price(Money::from_minor_units(1)).unwrap();
    let review = f.review(Role::Owner, DraftId::new(1)).unwrap();
    f.ledger = i64::MAX;
    assert_eq!(
        f.confirm(Role::Owner, RequestId::new(1), &review),
        Err(ConfirmationError::Overflow)
    );
    f.ledger = 0;
    f.next_invoice = u64::MAX;
    assert_eq!(
        f.confirm(Role::Owner, RequestId::new(1), &review),
        Err(ConfirmationError::Overflow)
    );
    assert_eq!(
        (
            f.stock(),
            f.ledger().minor_units(),
            f.requests.len(),
            f.posted.len()
        ),
        (5, 0, 0, 0)
    );
}
