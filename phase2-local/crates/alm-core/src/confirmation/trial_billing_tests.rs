use std::num::NonZeroU32;

use super::{
    CUSTOMER, DAY, OwnerPosting, ReceiptTerms, SaleTerms, TrialBillingError, customer_id, fixture,
    parse_rupees, preview_unit_prices, save_receipt,
};
use crate::confirmation::{ConfirmationError, ProposedPayment, ReceiptMode};
use crate::{
    DraftCustomer, DraftCustomerId, DraftId, DraftLine, Money, ProductId, ProvisionalDraft, Role,
};

fn draft(customer: DraftCustomer) -> ProvisionalDraft {
    ProvisionalDraft::new(
        DraftId::new(1),
        vec![DraftLine::new(ProductId::new(1), NonZeroU32::MIN)],
    )
    .with_customer(customer)
}

fn terms(paid: &str) -> SaleTerms<'_> {
    SaleTerms { date: DAY, paid }
}

fn receipt_terms(amount: &str) -> ReceiptTerms<'_> {
    ReceiptTerms {
        date: DAY,
        amount,
        mode: ReceiptMode::Cash,
    }
}

#[test]
fn decimal_rupees_convert_exactly_at_the_i64_boundary() {
    for (text, paise) in [
        ("0", 0),
        ("  .5 ", 50),
        (".01", 1),
        ("1.", 100),
        ("20.00", 2_000),
        ("92233720368547758.07", i64::MAX),
    ] {
        assert_eq!(parse_rupees(text), Ok(Money::from_minor_units(paise)));
    }
    for text in [
        "", " ", ".", "+1", "-0.01", "1e2", "1.2.3", "0.001", "1,000", "١",
    ] {
        assert_eq!(parse_rupees(text), Err(ConfirmationError::InvalidAmount));
    }
    for text in [
        "92233720368547758.08",
        "92233720368547759",
        "9223372036854775808",
    ] {
        assert_eq!(parse_rupees(text), Err(ConfirmationError::Overflow));
    }
}

#[test]
fn posting_review_accepts_zero_and_half_rupee_without_posting() {
    let mut authority = fixture().unwrap();
    let mut posting = OwnerPosting::default();
    let draft = draft(DraftCustomer::Typed(CUSTOMER.into()));
    for (paid, paise) in [("0", 0), (".5", 50), ("120.50", 12_050)] {
        posting
            .prepare(&mut authority, Role::Owner, Some(&draft), terms(paid))
            .unwrap();
        let pending = posting.pending(Role::Owner).unwrap();
        assert_eq!(
            pending.review().payment(),
            ProposedPayment::Named(Money::from_minor_units(paise))
        );
    }
    assert_eq!(authority.posted_count(), 0);
    assert_eq!(authority.ledger().minor_units(), 0);
}

#[test]
fn standalone_receipt_requires_positive_rupees() {
    let mut authority = fixture().unwrap();
    let before = authority.clone();
    for (amount, error) in [
        ("0.00", ConfirmationError::InvalidAmount),
        (".001", ConfirmationError::InvalidAmount),
        ("92233720368547758.08", ConfirmationError::Overflow),
    ] {
        assert_eq!(
            save_receipt(&mut authority, Role::Owner, receipt_terms(amount)),
            Err(TrialBillingError::Confirmation(error))
        );
        assert_eq!(authority, before);
    }
    save_receipt(&mut authority, Role::Owner, receipt_terms(".5")).unwrap();
    assert_eq!(
        authority.customer_due(customer_id().value()).minor_units(),
        -50
    );
    assert_eq!(authority.catalog, before.catalog);
    assert_eq!(authority.ledger(), before.ledger());
    assert_eq!(authority.next_invoice, before.next_invoice);
    assert_eq!(authority.current_sales, before.current_sales);
}

