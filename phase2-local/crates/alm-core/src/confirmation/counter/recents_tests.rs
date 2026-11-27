use std::num::NonZeroU32;

use crate::confirmation::demo_fixture::{customer_id, directory, fixture};
use crate::confirmation::owner_billing::{OwnerPosting, SaleTerms};
use crate::confirmation::{ConfirmationFixture, CounterTag};
use crate::{BillDate, DraftCustomer, DraftId, DraftLine, ProductId, ProvisionalDraft, Role};

fn post(fixture: &mut ConfirmationFixture, id: u64, date: &str, paid: &str) {
    let draft = ProvisionalDraft::new(
        DraftId::new(id),
        vec![DraftLine::new(ProductId::new(1), NonZeroU32::MIN)],
    )
    .with_customer(DraftCustomer::Selected {
        id: customer_id(),
        name: "Synthetic Customer".into(),
    });
    let mut posting = OwnerPosting::default();
    posting
        .prepare(
            fixture,
            &directory(),
            Role::Owner,
            Some(&draft),
            SaleTerms { date, paid },
        )
        .unwrap();
    posting.confirm(fixture, Role::Owner, 1).unwrap();
}

#[test]
fn recents_tag_unpaid_remainders_as_credit_without_amounts_or_effects() {
    let mut fixture = fixture().unwrap();
    post(&mut fixture, 1, "2025-04-29", "120.50");
    post(&mut fixture, 2, "2025-04-30", "20");
    let today = BillDate::new(2025, 4, 30).unwrap();
    let before = fixture.clone();
    let counter = fixture.counter(Role::Owner, today).unwrap();
    assert_eq!(counter.billed_today_count, 1);
    let recents = fixture.counter_recents(Role::Owner, &counter).unwrap();
    let tags: Vec<_> = recents
        .iter()
        .map(|row| (row.bill.invoice, row.tag))
        .collect();
    assert_eq!(tags, [(2, CounterTag::Credit), (1, CounterTag::Cash)]);
    assert_eq!(recents[0].customer.as_deref(), Some("Synthetic Customer"));
    assert_eq!(recents[1].date, BillDate::new(2025, 4, 29).ok());
    // DEC-034: a Manager's Counter lists the same recent bills (no profit).
    assert_eq!(
        fixture.counter_recents(Role::Manager, &counter),
        Ok(recents)
    );
    assert_eq!(fixture, before);
}
