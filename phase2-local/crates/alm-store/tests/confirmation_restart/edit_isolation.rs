use super::*;
use alm_store::DraftCustomerId;
use alm_store::confirmation::{EditChanges, Review};
use alm_store::draft_entry::{
    ActiveBillFacts, BillEntryState, DraftChoice, DraftEntry, DraftEntryAction, DraftEntryOutcome,
    DraftRowFacts,
};

#[test]
fn edit_entry_cancel_and_save_leave_new_bill_and_sqlite_unchanged() {
    let (mut f, _, _, mut changes) = setup();
    changes.appended_line = DraftLine::new(ProductId::new(2), NonZeroU32::new(2).unwrap());
    let active = draft(77, 1);
    let scratch = crate::Scratch::new();
    let path = scratch.0.join("draft.sqlite");
    let mut local = LocalSales::open_sqlite(&path, Money::from_minor_units(0)).unwrap();
    local
        .session(Role::Owner)
        .save_draft(active.clone())
        .unwrap();
    let rows = [DraftRowFacts::quantity(1.0)];
    let DraftEntry::AwaitingChoice(pending) = DraftEntryAction::NewBill.begin(Some((
        &active,
        ActiveBillFacts::new(&rows, "Synthetic", 0.0, ""),
    ))) else {
        panic!("pending new bill");
    };
    let DraftEntryOutcome::StayWithDraft(form) = pending.resolve(DraftChoice::Cancel) else {
        panic!("active form");
    };
    let mut entry = BillEntryState::default();
    entry.open(&form);
    let DraftEntry::AwaitingChoice(pending) = DraftEntryAction::NewBill.begin(Some((
        &active,
        ActiveBillFacts::new(&rows, "Synthetic", 0.0, ""),
    ))) else {
        panic!("pending new bill");
    };
    let edit = f.begin_edit(Role::Owner, DraftId::new(1), changes).unwrap();
    assert_eq!(
        entry.open_edit(Role::Manager, edit.clone()),
        Err(ConfirmationError::AccessDenied)
    );
    entry.open_edit(Role::Owner, edit.clone()).unwrap();
    let before = f.clone();
    entry.cancel_edit();
    assert_eq!(before, f);
    assert_eq!(entry.active_draft(), Some(&active));
    assert_eq!(entry.preview(), None);
    entry.open_edit(Role::Owner, edit).unwrap();
    assert_eq!(
        entry.save_edit(Role::Manager, &mut f),
        Err(ConfirmationError::AccessDenied)
    );
    let saved = entry.save_edit(Role::Owner, &mut f).unwrap();
    assert_eq!((saved.quantity, saved.total.minor_units()), (4, 480));
    assert_eq!(entry.preview(), Some(&saved));
    assert_eq!(entry.active_draft(), Some(&active));
    let DraftEntryOutcome::StayWithDraft(form) = pending.resolve(DraftChoice::Cancel) else {
        panic!("pending choice remains available");
    };
    assert_eq!(form.draft(), &active);
    assert_eq!(
        local
            .session(Role::Owner)
            .reopen_draft(active.id())
            .unwrap(),
        Some(active)
    );
}

fn setup() -> (ConfirmationFixture, Review, Receipt, EditChanges) {
    let mut f = ConfirmationFixture::with_ordered_lots(
        ProductId::new(1),
        &[(4, Money::from_minor_units(40))],
        Money::from_minor_units(125),
    )
    .unwrap();
    f.add_product(ProductId::new(2), 2, Money::from_minor_units(80))
        .unwrap();
    f.catalog
        .seed_lots(ProductId::new(2), &[(2, Money::from_minor_units(30))])
        .unwrap();
    f.register_draft(draft(1, 2).with_customer(DraftCustomer::Selected {
        id: DraftCustomerId::new("customer".into()),
        name: "Synthetic".into(),
    }))
    .unwrap();
    let review = f
        .review_named(Role::Owner, DraftId::new(1), Money::from_minor_units(100))
        .unwrap();
    let receipt = f.confirm(Role::Owner, RequestId::new(1), &review).unwrap();
    let changes = EditChanges {
        retained_price: Money::from_minor_units(150),
        appended_line: DraftLine::new(ProductId::new(2), NonZeroU32::new(3).unwrap()),
        appended_price: Money::from_minor_units(90),
    };
    (f, review, receipt, changes)
}
