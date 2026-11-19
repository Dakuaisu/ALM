use super::*;
use crate::confirmation::{ProductTerms, ProposedPayment, SaleUnit, pending};
use crate::{DraftCustomer, DraftCustomerId};

#[test]
fn l03_03_legacy_confirmation_rejects_a_draft_without_a_customer_or_date() {
    let mut fixture = authority();
    let incomplete = ProvisionalDraft::new(
        DraftId::new(43),
        vec![DraftLine::new(ProductId::new(1), NonZeroU32::MIN)],
    );
    fixture.register_draft(incomplete).unwrap();
    let review = fixture.review(Role::Owner, DraftId::new(43)).unwrap();

    assert_eq!(
        fixture.confirm(Role::Owner, RequestId::new(1), &review),
        Err(ConfirmationError::MissingCustomerName)
    );
    assert_clean(&fixture);
    assert!(fixture.checkout_approvals.is_empty());
}

#[test]
fn l03_03_decoded_named_snapshot_cannot_bypass_missing_date_at_confirmation() {
    let mut fixture = authority();
    let id = DraftId::new(43);
    let order = fixture
        .review(Role::Owner, id)
        .unwrap()
        .draft
        .with_bill_date(None);
    fixture.register_draft(order).unwrap();
    let review =
        pending::decode("3|43|2|2|150,150|2,637573746F6D6572,53796E746865746963|1,12;1,2|100")
            .unwrap();
    let request = RequestId::new(1);
    assert_eq!(
        fixture.confirm(Role::Manager, request, &review),
        Err(ConfirmationError::AccessDenied)
    );
    assert_eq!(
        fixture.confirm(Role::Owner, request, &review),
        Err(ConfirmationError::MissingBillDate)
    );
    assert_clean(&fixture);
    assert!(fixture.checkout_approvals.is_empty());
}

#[test]
fn l03_03_named_review_checks_authorization_and_bill_fields_before_payment() {
    use ConfirmationError::{
        AccessDenied, MissingBillDate, MissingBillableItems, MissingCustomerName,
    };
    let mut fixture = authority();
    let id = DraftId::new(43);
    let complete = fixture.review(Role::Owner, id).unwrap().draft;
    let invalid_received = Money::from_minor_units(-1);
    for (items, name, date, reason) in [
        (false, false, false, MissingBillableItems),
        (false, true, true, MissingBillableItems),
        (true, false, false, MissingCustomerName),
        (true, false, true, MissingCustomerName),
        (true, true, false, MissingBillDate),
    ] {
        let lines = if items {
            complete.lines().to_vec()
        } else {
            Vec::new()
        };
        let customer = if name {
            complete.customer().clone()
        } else {
            DraftCustomer::Unassigned
        };
        let bill_date = if date { complete.bill_date() } else { None };
        let incomplete = ProvisionalDraft::new(id, lines)
            .with_customer(customer)
            .with_bill_date(bill_date);
        fixture.register_draft(incomplete).unwrap();

        let denied = fixture.review_named(Role::Manager, id, invalid_received);
        let rejected = fixture.review_named(Role::Owner, id, invalid_received);

        assert_eq!(denied, Err(AccessDenied));
        assert_eq!(rejected, Err(reason));
        assert_clean(&fixture);
        assert!(fixture.checkout_approvals.is_empty());
    }
}

fn authority() -> ConfirmationFixture {
    let mut fixture = ConfirmationFixture::with_ordered_lots(
        ProductId::new(1),
        &[(20, Money::from_minor_units(20))],
        Money::from_minor_units(150),
    )
    .unwrap();
    fixture
        .set_checkout_product(
            ProductId::new(1),
            ProductTerms {
                pack: NonZeroU32::new(6).unwrap(),
                piece_price: Money::from_minor_units(17),
                case_price: Money::from_minor_units(150),
            },
        )
        .unwrap();
    fixture
        .set_loading_rate(Money::from_minor_units(13))
        .unwrap();
    fixture
        .register_draft(
            ProvisionalDraft::new(
                DraftId::new(43),
                vec![
                    DraftLine::new(ProductId::new(1), NonZeroU32::new(12).unwrap()),
                    DraftLine::new(ProductId::new(1), NonZeroU32::new(2).unwrap()),
                ],
            )
            .with_customer(DraftCustomer::Selected {
                id: DraftCustomerId::new("customer".into()),
                name: "Synthetic".into(),
            })
            .with_bill_date(test_bill_date()),
        )
        .unwrap();
    fixture
}

