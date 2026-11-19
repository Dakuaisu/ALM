use super::*;
use crate::confirmation::{ProductTerms, SaleUnit};
use crate::{DraftCustomer, DraftCustomerId, DraftLine, ProvisionalDraft};
use std::num::NonZeroU32;

fn configured(lots: &[(u32, Money)], quantities: [u32; 2]) -> ConfirmationFixture {
    let mut authority = ConfirmationFixture::with_ordered_lots(
        ProductId::new(1),
        lots,
        Money::from_minor_units(150),
    )
    .unwrap();
    authority
        .set_checkout_product(
            ProductId::new(1),
            ProductTerms {
                pack: NonZeroU32::new(6).unwrap(),
                piece_price: Money::from_minor_units(17),
                case_price: Money::from_minor_units(150),
            },
        )
        .unwrap();
    authority
        .set_loading_rate(Money::from_minor_units(13))
        .unwrap();
    let order = ProvisionalDraft::new(
        DraftId::new(43),
        quantities
            .into_iter()
            .map(|qty| DraftLine::new(ProductId::new(1), NonZeroU32::new(qty).unwrap()))
            .collect(),
    )
    .with_customer(DraftCustomer::Selected {
        id: DraftCustomerId::new("customer-a".into()),
        name: "Synthetic".into(),
    });
    authority.register_draft(order).unwrap();
    authority
}

fn approval(authority: &mut ConfirmationFixture) -> OwnerRequest {
    OwnerRequest {
        id: RequestId::new(987),
        review: authority
            .review_checkout(
                Role::Owner,
                DraftId::new(43),
                &[SaleUnit::Piece, SaleUnit::Case],
                Money::from_minor_units(100),
            )
            .unwrap(),
    }
}

#[test]
fn mixed_checkout_survives_lost_response_and_sqlite_process_restart() {
    let scratch = Scratch::new();
    let mut authority = configured(
        &[
            (3, Money::from_minor_units(80)),
            (4, Money::from_minor_units(20)),
        ],
        [12, 2],
    );
    let approved = approval(&mut authority);
    let amounts = approved.review.amounts().unwrap();
    assert_eq!(
        (
            amounts.packets,
            amounts.subtotal.minor_units(),
            amounts.loading.minor_units(),
            amounts.total.minor_units()
        ),
        (4, 504, 52, 556)
    );
    let mut first = Client::start("sqlite_submit", &scratch.0);
    first.write_frame(&approved.encode());
    let sent = first.sent();
    assert_eq!(sent, approved);
    let receipt = authority
        .confirm(Role::Owner, sent.id, &sent.review)
        .unwrap();
    assert_eq!(
        (
            receipt.quantity,
            receipt.total.minor_units(),
            receipt.received.unwrap().minor_units(),
            receipt.due.unwrap().minor_units()
        ),
        (4, 556, 100, 456)
    );
    first.write_frame("CONTINUE");
    assert_eq!(first.finish(), ["REPLY_EOF"]);
    assert_eq!(
        authority.catalog.lot_snapshot(ProductId::new(1)),
        vec![0, 3]
    );
    assert_eq!(
        authority.catalog.allocated_cost(DraftId::new(43)),
        Some(Some(260))
    );
    authority
        .set_loading_rate(Money::from_minor_units(99))
        .unwrap();
    let mut retry = Client::start("sqlite_retry", &scratch.0);
    let resent = retry.sent();
    assert_eq!(resent, approved);
    assert_eq!(
        authority.confirm(Role::Manager, resent.id, &resent.review),
        Err(ConfirmationError::AccessDenied)
    );
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
            authority.next_invoice,
            authority.requests.len()
        ),
        (3, 556, 100, 456, 2, 1)
    );
    assert_eq!(
        authority.catalog.lot_snapshot(ProductId::new(1)),
        vec![0, 3]
    );
}

#[test]
fn late_case_shortage_rolls_back_every_effect() {
    let mut authority = configured(&[(3, Money::from_minor_units(80))], [12, 2]);
    let approved = approval(&mut authority);
    assert_eq!(
        authority.confirm(Role::Owner, approved.id, &approved.review),
        Err(ConfirmationError::InsufficientStock)
    );
    assert_eq!(authority.catalog.lot_snapshot(ProductId::new(1)), vec![3]);
    assert!(authority.catalog.allocation_snapshot().is_empty());
    assert_eq!(
        (
            authority.ledger().minor_units(),
            authority.received_ledger().minor_units(),
            authority.customer_due("customer-a").minor_units(),
            authority.next_invoice,
            authority.requests.len(),
            authority.posted.len()
        ),
        (0, 0, 0, 1, 0, 0)
    );
}

#[test]
fn fractional_packets_are_rejected_and_terms_invalidate_reviews() {
    let mut fractional = configured(&[(7, Money::from_minor_units(20))], [11, 2]);
    assert_eq!(
        fractional.review_checkout(
            Role::Owner,
            DraftId::new(43),
            &[SaleUnit::Piece, SaleUnit::Case],
            Money::from_minor_units(100)
        ),
        Err(ConfirmationError::InvalidAmount)
    );
    assert_eq!(
        fractional.review_checkout(
            Role::Manager,
            DraftId::new(43),
            &[SaleUnit::Piece, SaleUnit::Case],
            Money::from_minor_units(100)
        ),
        Err(ConfirmationError::AccessDenied)
    );
    fractional = configured(&[(7, Money::from_minor_units(20))], [12, 2]);
    let approved = approval(&mut fractional);
    fractional
        .set_loading_rate(Money::from_minor_units(14))
        .unwrap();
    assert_eq!(
        fractional.confirm(Role::Owner, approved.id, &approved.review),
        Err(ConfirmationError::StaleReview)
    );
    assert_eq!(
        (
            fractional.stock(),
            fractional.ledger().minor_units(),
            fractional.received_ledger().minor_units(),
            fractional.customer_due("customer-a").minor_units(),
            fractional.next_invoice,
            fractional.requests.len()
        ),
        (7, 0, 0, 0, 1, 0)
    );
}

#[test]
fn changed_pack_or_unit_prices_reject_the_original_review() {
    for terms in [
        ProductTerms {
            pack: NonZeroU32::new(3).unwrap(),
            piece_price: Money::from_minor_units(17),
            case_price: Money::from_minor_units(150),
        },
        ProductTerms {
            pack: NonZeroU32::new(6).unwrap(),
            piece_price: Money::from_minor_units(18),
            case_price: Money::from_minor_units(150),
        },
        ProductTerms {
            pack: NonZeroU32::new(6).unwrap(),
            piece_price: Money::from_minor_units(17),
            case_price: Money::from_minor_units(151),
        },
    ] {
        let mut authority = configured(&[(7, Money::from_minor_units(20))], [12, 2]);
        let approved = approval(&mut authority);
        authority
            .set_checkout_product(ProductId::new(1), terms)
            .unwrap();
        assert_eq!(
            authority.confirm(Role::Owner, approved.id, &approved.review),
            Err(ConfirmationError::StaleReview)
        );
        assert_eq!(
            (
                authority.stock(),
                authority.ledger().minor_units(),
                authority.received_ledger().minor_units(),
                authority.customer_due("customer-a").minor_units(),
                authority.next_invoice,
                authority.requests.len()
            ),
            (7, 0, 0, 0, 1, 0)
        );
        assert!(authority.catalog.allocation_snapshot().is_empty());
    }
}
