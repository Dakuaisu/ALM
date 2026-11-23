use super::*;
use alm_phase2_local::confirmation::PendingError;
use alm_phase2_local::draft_entry::{
    ActiveBillForm, BillEntryState, ConfirmationAttempt, ConfirmationOutcome,
};

#[test]
fn walk_in_ack_must_match_request_and_review_and_retries_cannot_replace_an_existing_preview_on_edit()
 {
    let scratch = Scratch::new();
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
