use super::*;

pub(super) fn seeded() -> (ConfirmationFixture, BillDate, EditChanges) {
    let today = BillDate::new(2024, 2, 29).unwrap();
    let yesterday = BillDate::new(2024, 2, 28).unwrap();
    let mut f =
        ConfirmationFixture::new(ProductId::new(1), 10, Money::from_minor_units(125)).unwrap();
    f.add_product(ProductId::new(2), 10, Money::from_minor_units(80))
        .unwrap();
    for (id, key, date) in [
        (1, None, today),
        (2, Some(0), yesterday),
        (3, Some(10), today),
        (4, Some(30), today),
        (5, None, today),
    ] {
        f.register_draft(draft(id, 1).with_bill_date(Some(date)).with_customer(
            DraftCustomer::Selected {
                id: DraftCustomerId::new("buyer".into()),
                name: "Synthetic".into(),
            },
        ))
        .unwrap();
        let review = f
            .review_named(Role::Owner, DraftId::new(id), Money::from_minor_units(50))
            .unwrap();
        f.confirm_with_creation_key(
            Role::Owner,
            RequestId::new(id),
            &review,
            key.map(CreationKey::new),
        )
        .unwrap();
    }
    let changes = EditChanges {
        retained_price: Money::from_minor_units(125),
        appended_line: DraftLine::new(ProductId::new(2), NonZeroU32::MIN),
        appended_price: Money::from_minor_units(80),
    };
    (f, today, changes)
}

fn readback(
    f: &ConfirmationFixture,
    today: BillDate,
    label: &str,
    id: u64,
    before: Option<CreationKey>,
    expected: Option<CreationKey>,
    recents: &[u64],
) {
    let key = f
        .first_posts
        .iter()
        .find(|(draft, _)| *draft == DraftId::new(id))
        .unwrap()
        .1;
    let counter = f.counter(Role::Owner, today).unwrap();
    let order: Vec<_> = counter.recents.iter().map(|row| row.invoice).collect();
    let billed: Vec<_> = counter
        .billed_today
        .iter()
        .map(|row| (row.draft_id.value(), row.invoice))
        .collect();
    println!(
        "{label}: invoice={id}, key before={before:?}, after={key:?}, recents invoices={order:?}, billed_today={billed:?}"
    );
    assert_eq!(key, expected);
    assert_eq!(order, recents);
    assert_eq!(billed, [(1, 1), (3, 3), (4, 4), (5, 5)]);
    assert_eq!(counter.billed_today_count, 4);
    for (draft, date) in [
        (1, today),
        (2, BillDate::new(2024, 2, 28).unwrap()),
        (3, today),
        (4, today),
        (5, today),
    ] {
        let (review, receipt) = f.current_sale(Role::Owner, DraftId::new(draft)).unwrap();
        assert_eq!(review.draft().bill_date(), Some(date));
        assert_eq!(receipt.invoice, draft);
    }
}

#[test]
fn controlled_edit_creation_key_counter_readback() {
    let (mut f, today, changes) = seeded();
    let time = CreationKey::new(20);
    let before = f.clone();
    let edit = f.begin_edit(Role::Owner, DraftId::new(1), changes).unwrap();
    drop(edit);
    assert_eq!(f, before);
    readback(
        &f,
        today,
        "opened/abandoned",
        1,
        None,
        None,
        &[4, 3, 5, 2, 1],
    );
    let edit = f.begin_edit(Role::Owner, DraftId::new(1), changes).unwrap();
    assert_eq!(
        f.save_edit_with_creation_key(Role::Manager, &edit, time),
        Err(ConfirmationError::AccessDenied)
    );
    assert_eq!(f, before);
    readback(&f, today, "denied", 1, None, None, &[4, 3, 5, 2, 1]);
    f.save_edit_with_creation_key(Role::Owner, &edit, time)
        .unwrap();
    readback(
        &f,
        today,
        "missing saved",
        1,
        None,
        Some(time),
        &[4, 1, 3, 5, 2],
    );
    assert_eq!(
        f.save_edit_with_creation_key(Role::Owner, &edit, CreationKey::new(99)),
        Err(ConfirmationError::StaleReview)
    );
    readback(
        &f,
        today,
        "stale save",
        1,
        Some(time),
        Some(time),
        &[4, 1, 3, 5, 2],
    );
    let zero = f.begin_edit(Role::Owner, DraftId::new(2), changes).unwrap();
    f.save_edit_with_creation_key(Role::Owner, &zero, time)
        .unwrap();
    readback(
        &f,
        today,
        "zero saved",
        2,
        Some(CreationKey::new(0)),
        Some(time),
        &[4, 2, 1, 3, 5],
    );
    let existing = f.begin_edit(Role::Owner, DraftId::new(3), changes).unwrap();
    f.save_edit_with_creation_key(Role::Owner, &existing, CreationKey::new(99))
        .unwrap();
    readback(
        &f,
        today,
        "nonzero saved",
        3,
        Some(CreationKey::new(10)),
        Some(CreationKey::new(10)),
        &[4, 2, 1, 3, 5],
    );
    readback(&f, today, "untouched", 5, None, None, &[4, 2, 1, 3, 5]);
}

#[test]
fn failed_edit_does_not_assign_creation_key() {
    let (mut f, today, mut changes) = seeded();
    changes.appended_line = DraftLine::new(ProductId::new(2), NonZeroU32::new(100).unwrap());
    let edit = f.begin_edit(Role::Owner, DraftId::new(1), changes).unwrap();
    let before = f.clone();
    assert_eq!(
        f.save_edit_with_creation_key(Role::Owner, &edit, CreationKey::new(20)),
        Err(ConfirmationError::InsufficientStock)
    );
    assert_eq!(f, before);
    readback(&f, today, "failed stock", 1, None, None, &[4, 3, 5, 2, 1]);
}

#[test]
fn missing_first_post_metadata_rejects_without_mutation() {
    let (mut f, today, changes) = seeded();
    let edit = f.begin_edit(Role::Owner, DraftId::new(1), changes).unwrap();
    f.first_posts.remove(0);
    let before = f.clone();
    assert_eq!(
        f.save_edit_with_creation_key(Role::Owner, &edit, CreationKey::new(20)),
        Err(ConfirmationError::InvalidDraft)
    );
    assert_eq!(f, before);
    let counter = f.counter(Role::Owner, today).unwrap();
    let invoices: Vec<_> = counter.recents.iter().map(|row| row.invoice).collect();
    let billed: Vec<_> = counter.billed_today.iter().map(|row| row.invoice).collect();
    println!(
        "missing metadata: invoice=1, key before=absent, after=absent, recents invoices={invoices:?}, billed_today invoices={billed:?}"
    );
    assert_eq!(invoices, [4, 3, 5, 2]);
    assert_eq!(billed, [3, 4, 5]);
}

#[test]
fn no_time_save_keeps_missing_creation_key() {
    let (mut f, today, changes) = seeded();
    let edit = f.begin_edit(Role::Owner, DraftId::new(1), changes).unwrap();
    f.save_edit(Role::Owner, &edit).unwrap();
    readback(&f, today, "no-time save", 1, None, None, &[4, 3, 5, 2, 1]);
}
