use super::restart::Scratch;
use super::*;
use crate::{LocalSales, Money};

#[test]
fn owner_persists_approval_before_sending_and_retries_it_after_draft_edit() {
    let scratch = Scratch::new();
    let path = scratch.0.join("pending.sqlite");
    let mut authority = fixture(5);
    let review = authority.review(Role::Owner, DraftId::new(1)).unwrap();
    let id = RequestId::new(71);
    let mut sales = LocalSales::open_sqlite(&path, Money::from_minor_units(0)).unwrap();
    sales
        .session(Role::Owner)
        .confirm_pending(id, &review, |sent_id, sent_review| {
            let mut other = LocalSales::open_sqlite(&path, Money::from_minor_units(0)).unwrap();
            other
                .session(Role::Owner)
                .retry_pending(id, |stored_id, stored_review| {
                    assert_eq!((stored_id, stored_review), (sent_id, sent_review));
                })
                .unwrap();
        })
        .unwrap();
    sales.session(Role::Owner).save_draft(draft(1, 3)).unwrap();
    drop(sales);
    authority
        .set_unit_price(Money::from_minor_units(400))
        .unwrap();
    let mut reopened = LocalSales::open_sqlite(&path, Money::from_minor_units(0)).unwrap();
    reopened
        .session(Role::Owner)
        .retry_pending(id, |stored_id, stored_review| {
            assert_eq!((stored_id, stored_review), (id, &review));
        })
        .unwrap();
}
