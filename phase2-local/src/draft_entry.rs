//! Synthetic L03-04 transitions for the active provisional draft only.
//! Draft construction (including defaults and customer prefill) is out of scope.

use crate::ProvisionalDraft;

/// Caller-supplied classification of active draft work. The source guard checks
/// more than the line data represented by [`ProvisionalDraft`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DraftWork<'a> {
    /// There is no active draft, or it has no qualifying work.
    None,
    InProgress(&'a ProvisionalDraft),
}

/// Guarded New Bill and the two direct actions from the bill form.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DraftEntryAction {
    NewBill,
    FreshBill,
    Clear,
}

#[derive(Debug, Eq, PartialEq)]
#[must_use]
pub enum DraftEntry<'a> {
    AwaitingChoice(PendingDraftChoice<'a>),
    Resolved(DraftEntryOutcome<'a>),
}

/// Only guarded New Bill entry with qualifying work can yield a pending choice.
#[derive(Debug, Eq, PartialEq)]
pub struct PendingDraftChoice<'a>(&'a ProvisionalDraft);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DraftChoice {
    Cancel,
    Discard,
    Continue,
}

#[derive(Debug, Eq, PartialEq)]
#[must_use]
pub enum DraftEntryOutcome<'a> {
    /// Keep the current location and the active draft unchanged.
    StayWithDraft(&'a ProvisionalDraft),
    /// Open bill entry with the same active draft.
    OpenExisting(&'a ProvisionalDraft),
    /// Open bill entry with a fresh active draft, not a persisted deletion or posting.
    OpenFresh,
}

impl DraftEntryAction {
    /// Resolves direct actions immediately; only New Bill consults the guard.
    pub const fn begin(self, work: DraftWork<'_>) -> DraftEntry<'_> {
        match self {
            Self::NewBill => match work {
                DraftWork::None => DraftEntry::Resolved(DraftEntryOutcome::OpenFresh),
                DraftWork::InProgress(draft) => {
                    DraftEntry::AwaitingChoice(PendingDraftChoice(draft))
                }
            },
            Self::FreshBill | Self::Clear => DraftEntry::Resolved(DraftEntryOutcome::OpenFresh),
        }
    }
}

impl<'a> PendingDraftChoice<'a> {
    pub const fn resolve(self, choice: DraftChoice) -> DraftEntryOutcome<'a> {
        match choice {
            DraftChoice::Cancel => DraftEntryOutcome::StayWithDraft(self.0),
            DraftChoice::Discard => DraftEntryOutcome::OpenFresh,
            DraftChoice::Continue => DraftEntryOutcome::OpenExisting(self.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use super::{
        DraftChoice, DraftEntry, DraftEntryAction, DraftEntryOutcome, DraftWork, PendingDraftChoice,
    };
    use crate::{DraftId, DraftLine, ProductId, ProvisionalDraft};

    fn populated_draft() -> ProvisionalDraft {
        ProvisionalDraft::new(
            DraftId::new(1),
            vec![DraftLine::new(ProductId::new(2), NonZeroU32::MIN)],
        )
    }

    fn guarded_choice(draft: &ProvisionalDraft) -> PendingDraftChoice<'_> {
        let DraftEntry::AwaitingChoice(pending) =
            DraftEntryAction::NewBill.begin(DraftWork::InProgress(draft))
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
        let work = DraftWork::None;

        // When New Bill is requested.
        let entry = DraftEntryAction::NewBill.begin(work);

        // Then bill entry requests a fresh draft without a choice.
        assert_eq!(entry, DraftEntry::Resolved(DraftEntryOutcome::OpenFresh));
    }

    #[test]
    fn new_bill_can_be_guarded_even_without_draft_lines() {
        // Given qualifying work outside the synthetic draft's line data.
        let draft = ProvisionalDraft::new(DraftId::new(1), Vec::new());
        let work = DraftWork::InProgress(&draft);

        // When New Bill is requested.
        let entry = DraftEntryAction::NewBill.begin(work);

        // Then it still awaits a choice.
        assert!(matches!(entry, DraftEntry::AwaitingChoice(_)));
    }

    #[test]
    fn fresh_bill_bypasses_the_guard_with_work_in_progress() {
        // Given a populated active draft.
        let draft = populated_draft();

        // When Fresh Bill is requested directly.
        let entry = DraftEntryAction::FreshBill.begin(DraftWork::InProgress(&draft));

        // Then it requests an active-draft reset without a choice.
        assert_eq!(entry, DraftEntry::Resolved(DraftEntryOutcome::OpenFresh));
    }

    #[test]
    fn clear_bypasses_the_guard_with_work_in_progress() {
        // Given a populated active draft.
        let draft = populated_draft();

        // When Clear is requested directly.
        let entry = DraftEntryAction::Clear.begin(DraftWork::InProgress(&draft));

        // Then it requests the same active-draft reset as Fresh Bill.
        assert_eq!(entry, DraftEntry::Resolved(DraftEntryOutcome::OpenFresh));
    }
}
