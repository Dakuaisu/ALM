use super::*;
use crate::DraftCustomerId;

#[test]
fn named_part_payment_posts_separate_financial_amounts() {
    let mut f = fixture(5);
    let order = draft(1, 2).with_customer(DraftCustomer::Selected {
        id: DraftCustomerId::new("customer-a".into()),
        name: "Synthetic customer".into(),
    });
    f.reconnect_review(Role::Owner, order).unwrap();
    let review = f
        .review_named(Role::Owner, DraftId::new(1), Money::from_minor_units(100))
        .unwrap();
    assert_eq!(f.ledger().minor_units(), 0);
    let receipt = f.confirm(Role::Owner, RequestId::new(1), &review).unwrap();
    assert_eq!(receipt.total.minor_units(), 250);
    assert_eq!(receipt.received.unwrap().minor_units(), 100);
    assert_eq!(receipt.due.unwrap().minor_units(), 150);
    assert_eq!(f.customer_due("customer-a").minor_units(), 150);
    assert_eq!(f.received_ledger().minor_units(), 100);
}

#[test]
fn invalid_named_input_and_manager_review_do_not_post() {
    let mut f = fixture(5);
    for customer in [
        DraftCustomer::Unassigned,
        DraftCustomer::Typed("Name".into()),
        DraftCustomer::Selected {
            id: DraftCustomerId::new(" ".into()),
            name: "Name".into(),
        },
        DraftCustomer::Selected {
            id: DraftCustomerId::new("id".into()),
            name: " ".into(),
        },
    ] {
        f.reconnect_review(Role::Owner, draft(1, 2).with_customer(customer))
            .unwrap();
        assert_eq!(
            f.review_named(Role::Owner, DraftId::new(1), Money::from_minor_units(0)),
            Err(ConfirmationError::InvalidDraft)
        );
    }
    f.reconnect_review(
        Role::Owner,
        draft(1, 2).with_customer(DraftCustomer::Selected {
            id: DraftCustomerId::new("id".into()),
            name: "Name".into(),
        }),
    )
    .unwrap();
    for amount in [-1, 251] {
        assert_eq!(
            f.review_named(
                Role::Owner,
                DraftId::new(1),
                Money::from_minor_units(amount)
            ),
            Err(ConfirmationError::InvalidAmount)
        );
    }
    assert_eq!(
        f.review_named(Role::Manager, DraftId::new(1), Money::from_minor_units(0)),
        Err(ConfirmationError::AccessDenied)
    );
    assert_eq!(
        (
            f.stock(),
            f.ledger().minor_units(),
            f.received_ledger().minor_units(),
            f.posted_count()
        ),
        (5, 0, 0, 0)
    );
}

#[test]
fn balance_overflow_after_fifo_staging_rolls_back() {
    let mut f = fixture(5);
    f.customer_balances.insert("id".into(), i64::MAX);
    f.reconnect_review(
        Role::Owner,
        draft(1, 2).with_customer(DraftCustomer::Selected {
            id: DraftCustomerId::new("id".into()),
            name: "Name".into(),
        }),
    )
    .unwrap();
    let review = f
        .review_named(Role::Owner, DraftId::new(1), Money::from_minor_units(0))
        .unwrap();
    assert_eq!(
        f.confirm(Role::Owner, RequestId::new(1), &review),
        Err(ConfirmationError::Overflow)
    );
    assert_eq!(
        (
            f.stock(),
            f.ledger().minor_units(),
            f.received_ledger().minor_units(),
            f.customer_due("id").minor_units(),
            f.next_invoice,
            f.posted_count()
        ),
        (5, 0, 0, i64::MAX, 1, 0)
    );
    assert!(f.catalog.allocation_snapshot().is_empty());
}

