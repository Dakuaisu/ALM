use std::num::NonZeroU32;

use super::{
    ActiveBillFacts, CustomerId, CustomerPrefill, DraftChoice, DraftEntry, DraftEntryAction,
    DraftEntryOutcome, DraftRowFacts, FreshBillStart, PendingDraftChoice,
};
use crate::{DraftId, DraftLine, ProductId, ProvisionalDraft};

mod typed_customer;

fn populated_draft() -> ProvisionalDraft {
    ProvisionalDraft::new(
        DraftId::new(1),
        vec![DraftLine::new(ProductId::new(2), NonZeroU32::MIN)],
    )
}

fn customer<'a>(id: &'a str, name: &'a str) -> CustomerPrefill<'a> {
    CustomerPrefill::new(CustomerId::new(id), name)
}

fn fresh_start(id: u64, customer: Option<CustomerPrefill<'_>>) -> FreshBillStart<'_> {
    FreshBillStart::new(DraftId::new(id), customer)
}

#[test]
fn new_bill_prefills_a_fresh_empty_draft_without_pending_work() {
    // Given no active draft and a requested customer distinct from any prior one.
    let start = fresh_start(20, Some(customer("cust-requested", "Requested Name")));

    // When New Bill is opened.
    let entry = DraftEntryAction::NewBill.begin_with_customer(None, start);

    // Then the new active draft is empty and carries the requested pair.
    let DraftEntry::Resolved(DraftEntryOutcome::OpenFreshWith(form)) = entry else {
        panic!("New Bill must open a fresh form");
    };
    assert!(form.draft().lines().is_empty());
    assert_eq!(form.draft().id(), DraftId::new(20));
    assert_eq!(
        form.customer(),
        Some(customer("cust-requested", "Requested Name"))
    );
    let selected = form.customer().expect("requested customer is selected");
    assert_eq!(selected.id().value(), "cust-requested");
    assert_eq!(selected.name(), "Requested Name");
}

#[test]
fn discard_prefills_a_fresh_empty_draft_with_the_requested_customer() {
    // Given existing work with a different customer from the requested one.
    let draft = populated_draft();
    let facts = ActiveBillFacts::blank()
        .with_selected_customer(customer("cust-existing", "Existing Customer"));
    let entry = DraftEntryAction::NewBill.begin_with_customer(
        Some((&draft, facts)),
        fresh_start(21, Some(customer("cust-new", "New Customer"))),
    );
    let DraftEntry::AwaitingChoice(pending) = entry else {
        panic!("work in progress must await a choice");
    };

    // When the existing draft is discarded.
    let outcome = pending.resolve(DraftChoice::Discard);

    // Then a new empty active draft carries the new ID/name pair.
    let DraftEntryOutcome::OpenFreshWith(form) = outcome else {
        panic!("Discard must open a fresh form");
    };
    assert!(form.draft().lines().is_empty());
    assert_eq!(form.draft().id(), DraftId::new(21));
    assert_eq!(form.customer(), Some(customer("cust-new", "New Customer")));
    let selected = form.customer().expect("requested customer is selected");
    assert_eq!(selected.id().value(), "cust-new");
    assert_eq!(selected.name(), "New Customer");
}

#[test]
fn cancel_and_continue_keep_existing_customer_on_the_same_draft() {
    // Given an active form for one customer and a different requested customer.
    let draft = populated_draft();
    let existing = customer("cust-existing", "Existing Customer");
    let facts = ActiveBillFacts::blank().with_selected_customer(existing);
    let pending = DraftEntryAction::NewBill.begin_with_customer(
        Some((&draft, facts)),
        fresh_start(22, Some(customer("cust-other", "Other Customer"))),
    );
    let DraftEntry::AwaitingChoice(pending) = pending else {
        panic!("work in progress must await a choice");
    };

    // When Cancel is selected.
    let canceled = pending.resolve(DraftChoice::Cancel);

    // Then the same draft and existing customer remain active.
    let DraftEntryOutcome::StayWithDraft(form) = canceled else {
        panic!("Cancel must stay with the existing form");
    };
    assert_eq!(form.draft(), &draft);
    assert_eq!(form.customer(), Some(existing));
    let selected = form.customer().expect("existing customer is retained");
    assert_eq!(selected.id().value(), "cust-existing");
    assert_eq!(selected.name(), "Existing Customer");

    // When Continue is selected from a separately prepared choice.
    let continued = DraftEntryAction::NewBill.begin_with_customer(
        Some((&draft, facts)),
        fresh_start(23, Some(customer("cust-other", "Other Customer"))),
    );
    let DraftEntry::AwaitingChoice(continued) = continued else {
        panic!("work in progress must await a choice");
    };
    let continued = continued.resolve(DraftChoice::Continue);

    // Then the same draft remains active with its existing customer.
    let DraftEntryOutcome::OpenExisting(form) = continued else {
        panic!("Continue must open the existing form");
    };
    assert_eq!(form.draft(), &draft);
    assert_eq!(form.customer(), Some(existing));
    let selected = form.customer().expect("existing customer is retained");
    assert_eq!(selected.id().value(), "cust-existing");
    assert_eq!(selected.name(), "Existing Customer");
}

#[test]
fn fresh_start_without_customer_constructs_empty_form() {
    // Given a fresh start with no selected customer.
    let start = fresh_start(24, None);

    // When the form is constructed.
    let form = start.into_form();

    // Then it has no customer and no bill lines.
    assert!(form.draft().lines().is_empty());
    assert_eq!(form.customer(), None);
}

#[test]
fn new_bill_without_customer_starts_an_empty_unassigned_form() {
    // Given no current draft and no requested customer.
    let start = fresh_start(25, None);

    // When New Bill is requested.
    let entry = DraftEntryAction::NewBill.begin_with_customer(None, start);

    // Then it opens an empty form with no customer assignment.
    let DraftEntry::Resolved(DraftEntryOutcome::OpenFreshWith(form)) = entry else {
        panic!("New Bill must open a fresh form");
    };
    assert!(form.draft().lines().is_empty());
    assert_eq!(form.customer(), None);
}

fn guarded_choice(draft: &ProvisionalDraft) -> PendingDraftChoice<'_> {
    const ROWS: [DraftRowFacts<'static>; 1] = [DraftRowFacts::quantity(1.0)];
    let DraftEntry::AwaitingChoice(pending) =
        DraftEntryAction::NewBill.begin(Some((draft, ActiveBillFacts::new(&ROWS, "", 0.0, ""))))
    else {
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
    let DraftEntryOutcome::StayWithDraft(form) = outcome else {
        panic!("Cancel must stay with the existing form");
    };
    assert_eq!(form.draft(), &draft);
    assert_eq!(form.customer(), None);
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
    let DraftEntryOutcome::OpenExisting(form) = outcome else {
        panic!("Continue must open the existing form");
    };
    assert_eq!(form.draft(), &draft);
    assert_eq!(form.customer(), None);
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
    let rows = [DraftRowFacts::crate_row()];

    // When Fresh Bill is requested directly.
    let entry =
        DraftEntryAction::FreshBill.begin(Some((&draft, ActiveBillFacts::new(&rows, "", 0.0, ""))));

    // Then it requests an active-draft reset without a choice.
    assert_eq!(entry, DraftEntry::Resolved(DraftEntryOutcome::OpenFresh));
}

#[test]
fn clear_bypasses_the_guard_with_work_in_progress() {
    // Given a populated active draft.
    let draft = populated_draft();
    let rows = [DraftRowFacts::crate_row()];

    // When Clear is requested directly.
    let entry =
        DraftEntryAction::Clear.begin(Some((&draft, ActiveBillFacts::new(&rows, "", 0.0, ""))));

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
