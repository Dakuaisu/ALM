use super::{super::pending, *};
use crate::DraftCustomerId;

pub(super) fn cash_draft(id: u64, quantity: u32) -> ProvisionalDraft {
    draft(id, quantity).with_customer(DraftCustomer::Typed("Local - Neighbor".into()))
}

#[test]
fn walk_in_cash_requires_an_exact_unlinked_label_a_date_and_owner() {
    let mut authority = fixture(5);
    let id = DraftId::new(1);
    for customer in [
        DraftCustomer::Typed("local - neighbor".into()),
        DraftCustomer::Typed("Local - Neighbor ".into()),
        DraftCustomer::Selected {
            id: DraftCustomerId::new("namesake".into()),
            name: "Local - Neighbor".into(),
        },
    ] {
        authority
            .register_draft(cash_draft(1, 2).with_customer(customer))
            .unwrap();
        assert_eq!(
            authority.review_walk_in_cash(Role::Owner, id),
            Err(ConfirmationError::InvalidDraft)
        );
    }
    authority
        .register_draft(cash_draft(1, 2).with_bill_date(None))
        .unwrap();
    assert_eq!(
        authority.review_walk_in_cash(Role::Owner, id),
        Err(ConfirmationError::MissingBillDate)
    );
    assert_eq!(
        authority.review_walk_in_cash(Role::Manager, id),
        Err(ConfirmationError::AccessDenied)
    );
    assert_eq!(authority.posted_count(), 0);
}

#[test]
fn walk_in_cash_uses_checked_total_and_cannot_be_replayed_as_legacy() {
    let mut authority = fixture(5);
    let draft = cash_draft(1, 2);
    let id = draft.id();
    authority.register_draft(draft).unwrap();
    authority
        .set_unit_price(Money::from_minor_units(i64::MAX))
        .unwrap();
    assert_eq!(
        authority.review_walk_in_cash(Role::Owner, id),
        Err(ConfirmationError::Overflow)
    );
    assert_eq!(authority.posted_count(), 0);

    authority
        .set_unit_price(Money::from_minor_units(125))
        .unwrap();
    let legacy = authority.review(Role::Owner, id).unwrap();
    assert_eq!(legacy.payment(), ProposedPayment::Legacy);
    let cash = authority.review_walk_in_cash(Role::Owner, id).unwrap();
    assert_eq!(
        cash.payment(),
        ProposedPayment::WalkInCash(Money::from_minor_units(250))
    );
    let request = RequestId::new(101);
    authority.confirm(Role::Owner, request, &cash).unwrap();
    assert_eq!(
        authority.confirm(Role::Owner, request, &legacy),
        Err(ConfirmationError::RequestConflict)
    );
    assert_eq!(authority.posted_count(), 1);
}

#[test]
fn walk_in_cash_snapshot_rejects_missing_date_non_cash_identity_and_non_full_amount() {
    let mut authority = fixture(5);
    let draft = cash_draft(1, 2);
    let id = draft.id();
    authority.register_draft(draft).unwrap();
    let cash = authority.review_walk_in_cash(Role::Owner, id).unwrap();
    let wire = pending::encode(&cash);
    assert_eq!(pending::decode(&wire).unwrap(), cash);
    let payload = wire.strip_prefix("7|250|6|2024-02-29|").unwrap();
    for malformed in [
        format!("7|250|{payload}"),
        format!("7|249|6|2024-02-29|{payload}"),
        format!("7|251|6|2024-02-29|{payload}"),
        wire.replace("|1,,", "|2,78,"),
    ] {
        assert!(matches!(
            pending::decode(&malformed),
            Err(crate::InvalidStoredText)
        ));
    }
}