#[test]
fn unpaid_sales_accumulate_by_customer_id_and_shortage_is_atomic() {
    let mut f = fixture(3);
    for (draft_id, customer_id, name, balance) in [
        (1, "a", "Same", 125),
        (2, "b", "Same", 125),
        (3, "a", "Renamed", 250),
    ] {
        f.reconnect_review(
            Role::Owner,
            draft(draft_id, 1).with_customer(DraftCustomer::Selected {
                id: DraftCustomerId::new(customer_id.into()),
                name: name.into(),
            }),
        )
        .unwrap();
        let review = f
            .review_named(
                Role::Owner,
                DraftId::new(draft_id),
                Money::from_minor_units(0),
            )
            .unwrap();
        let receipt = f
            .confirm(Role::Owner, RequestId::new(draft_id), &review)
            .unwrap();
        assert_eq!(
            (
                receipt.total,
                receipt.received,
                receipt.due,
                receipt.customer_balance
            ),
            (
                Money::from_minor_units(125),
                Some(Money::from_minor_units(0)),
                Some(Money::from_minor_units(125)),
                Some(Money::from_minor_units(balance))
            )
        );
    }
    assert_eq!(
        (
            f.customer_due("a").minor_units(),
            f.customer_due("b").minor_units(),
            f.received_ledger().minor_units()
        ),
        (250, 125, 0)
    );
    f.reconnect_review(
        Role::Owner,
        draft(4, 1).with_customer(DraftCustomer::Selected {
            id: DraftCustomerId::new("a".into()),
            name: "Same".into(),
        }),
    )
    .unwrap();
    let review = f
        .review_named(Role::Owner, DraftId::new(4), Money::from_minor_units(25))
        .unwrap();
    let allocations = f.catalog.allocation_snapshot();
    let lots = f.catalog.lot_snapshot(ProductId::new(1));
    let costs = [1, 2, 3].map(|id| f.catalog.allocated_cost(DraftId::new(id)));
    let requests = f.requests.clone();
    let posted = f.posted.clone();
    let balances = f.customer_balances.clone();
    assert_eq!(
        f.confirm(Role::Owner, RequestId::new(4), &review),
        Err(ConfirmationError::InsufficientStock)
    );
    assert_eq!(
        (
            f.customer_due("a").minor_units(),
            f.ledger().minor_units(),
            f.received_ledger().minor_units(),
            f.posted_count(),
            f.next_invoice,
            f.stock()
        ),
        (250, 375, 0, 3, 4, 0)
    );
    assert_eq!(f.catalog.allocation_snapshot(), allocations);
    assert_eq!(f.catalog.lot_snapshot(ProductId::new(1)), lots);
    assert_eq!(
        [1, 2, 3].map(|id| f.catalog.allocated_cost(DraftId::new(id))),
        costs
    );
    assert_eq!(f.catalog.allocated_cost(DraftId::new(4)), None);
    assert_eq!(f.requests, requests);
    assert_eq!(f.posted, posted);
    assert_eq!(f.customer_balances, balances);
}

#[test]
fn replay_returns_original_balance_after_later_sale() {
    let mut f = fixture(3);
    let customer = DraftCustomer::Selected {
        id: DraftCustomerId::new("a".into()),
        name: "Name".into(),
    };
    f.reconnect_review(Role::Owner, draft(1, 1).with_customer(customer.clone()))
        .unwrap();
    let first = f
        .review_named(Role::Owner, DraftId::new(1), Money::from_minor_units(25))
        .unwrap();
    let receipt = f.confirm(Role::Owner, RequestId::new(1), &first).unwrap();
    f.reconnect_review(Role::Owner, draft(2, 1).with_customer(customer))
        .unwrap();
    let later = f
        .review_named(Role::Owner, DraftId::new(2), Money::from_minor_units(0))
        .unwrap();
    f.confirm(Role::Owner, RequestId::new(2), &later).unwrap();
    assert_eq!(f.customer_due("a").minor_units(), 225);
    assert_eq!(receipt.customer_balance.unwrap().minor_units(), 100);
    assert_eq!(
        f.confirm(Role::Owner, RequestId::new(1), &first),
        Ok(receipt)
    );
    assert_eq!(f.customer_due("a").minor_units(), 225);
}