#[test]
fn selected_identity_is_not_rebound_by_its_label() {
    let mut authority = fixture().unwrap();
    let before = authority.clone();
    let mut posting = OwnerPosting::default();
    for customer in [
        DraftCustomer::Selected {
            id: DraftCustomerId::new("other".into()),
            name: CUSTOMER.into(),
        },
        DraftCustomer::Selected {
            id: customer_id(),
            name: "Other".into(),
        },
        DraftCustomer::Typed("Other".into()),
        DraftCustomer::Unassigned,
    ] {
        let draft = draft(customer);
        assert_eq!(
            posting.prepare(&mut authority, Role::Owner, Some(&draft), terms("0")),
            Err(TrialBillingError::Customer)
        );
        assert_eq!(authority, before);
    }
    let selected = DraftCustomer::Selected {
        id: customer_id(),
        name: CUSTOMER.into(),
    };
    let draft = draft(selected.clone());
    posting
        .prepare(&mut authority, Role::Owner, Some(&draft), terms("0"))
        .unwrap();
    let pending = posting.pending(Role::Owner).unwrap();
    assert_eq!(pending.review().draft().customer(), &selected);
}

#[test]
fn manager_and_stale_serial_cannot_change_authority() {
    let mut authority = fixture().unwrap();
    let mut posting = OwnerPosting::default();
    let draft = draft(DraftCustomer::Typed(CUSTOMER.into()));
    let before = authority.clone();
    let denied = TrialBillingError::Confirmation(ConfirmationError::AccessDenied);
    assert_eq!(
        posting.prepare(&mut authority, Role::Manager, None, terms("bad")),
        Err(denied)
    );
    assert_eq!(
        save_receipt(&mut authority, Role::Manager, receipt_terms("bad")),
        Err(denied)
    );
    assert_eq!(authority, before);

    posting
        .prepare(&mut authority, Role::Owner, Some(&draft), terms("0"))
        .unwrap();
    let stale = posting.pending(Role::Owner).unwrap().serial();
    posting
        .prepare(&mut authority, Role::Owner, Some(&draft), terms(".5"))
        .unwrap();
    let current = posting.pending(Role::Owner).unwrap().serial();
    let before = authority.clone();
    assert!(posting.pending(Role::Manager).is_none());
    assert_eq!(
        posting.confirm(&mut authority, Role::Manager, current),
        Err(denied)
    );
    assert_eq!(
        posting.confirm(&mut authority, Role::Owner, stale),
        Err(TrialBillingError::Confirmation(
            ConfirmationError::StaleReview
        ))
    );
    assert_eq!(authority, before);

    assert_eq!(
        posting.prepare(&mut authority, Role::Owner, Some(&draft), terms("120.51")),
        Err(TrialBillingError::Confirmation(
            ConfirmationError::InvalidAmount
        ))
    );
    assert!(posting.pending(Role::Owner).is_none());
    assert_eq!(
        posting.confirm(&mut authority, Role::Owner, current),
        Err(TrialBillingError::NoReview)
    );
    assert_eq!(authority, before);
}

#[test]
fn preview_preserves_current_review_and_serial_exhaustion_does_not_wrap() {
    let mut authority = fixture().unwrap();
    let mut posting = OwnerPosting {
        last_serial: u64::MAX - 1,
        pending: None,
    };
    let draft = draft(DraftCustomer::Typed(CUSTOMER.into()));
    posting
        .prepare(&mut authority, Role::Owner, Some(&draft), terms(".5"))
        .unwrap();
    let before = authority.clone();
    assert_eq!(
        preview_unit_prices(&authority, Role::Manager, &draft),
        Err(ConfirmationError::AccessDenied)
    );
    assert_eq!(
        preview_unit_prices(&authority, Role::Owner, &draft),
        Ok(vec![Money::from_minor_units(12_050)])
    );
    assert_eq!(authority, before);

    let serial = posting.pending(Role::Owner).unwrap().serial();
    assert_eq!(serial, u64::MAX);
    let receipt = posting
        .confirm(&mut authority, Role::Owner, serial)
        .unwrap();
    assert_eq!(receipt.received, Some(Money::from_minor_units(50)));
    let before = authority.clone();
    assert_eq!(
        posting.prepare(&mut authority, Role::Owner, Some(&draft), terms("0")),
        Err(TrialBillingError::ReviewLimit)
    );
    assert!(posting.pending(Role::Owner).is_none());
    assert_eq!(authority, before);
}
