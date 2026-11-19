use std::num::NonZeroU32;

use super::{
    ActiveBillFacts, DraftChoice, DraftEntry, DraftEntryAction, DraftEntryOutcome, DraftRowFacts,
    PendingDraftChoice,
};
use crate::{DraftId, DraftLine, ProductId, ProvisionalDraft};

fn populated_draft() -> ProvisionalDraft {
    ProvisionalDraft::new(
        DraftId::new(1),
        vec![DraftLine::new(ProductId::new(2), NonZeroU32::MIN)],
    )
}

fn guarded_choice(draft: &ProvisionalDraft) -> PendingDraftChoice<'_> {
    let DraftEntry::AwaitingChoice(pending) = DraftEntryAction::NewBill.begin(Some((
        draft,
        ActiveBillFacts::new(&[DraftRowFacts::quantity(1.0)], "", 0.0, ""),
    ))) else {
        panic!("New Bill must await a choice when work is in progress");
    };
    pending
}

#[test]
fn cancel_keeps_the_current_location_and_draft() {
    // Given a populated draft and a pending choice.
    let draft = populated_draft();
    let pending = guarded_choice(&draft);

    // When the entry is canceled.
    let outcome = pending.resolve(DraftChoice::Cancel);

    // Then neither the location nor draft changes.
    assert_eq!(outcome, DraftEntryOutcome::StayWithDraft(&draft));
}

#[test]
fn discard_opens_a_fresh_provisional_draft() {
    // Given a populated draft and a pending choice.
    let draft = populated_draft();
    let pending = guarded_choice(&draft);

    // When the active draft is discarded.
    let outcome = pending.resolve(DraftChoice::Discard);

    // Then bill entry requests a fresh active draft only.
    assert_eq!(outcome, DraftEntryOutcome::OpenFresh);
}

#[test]
fn continue_opens_the_same_provisional_draft() {
    // Given a populated draft and a pending choice.
    let draft = populated_draft();
    let pending = guarded_choice(&draft);

    // When the draft is continued.
    let outcome = pending.resolve(DraftChoice::Continue);

    // Then bill entry opens the existing draft unchanged.
    assert_eq!(outcome, DraftEntryOutcome::OpenExisting(&draft));
}

#[test]
fn new_bill_without_pending_work_opens_fresh_directly() {
    // Given no qualifying work.
    // When New Bill is requested.
    let entry = DraftEntryAction::NewBill.begin(None);

    // Then bill entry requests a fresh draft without a choice.
    assert_eq!(entry, DraftEntry::Resolved(DraftEntryOutcome::OpenFresh));
}

#[test]
fn new_bill_can_be_guarded_even_without_draft_lines() {
    // Given qualifying work outside the synthetic draft's line data.
    let draft = ProvisionalDraft::new(DraftId::new(1), Vec::new());
    let rows = [DraftRowFacts::crate_row()];
    // When New Bill is requested.
    let entry =
        DraftEntryAction::NewBill.begin(Some((&draft, ActiveBillFacts::new(&rows, "", 0.0, ""))));

    // Then it still awaits a choice.
    assert!(matches!(entry, DraftEntry::AwaitingChoice(_)));
}

#[test]
fn fresh_bill_bypasses_the_guard_with_work_in_progress() {
    // Given a populated active draft.
    let draft = populated_draft();

    // When Fresh Bill is requested directly.
    let entry = DraftEntryAction::FreshBill.begin(Some((
        &draft,
        ActiveBillFacts::new(&[DraftRowFacts::crate_row()], "", 0.0, ""),
    )));

    // Then it requests an active-draft reset without a choice.
    assert_eq!(entry, DraftEntry::Resolved(DraftEntryOutcome::OpenFresh));
}

#[test]
fn clear_bypasses_the_guard_with_work_in_progress() {
    // Given a populated active draft.
    let draft = populated_draft();

    // When Clear is requested directly.
    let entry = DraftEntryAction::Clear.begin(Some((
        &draft,
        ActiveBillFacts::new(&[DraftRowFacts::crate_row()], "", 0.0, ""),
    )));

    // Then it requests the same active-draft reset as Fresh Bill.
    assert_eq!(entry, DraftEntry::Resolved(DraftEntryOutcome::OpenFresh));
}

