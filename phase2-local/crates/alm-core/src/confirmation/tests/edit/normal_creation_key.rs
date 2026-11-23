use super::*;
use crate::draft_entry::BillEntryState;

fn controlled_time() -> Result<CreationKey, ConfirmationError> {
    Ok(CreationKey::new(20))
}

fn bad_time() -> Result<CreationKey, ConfirmationError> {
    Err(ConfirmationError::Overflow)
}

fn entry(clock: fn() -> Result<CreationKey, ConfirmationError>) -> BillEntryState {
    let mut entry = BillEntryState::default();
    entry.set_edit_clock_for_test(clock);
    entry
}

fn check_counter(f: &ConfirmationFixture, today: BillDate, recents: &[u64]) {
    let counter = f.counter(Role::Owner, today).unwrap();
    let actual: Vec<_> = counter.recents.iter().map(|row| row.invoice).collect();
    let billed: Vec<_> = counter.billed_today.iter().map(|row| row.invoice).collect();
    println!("recents={actual:?}, billed_today={billed:?}");
    assert_eq!(actual, recents);
    assert_eq!(billed, [1, 3, 4, 5]);
    assert_eq!(counter.billed_today_count, 4);
}

#[test]
fn normal_save_assigns_missing_and_zero_keys_but_preserves_nonzero() {
    let (mut f, today, changes) = super::creation_key::seeded();
    let mut entry = entry(controlled_time);
    check_counter(&f, today, &[4, 3, 5, 2, 1]);
    for (id, expected, before, after) in [
        (1, &[4, 1, 3, 5, 2][..], None, 20),
        (2, &[4, 2, 1, 3, 5][..], Some(0), 20),
        (3, &[4, 2, 1, 3, 5][..], Some(10), 10),
    ] {
        let draft = DraftId::new(id);
        let index = f
            .first_posts
            .iter()
            .position(|(id, _)| *id == draft)
            .unwrap();
        let before_key = f.first_posts[index].1;
        assert_eq!(before_key, before.map(CreationKey::new));
        let edit = f.begin_edit(Role::Owner, draft, changes).unwrap();
        entry.open_edit(Role::Owner, edit).unwrap();
        let saved = entry.save_edit(Role::Owner, &mut f).unwrap();
        assert_eq!(saved.invoice, id);
        assert_eq!(entry.preview(), Some(&saved));
        assert_eq!(
            f.current_sale(Role::Owner, DraftId::new(id)).unwrap().1,
            &saved
        );
        let after_key = f.first_posts[index].1;
        assert_eq!(after_key, Some(CreationKey::new(after)));
        println!(
            "saved invoice={id}, preview invoice={}, before={before_key:?}, after={after_key:?}",
            entry.preview().unwrap().invoice,
        );
        check_counter(&f, today, expected);
    }
    assert_eq!(f.first_posts[4].1, None);
}

#[test]
fn normal_edit_denial_abandonment_and_failed_save_preserve_state() {
    let (mut f, today, mut changes) = super::creation_key::seeded();
    let mut entry = entry(bad_time);
    let edit = f.begin_edit(Role::Owner, DraftId::new(1), changes).unwrap();
    let before = f.clone();
    assert_eq!(
        entry.open_edit(Role::Manager, edit.clone()),
        Err(ConfirmationError::AccessDenied)
    );
    entry.open_edit(Role::Owner, edit).unwrap();
    assert_eq!(
        entry.save_edit(Role::Manager, &mut f),
        Err(ConfirmationError::AccessDenied)
    );
    assert_eq!(f, before);
    println!("Manager denial: key unchanged");
    check_counter(&f, today, &[4, 3, 5, 2, 1]);
    entry.cancel_edit();
    assert_eq!(f, before);
    assert_eq!(entry.preview(), None);
    println!("abandonment: key unchanged");
    check_counter(&f, today, &[4, 3, 5, 2, 1]);
    changes.appended_line = DraftLine::new(ProductId::new(2), NonZeroU32::new(100).unwrap());
    let edit = f.begin_edit(Role::Owner, DraftId::new(1), changes).unwrap();
    entry.open_edit(Role::Owner, edit).unwrap();
    assert_eq!(
        entry.save_edit(Role::Owner, &mut f),
        Err(ConfirmationError::InsufficientStock)
    );
    assert_eq!(f, before);
    assert_eq!(entry.preview(), None);
    println!("stock shortage: key unchanged");
    check_counter(&f, today, &[4, 3, 5, 2, 1]);
}

#[test]
fn normal_save_reads_clock_only_for_missing_or_zero_keys() {
    let (mut f, today, changes) = super::creation_key::seeded();
    let mut entry = entry(bad_time);
    let edit = f.begin_edit(Role::Owner, DraftId::new(1), changes).unwrap();
    entry.open_edit(Role::Owner, edit).unwrap();
    let before = f.clone();
    assert_eq!(
        entry.save_edit(Role::Owner, &mut f),
        Err(ConfirmationError::Overflow)
    );
    assert_eq!(f, before);
    assert_eq!(entry.preview(), None);
    check_counter(&f, today, &[4, 3, 5, 2, 1]);
    let edit = f.begin_edit(Role::Owner, DraftId::new(3), changes).unwrap();
    entry.open_edit(Role::Owner, edit).unwrap();
    let saved = entry.save_edit(Role::Owner, &mut f).unwrap();
    assert_eq!(saved.invoice, 3);
    assert_eq!(entry.preview(), Some(&saved));
    check_counter(&f, today, &[4, 3, 5, 2, 1]);
}
