use super::*;

#[test]
fn review_accepts_ordered_repeated_lines_for_a_catalog_product() {
    let mut fixture = fixture(8);
    let ordered = ProvisionalDraft::new(
        DraftId::new(2),
        vec![
            DraftLine::new(ProductId::new(1), NonZeroU32::new(2).unwrap()),
            DraftLine::new(ProductId::new(1), NonZeroU32::new(1).unwrap()),
        ],
    );
    let review = fixture
        .reconnect_review(Role::Owner, ordered.clone())
        .unwrap();
    assert_eq!(review.draft(), &ordered);
}

fn order(a1: u32, b: u32, a2: u32) -> ProvisionalDraft {
    ProvisionalDraft::new(
        DraftId::new(43),
        [(1, a1), (2, b), (1, a2)]
            .into_iter()
            .map(|(id, qty)| DraftLine::new(ProductId::new(id), NonZeroU32::new(qty).unwrap()))
            .collect(),
    )
}

fn catalog(a: u32, b: u32) -> ConfirmationFixture {
    let mut f =
        ConfirmationFixture::new(ProductId::new(1), a, Money::from_minor_units(125)).unwrap();
    f.add_product(ProductId::new(2), b, Money::from_minor_units(210))
        .unwrap();
    f
}

fn effects(f: &ConfirmationFixture) -> (u32, Option<u32>, i64, u64, usize, usize) {
    (
        f.stock(),
        f.product_stock(ProductId::new(2)),
        f.ledger().minor_units(),
        f.next_invoice,
        f.requests.len(),
        f.posted.len(),
    )
}

#[test]
fn ordered_mixed_sale_posts_once_and_replays_only_identical_review() {
    let mut f = catalog(8, 5);
    let review = f.reconnect_review(Role::Owner, order(2, 3, 1)).unwrap();
    assert_eq!(
        review.unit_prices(),
        &[
            Money::from_minor_units(125),
            Money::from_minor_units(210),
            Money::from_minor_units(125)
        ]
    );
    assert_eq!(
        f.confirm(Role::Manager, RequestId::new(9), &review),
        Err(ConfirmationError::AccessDenied)
    );
    let receipt = f.confirm(Role::Owner, RequestId::new(9), &review).unwrap();
    assert_eq!(
        receipt,
        Receipt {
            draft_id: DraftId::new(43),
            invoice: 1,
            quantity: 6,
            total: Money::from_minor_units(1005)
        }
    );
    assert_eq!(effects(&f), (5, Some(2), 1005, 2, 1, 1));
    f.set_product_price(ProductId::new(2), Money::from_minor_units(300))
        .unwrap();
    let changed = f.review(Role::Owner, DraftId::new(43)).unwrap();
    assert_eq!(
        f.confirm(Role::Owner, RequestId::new(9), &changed),
        Err(ConfirmationError::RequestConflict)
    );
    assert_eq!(
        f.confirm(Role::Manager, RequestId::new(9), &review),
        Err(ConfirmationError::AccessDenied)
    );
    assert_eq!(
        f.confirm(Role::Owner, RequestId::new(9), &review),
        Ok(receipt)
    );
    assert_eq!(effects(&f), (5, Some(2), 1005, 2, 1, 1));
}

#[test]
fn later_product_shortage_leaves_every_effect_unchanged() {
    let mut f = catalog(8, 2);
    let review = f.reconnect_review(Role::Owner, order(2, 3, 1)).unwrap();
    assert_eq!(
        f.confirm(Role::Owner, RequestId::new(9), &review),
        Err(ConfirmationError::InsufficientStock)
    );
    assert_eq!(effects(&f), (8, Some(2), 0, 1, 0, 0));
}

#[test]
fn repeated_product_cumulative_shortage_leaves_every_effect_unchanged() {
    let mut f = catalog(2, 5);
    let review = f.reconnect_review(Role::Owner, order(2, 3, 1)).unwrap();
    assert_eq!(
        f.confirm(Role::Owner, RequestId::new(9), &review),
        Err(ConfirmationError::InsufficientStock)
    );
    assert_eq!(effects(&f), (2, Some(5), 0, 1, 0, 0));
}

#[test]
fn later_line_total_overflow_leaves_every_effect_unchanged() {
    let mut f = catalog(8, 5);
    f.set_product_price(ProductId::new(2), Money::from_minor_units(i64::MAX))
        .unwrap();
    let review = f.reconnect_review(Role::Owner, order(2, 3, 1)).unwrap();
    assert_eq!(
        f.confirm(Role::Owner, RequestId::new(9), &review),
        Err(ConfirmationError::Overflow)
    );
    assert_eq!(effects(&f), (8, Some(5), 0, 1, 0, 0));
}

#[test]
fn changed_mixed_draft_conflicts_for_posted_request_and_old_terms_go_stale() {
    let mut f = catalog(8, 5);
    let original = f.reconnect_review(Role::Owner, order(2, 3, 1)).unwrap();
    f.set_product_price(ProductId::new(2), Money::from_minor_units(211))
        .unwrap();
    assert_eq!(
        f.confirm(Role::Owner, RequestId::new(9), &original),
        Err(ConfirmationError::StaleReview)
    );
    let current = f.review(Role::Owner, DraftId::new(43)).unwrap();
    f.confirm(Role::Owner, RequestId::new(9), &current).unwrap();
    let changed = f.reconnect_review(Role::Owner, order(1, 3, 2)).unwrap();
    assert_eq!(
        f.confirm(Role::Owner, RequestId::new(9), &changed),
        Err(ConfirmationError::RequestConflict)
    );
}
