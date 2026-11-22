use super::*;
use crate::CrateRowAlreadyPresent;
use crate::confirmation::pending;

#[test]
fn adding_a_second_crate_row_preserves_the_original_until_explicit_replacement() {
    let mut draft = ProvisionalDraft::new(ID, vec![]);
    let replacement = CrateRow::new(qty(1), vec![components()[1]]);
    assert_eq!(draft.replace_crate_row(replacement.clone()), None);
    assert!(draft.crate_row().is_none());

    let original = CrateRow::new(qty(2), components().to_vec());
    assert_eq!(draft.add_crate_row(original.clone()), Ok(()));
    assert_eq!(
        draft.add_crate_row(replacement.clone()),
        Err(CrateRowAlreadyPresent)
    );
    assert_eq!(draft.crate_row(), Some(&original));

    assert_eq!(draft.replace_crate_row(replacement.clone()), Some(original));
    assert_eq!(draft.crate_row(), Some(&replacement));
}

#[test]
fn crate_requires_exact_current_authority_approval() {
    let draft = distinct_order();
    let mut configured = authority(false, draft.clone());
    assert_eq!(
        configured.review(Role::Owner, ID),
        Err(ConfirmationError::InvalidDraft)
    );
    assert_eq!(
        configured.review_named(Role::Owner, ID, money(100)),
        Err(ConfirmationError::InvalidDraft)
    );
    assert_eq!(
        configured.reconnect_review(Role::Owner, draft.clone()),
        Err(ConfirmationError::InvalidDraft)
    );
    assert_eq!(
        configured.review_checkout(Role::Manager, ID, &[], money(100)),
        Err(ConfirmationError::AccessDenied)
    );
    let approved = approval(&mut configured);
    let encoded = pending::encode(&approved);
    let old = "5|43|1|5|2,63726174652D6275796572,53796E746865746963|2;2,9,0;1,2,1|450|13|6,1|100";
    let undated = pending::decode(old).unwrap();
    assert_eq!(undated.draft.bill_date(), None);
    assert_eq!(pending::encode(&undated), old);
    assert_eq!(encoded, format!("6|2024-02-29|{old}"));
    let decoded = pending::decode(&encoded).unwrap();
    assert_eq!(decoded, approved);
    let mut unapproved = authority(false, draft);
    assert_eq!(
        unapproved.confirm(Role::Owner, REQUEST, &decoded),
        Err(ConfirmationError::StaleReview)
    );
    let altered = pending::decode(&encoded.replace("|100", "|101")).unwrap();
    assert_ne!(altered, approved);
    assert_eq!(
        configured.confirm(Role::Owner, REQUEST, &altered),
        Err(ConfirmationError::StaleReview)
    );
    let altered_product = pending::decode(&encoded.replace(";1,2,1", ";2,2,1")).unwrap();
    assert_ne!(altered_product, approved);
    assert_eq!(
        configured.confirm(Role::Owner, REQUEST, &altered_product),
        Err(ConfirmationError::StaleReview)
    );
    for (original, malformed) in [("6,1|", "6,0|"), (";2,9,0", ";2,8,0")] {
        assert!(pending::decode(&encoded.replace(original, malformed)).is_err());
    }
    configured.set_crate_rate(money(451)).unwrap();
    assert_eq!(
        configured.confirm(Role::Owner, REQUEST, &approved),
        Err(ConfirmationError::StaleReview)
    );
    assert_eq!(effects(&configured), (0, 0, 0, 1, 0, 0));
    assert_eq!(effects(&unapproved), (0, 0, 0, 1, 0, 0));
}

#[test]
fn mixed_empty_fractional_and_overflowing_crate_inputs_are_declined() {
    for (draft, error) in [
        (
            order(
                CrateRow::new(qty(2), components().to_vec()),
                vec![DraftLine::new(CASE, qty(1))],
            ),
            ConfirmationError::InvalidDraft,
        ),
        (
            order(CrateRow::new(qty(2), vec![]), vec![]),
            ConfirmationError::MissingBillableItems,
        ),
        (
            order(CrateRow::new(qty(1), components().to_vec()), vec![]),
            ConfirmationError::InvalidAmount,
        ),
        (
            order(CrateRow::new(qty(u32::MAX), vec![components()[1]]), vec![]),
            ConfirmationError::Overflow,
        ),
    ] {
        let mut configured = authority(false, draft);
        assert_eq!(
            configured.review_checkout(Role::Owner, ID, &[], money(0)),
            Err(error)
        );
        assert!(configured.checkout_approvals.is_empty());
        assert_eq!(effects(&configured), (0, 0, 0, 1, 0, 0));
    }
}

#[test]
fn replacing_regular_draft_with_crate_invalidates_all_prior_review_paths() {
    let draft = ProvisionalDraft::new(ID, vec![DraftLine::new(CASE, qty(2))]);
    let draft = draft
        .with_customer(distinct_order().customer().clone())
        .with_bill_date(test_bill_date());
    let mut configured = authority(false, draft);
    let legacy = configured.review(Role::Owner, ID).unwrap();
    let named = configured
        .review_named(Role::Owner, ID, money(100))
        .unwrap();
    let product_checkout = configured
        .review_checkout(Role::Owner, ID, &[SaleUnit::Case], money(100))
        .unwrap();
    configured.register_draft(distinct_order()).unwrap();
    for prior in [legacy, named, product_checkout] {
        assert_eq!(
            configured.confirm(Role::Owner, REQUEST, &prior),
            Err(ConfirmationError::StaleReview)
        );
    }
    assert_eq!(effects(&configured), (0, 0, 0, 1, 0, 0));
}
