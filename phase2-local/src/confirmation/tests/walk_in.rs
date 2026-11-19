use super::{super::pending, *};
use crate::draft_entry::{
    ActiveBillForm, BillEntryState, ConfirmationAttempt, ConfirmationOutcome,
};
use crate::{DraftCustomerId, LocalSales};

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
            Err(crate::DraftStorageError::InvalidOutput)
        ));
    }
}

#[test]
fn walk_in_ack_must_match_request_and_review_and_retries_cannot_replace_an_existing_preview_on_edit()
 {
    let scratch = restart::Scratch::new();
    let mut local = LocalSales::open_sqlite(
        &scratch.0.join("correlation.sqlite3"),
        Money::from_minor_units(0),
    )
    .unwrap();
    let mut session = local.session(Role::Owner);
    let mut authority = fixture(5);
    let draft = cash_draft(1, 2);
    authority.register_draft(draft.clone()).unwrap();
    let review = authority
        .review_walk_in_cash(Role::Owner, draft.id())
        .unwrap();
    let request = RequestId::new(98);
    let submit = ConfirmationAttempt::Submit {
        request,
        review: &review,
    };
    let mut entry = BillEntryState::default();
    assert!(matches!(
        entry.confirm(&mut session, submit, |_, _| panic!(
            "no matching active draft"
        )),
        Err(PendingError::ActiveDraftMismatch)
    ));
    entry.open(&ActiveBillForm::reopen(&draft));

    let mut other_authority = fixture(5);
    other_authority.register_draft(draft.clone()).unwrap();
    let wrong_request_ack = other_authority
        .confirm_acknowledged(Role::Owner, RequestId::new(99), &review)
        .unwrap();
    let submit = ConfirmationAttempt::Submit {
        request,
        review: &review,
    };
    assert_eq!(
        entry
            .confirm(&mut session, submit, |_, _| Ok(Some(wrong_request_ack)))
            .unwrap(),
        ConfirmationOutcome::Uncorrelated
    );
    assert_eq!(
        (entry.active_draft(), entry.preview()),
        (Some(&draft), None)
    );

    let mut other_authority = fixture(5);
    other_authority.register_draft(draft.clone()).unwrap();
    other_authority
        .set_unit_price(Money::from_minor_units(126))
        .unwrap();
    let different_review = other_authority
        .review_walk_in_cash(Role::Owner, draft.id())
        .unwrap();
    let wrong_review_ack = other_authority
        .confirm_acknowledged(Role::Owner, request, &different_review)
        .unwrap();
    assert_eq!(
        entry
            .confirm(
                &mut session,
                ConfirmationAttempt::Retry { request },
                |_, _| Ok(Some(wrong_review_ack))
            )
            .unwrap(),
        ConfirmationOutcome::Uncorrelated
    );
    assert_eq!(
        (entry.active_draft(), entry.preview()),
        (Some(&draft), None)
    );

    let accepted = entry
        .confirm(
            &mut session,
            ConfirmationAttempt::Retry { request },
            |id, review| {
                authority
                    .confirm_acknowledged(Role::Owner, id, review)
                    .map(Some)
            },
        )
        .unwrap();
    let ConfirmationOutcome::Acknowledged(receipt) = accepted else {
        panic!("expected authoritative receipt")
    };
    assert_eq!(
        (entry.active_draft(), entry.preview()),
        (None, Some(&receipt))
    );

    let edited = cash_draft(1, 3);
    entry.open(&ActiveBillForm::reopen(&edited));
    let other_draft = cash_draft(2, 1);
    authority.register_draft(other_draft.clone()).unwrap();
    let other_review = authority
        .review_walk_in_cash(Role::Owner, other_draft.id())
        .unwrap();
    let other_request = RequestId::new(100);
    session
        .confirm_pending(other_request, &other_review, |_, _| ())
        .unwrap();
    let other = entry
        .confirm(
            &mut session,
            ConfirmationAttempt::Retry {
                request: other_request,
            },
            |id, review| {
                authority
                    .confirm_acknowledged(Role::Owner, id, review)
                    .map(Some)
            },
        )
        .unwrap();
    assert!(matches!(other, ConfirmationOutcome::Acknowledged(other) if other != receipt));
    assert_eq!(
        (entry.active_draft(), entry.preview()),
        (Some(&edited), Some(&receipt))
    );
}
