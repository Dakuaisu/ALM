//! Synthetic L03-04 transitions for the active provisional draft only.
//! Draft construction (including defaults and customer prefill) is out of scope.

use crate::ProvisionalDraft;

/// Matches the text check in the frozen HTML's `draftHasWork` (L3369-3370).
fn has_form_text(text: &str) -> bool {
    !text
        .trim_matches(|ch: char| ch == '\u{FEFF}' || (ch != '\u{0085}' && ch.is_whitespace()))
        .is_empty()
}

/// Incomplete row values read from the active bill form.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DraftRowFacts<'a> {
    is_crate: bool,
    item: &'a str,
    quantity: f64,
}

impl<'a> DraftRowFacts<'a> {
    pub const fn crate_row() -> Self {
        Self {
            is_crate: true,
            item: "",
            quantity: 0.0,
        }
    }

    pub const fn item(item: &'a str) -> Self {
        Self {
            is_crate: false,
            item,
            quantity: 0.0,
        }
    }

    pub const fn quantity(quantity: f64) -> Self {
        Self {
            is_crate: false,
            item: "",
            quantity,
        }
    }

    fn has_work(self) -> bool {
        self.is_crate
            || has_form_text(self.item)
            || (self.quantity != 0.0 && !self.quantity.is_nan())
    }
}

/// Active form facts, separate from finalized provisional draft lines.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActiveBillFacts<'a> {
    rows: &'a [DraftRowFacts<'a>],
    customer: &'a str,
    paid: f64,
    note: &'a str,
}

impl<'a> ActiveBillFacts<'a> {
    pub const fn blank() -> Self {
        Self::new(&[], "", 0.0, "")
    }

    pub const fn new(
        rows: &'a [DraftRowFacts<'a>],
        customer: &'a str,
        paid: f64,
        note: &'a str,
    ) -> Self {
        Self {
            rows,
            customer,
            paid,
            note,
        }
    }

    fn has_work(self) -> bool {
        self.rows.iter().copied().any(DraftRowFacts::has_work)
            || has_form_text(self.customer)
            || (self.paid != 0.0 && !self.paid.is_nan())
            || has_form_text(self.note)
    }
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
    /// Resolves direct actions immediately; New Bill derives its guard from active form facts.
    pub fn begin<'a>(
        self,
        active: Option<(&'a ProvisionalDraft, ActiveBillFacts<'_>)>,
    ) -> DraftEntry<'a> {
        match self {
            Self::NewBill => match active {
                Some((draft, facts)) if facts.has_work() => {
                    DraftEntry::AwaitingChoice(PendingDraftChoice(draft))
                }
                Some(_) | None => DraftEntry::Resolved(DraftEntryOutcome::OpenFresh),
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
mod tests;