fn awaits_choice(facts: ActiveBillFacts<'_>) -> bool {
    let draft = ProvisionalDraft::new(DraftId::new(1), Vec::new());
    matches!(
        DraftEntryAction::NewBill.begin(Some((&draft, facts))),
        DraftEntry::AwaitingChoice(_)
    )
}

#[test]
fn blank_active_form_without_lines_does_not_guard_new_bill() {
    // Given an active but blank form with no finalized draft lines.
    let draft = ProvisionalDraft::new(DraftId::new(1), Vec::new());
    let facts = ActiveBillFacts::new(&[], "\u{FEFF}", 0.0, "\u{00A0}");

    // When New Bill is requested.
    let entry = DraftEntryAction::NewBill.begin(Some((&draft, facts)));

    // Then it opens fresh directly without a choice.
    assert_eq!(entry, DraftEntry::Resolved(DraftEntryOutcome::OpenFresh));
}

#[test]
fn crate_row_without_lines_guards_new_bill() {
    // Given an incomplete crate row and no finalized draft lines.
    let guarded = awaits_choice(ActiveBillFacts::new(
        &[DraftRowFacts::crate_row()],
        "",
        0.0,
        "",
    ));

    // When New Bill is requested.
    // Then the active draft is preserved for a choice.
    assert!(guarded);
}

#[test]
fn trimmed_item_without_lines_guards_new_bill() {
    // Given an incomplete row with trimmed nonempty item text.
    let guarded = awaits_choice(ActiveBillFacts::new(
        &[DraftRowFacts::item(" item ")],
        "",
        0.0,
        "",
    ));

    // When New Bill is requested.
    // Then the active draft is preserved for a choice.
    assert!(guarded);
}

#[test]
fn nonzero_numeric_quantity_without_lines_guards_new_bill() {
    // Given an incomplete row with a negative fractional numeric quantity.
    let guarded = awaits_choice(ActiveBillFacts::new(
        &[DraftRowFacts::quantity(-0.5)],
        "",
        0.0,
        "",
    ));

    // When New Bill is requested.
    // Then the active draft is preserved for a choice.
    assert!(guarded);
}

#[test]
fn zero_and_nan_quantities_without_other_work_do_not_guard() {
    // Given incomplete rows with numeric zero and normalized nonnumeric NaN quantities.
    let rows = [
        DraftRowFacts::quantity(0.0),
        DraftRowFacts::quantity(f64::NAN),
    ];
    let facts = ActiveBillFacts::new(&rows, "", 0.0, "");

    // When New Bill is requested.
    let draft = ProvisionalDraft::new(DraftId::new(1), Vec::new());
    let entry = DraftEntryAction::NewBill.begin(Some((&draft, facts)));

    // Then it opens fresh directly without a choice.
    assert_eq!(entry, DraftEntry::Resolved(DraftEntryOutcome::OpenFresh));
}

#[test]
fn trimmed_customer_without_lines_guards_new_bill() {
    // Given a trimmed nonempty customer value and no finalized lines.
    let guarded = awaits_choice(ActiveBillFacts::new(&[], " customer ", 0.0, ""));

    // When New Bill is requested.
    // Then the active draft is preserved for a choice.
    assert!(guarded);
}

#[test]
fn trimmed_note_without_lines_guards_new_bill() {
    // Given a note that JavaScript trim does not strip.
    let facts = ActiveBillFacts::new(&[], "", 0.0, "\u{0085}");

    // When New Bill is requested.
    let guarded = awaits_choice(facts);

    // Then the active draft is preserved for a choice.
    assert!(guarded);
}

#[test]
fn nonzero_paid_value_without_lines_guards_new_bill() {
    // Given a negative fractional paid form value and no finalized lines.
    let guarded = awaits_choice(ActiveBillFacts::new(&[], "", -0.25, ""));

    // When New Bill is requested.
    // Then the active draft is preserved for a choice.
    assert!(guarded);
}
