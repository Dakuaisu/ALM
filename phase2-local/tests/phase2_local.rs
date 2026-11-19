use std::num::NonZeroU32;

use alm_phase2_local::{
    AccessDenied, DraftId, DraftLine, LocalSales, Money, ProductId, ProvisionalDraft, Role,
};

#[test]
fn owner_sees_exact_synthetic_total_profit() {
    // Given: a local service with a synthetic total and an Owner session.
    let total = Money::from_minor_units(12_345);
    let mut sales = LocalSales::new(total);
    let owner = sales.session(Role::Owner);

    // When: the Owner reads total profit.
    let result = owner.total_profit();

    // Then: the exact synthetic Money value is returned.
    assert_eq!(result, Ok(total));
}

#[test]
fn manager_cannot_read_total_profit_after_owner_read_on_same_service() {
    // Given: an Owner and a Manager sharing one local service.
    let mut sales = LocalSales::new(Money::from_minor_units(54_321));
    assert_eq!(
        sales.session(Role::Owner).total_profit(),
        Ok(Money::from_minor_units(54_321))
    );
    let manager = sales.session(Role::Manager);

    // When: the Manager reads total profit after the Owner's read.
    let result = manager.total_profit();

    // Then: access remains denied for the Manager.
    assert_eq!(result, Err(AccessDenied));
}

#[test]
fn owner_can_save_reopen_and_edit_provisional_draft() {
    // Given: an Owner session and a provisional draft with one line.
    let mut sales = LocalSales::new(Money::from_minor_units(100));
    let mut owner = sales.session(Role::Owner);
    let draft_id = DraftId::new(1);
    let line = DraftLine::new(
        ProductId::new(7),
        NonZeroU32::new(2).expect("two is non-zero"),
    );
    let draft = ProvisionalDraft::new(draft_id, vec![line]);

    // When: the Owner saves, reopens, and edits the draft.
    owner.save_draft(draft);
    let reopened = owner.reopen_draft(draft_id).expect("saved draft reopens");
    let updated = DraftLine::new(reopened.lines()[0].product_id(), NonZeroU32::MIN);
    owner.save_draft(ProvisionalDraft::new(draft_id, vec![updated]));
    let reopened = owner.reopen_draft(draft_id).expect("edited draft reopens");

    // Then: the reopened draft retains its identity and line data.
    assert_eq!(reopened.id(), draft_id);
    assert_eq!(reopened.lines().len(), 1);
    assert_eq!(reopened.lines()[0].product_id(), ProductId::new(7));
    assert_eq!(reopened.lines()[0].quantity(), NonZeroU32::MIN);
}

#[test]
fn manager_can_save_reopen_and_edit_provisional_draft() {
    // Given: a Manager session and a provisional draft with one line.
    let mut sales = LocalSales::new(Money::from_minor_units(100));
    let mut manager = sales.session(Role::Manager);
    let draft_id = DraftId::new(2);
    let line = DraftLine::new(
        ProductId::new(8),
        NonZeroU32::new(3).expect("three is non-zero"),
    );
    let draft = ProvisionalDraft::new(draft_id, vec![line]);

    // When: the Manager saves, reopens, and edits the draft.
    manager.save_draft(draft);
    let reopened = manager.reopen_draft(draft_id).expect("saved draft reopens");
    let updated = DraftLine::new(reopened.lines()[0].product_id(), NonZeroU32::MIN);
    manager.save_draft(ProvisionalDraft::new(draft_id, vec![updated]));
    let reopened = manager
        .reopen_draft(draft_id)
        .expect("edited draft reopens");

    // Then: the reopened draft retains its identity and line data.
    assert_eq!(reopened.id(), draft_id);
    assert_eq!(reopened.lines().len(), 1);
    assert_eq!(reopened.lines()[0].product_id(), ProductId::new(8));
    assert_eq!(reopened.lines()[0].quantity(), NonZeroU32::MIN);
}