fn issue(fixture: &mut ConfirmationFixture, received: i64) -> Review {
    fixture
        .review_checkout(
            Role::Owner,
            DraftId::new(43),
            &[SaleUnit::Piece, SaleUnit::Case],
            Money::from_minor_units(received),
        )
        .unwrap()
}

fn assert_clean(fixture: &ConfirmationFixture) {
    assert_eq!(fixture.stock(), 20);
    assert_eq!(fixture.catalog.lot_snapshot(ProductId::new(1)), vec![20]);
    assert!(fixture.catalog.allocation_snapshot().is_empty());
    assert_eq!(fixture.catalog.allocated_cost(DraftId::new(43)), None);
    assert!(fixture.customer_balances.is_empty());
    assert_eq!(
        (
            fixture.ledger().minor_units(),
            fixture.received_ledger().minor_units(),
            fixture.customer_due("customer").minor_units(),
            fixture.next_invoice,
            fixture.requests.len(),
            fixture.posted.len()
        ),
        (0, 0, 0, 1, 0, 0)
    );
}

#[test]
fn altered_decoded_unit_cannot_post_before_approval() {
    let mut fixture = authority();
    let review = issue(&mut fixture, 100);
    let mut altered = review.clone();
    altered.checkout.as_mut().unwrap().lines[0].unit = SaleUnit::Case;
    let submitted = pending::decode(&pending::encode(&altered)).unwrap();
    assert_eq!(
        fixture.confirm(Role::Owner, RequestId::new(1), &submitted),
        Err(ConfirmationError::StaleReview)
    );
    assert_clean(&fixture);
}

#[test]
fn altered_decoded_payment_cannot_post_before_approval() {
    let mut fixture = authority();
    let mut altered = issue(&mut fixture, 100);
    altered.payment = ProposedPayment::Named(Money::from_minor_units(101));
    let submitted = pending::decode(&pending::encode(&altered)).unwrap();
    assert_eq!(
        fixture.confirm(Role::Owner, RequestId::new(1), &submitted),
        Err(ConfirmationError::StaleReview)
    );
    assert_clean(&fixture);
}

#[test]
fn legacy_downgrade_cannot_bypass_checkout_approval() {
    let mut fixture = authority();
    let mut altered = issue(&mut fixture, 100);
    altered.payment = ProposedPayment::Legacy;
    assert!(pending::decode(&pending::encode(&altered)).is_err());
    altered.checkout = None;
    let submitted = pending::decode(&pending::encode(&altered)).unwrap();
    assert_eq!(
        fixture.confirm(Role::Owner, RequestId::new(1), &submitted),
        Err(ConfirmationError::StaleReview)
    );
    assert_clean(&fixture);
}

#[test]
fn rejected_review_does_not_replace_valid_approval() {
    let mut fixture = authority();
    let approved = issue(&mut fixture, 100);
    assert_eq!(
        fixture.review_checkout(
            Role::Manager,
            DraftId::new(43),
            &[SaleUnit::Piece, SaleUnit::Case],
            Money::from_minor_units(101)
        ),
        Err(ConfirmationError::AccessDenied)
    );
    assert_eq!(
        fixture.review_checkout(
            Role::Owner,
            DraftId::new(43),
            &[SaleUnit::Piece, SaleUnit::Case],
            Money::from_minor_units(10_000)
        ),
        Err(ConfirmationError::InvalidAmount)
    );
    assert!(
        fixture
            .confirm(Role::Owner, RequestId::new(1), &approved)
            .is_ok()
    );
}

#[test]
fn latest_checkout_review_replaces_previous_approval_and_replay_is_exact() {
    let mut fixture = authority();
    let old = issue(&mut fixture, 100);
    let latest = issue(&mut fixture, 101);
    assert_eq!(
        fixture.confirm(Role::Owner, RequestId::new(1), &old),
        Err(ConfirmationError::StaleReview)
    );
    assert_clean(&fixture);
    let receipt = fixture
        .confirm(Role::Owner, RequestId::new(1), &latest)
        .unwrap();
    assert_eq!(
        fixture.confirm(Role::Owner, RequestId::new(1), &latest),
        Ok(receipt)
    );
    assert_eq!(
        fixture.confirm(Role::Owner, RequestId::new(1), &old),
        Err(ConfirmationError::RequestConflict)
    );
}
