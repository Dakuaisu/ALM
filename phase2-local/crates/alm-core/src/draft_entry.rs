//! Synthetic L03-04 transitions for the active provisional draft only.
//! New-bill form construction stays separate from persisted draft state.

use crate::ProvisionalDraft;

mod snapshot;

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
    selected_customer: Option<CustomerPrefill<'a>>,
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
            selected_customer: None,
            paid,
            note,
        }
    }

    pub const fn with_selected_customer(mut self, customer: CustomerPrefill<'a>) -> Self {
        self.customer = customer.name();
        self.selected_customer = Some(customer);
        self
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
pub struct PendingDraftChoice<'a> {
    draft: &'a ProvisionalDraft,
    fresh_start: Option<FreshBillStart<'a>>,
    existing_customer: Option<CustomerPrefill<'a>>,
    existing_customer_name: &'a str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CustomerId<'a>(&'a str);

impl<'a> CustomerId<'a> {
    pub const fn new(value: &'a str) -> Self {
        Self(value)
    }

    pub const fn value(self) -> &'a str {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CustomerPrefill<'a> {
    id: CustomerId<'a>,
    name: &'a str,
}

impl<'a> CustomerPrefill<'a> {
    pub const fn new(id: CustomerId<'a>, name: &'a str) -> Self {
        Self { id, name }
    }

    pub const fn id(self) -> CustomerId<'a> {
        self.id
    }

    pub const fn name(self) -> &'a str {
        self.name
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FreshBillStart<'a> {
    draft: ProvisionalDraft,
    customer: Option<CustomerPrefill<'a>>,
}

impl<'a> FreshBillStart<'a> {
    pub fn new(draft_id: crate::DraftId, customer: Option<CustomerPrefill<'a>>) -> Self {
        Self {
            draft: ProvisionalDraft::new(draft_id, Vec::new()),
            customer,
        }
    }

    pub fn into_form(self) -> ActiveBillForm<'a> {
        ActiveBillForm {
            draft: ActiveDraft::Fresh(self.draft),
            customer: self.customer,
            customer_name: self.customer.map_or("", CustomerPrefill::name),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ActiveDraft<'a> {
    Existing(&'a ProvisionalDraft),
    Fresh(ProvisionalDraft),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActiveBillForm<'a> {
    draft: ActiveDraft<'a>,
    customer: Option<CustomerPrefill<'a>>,
    customer_name: &'a str,
}

impl<'a> ActiveBillForm<'a> {
    pub fn draft(&self) -> &ProvisionalDraft {
        match &self.draft {
            ActiveDraft::Existing(draft) => draft,
            ActiveDraft::Fresh(draft) => draft,
        }
    }

    pub const fn customer(&self) -> Option<CustomerPrefill<'a>> {
        self.customer
    }

    pub const fn customer_name(&self) -> &'a str {
        self.customer_name
    }
}

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
    StayWithDraft(ActiveBillForm<'a>),
    /// Open bill entry with the same active draft.
    OpenExisting(ActiveBillForm<'a>),
    /// Open bill entry with a fresh active draft, not a persisted deletion or posting.
    OpenFresh,
    /// Open a constructed empty form with its requested customer selection.
    OpenFreshWith(ActiveBillForm<'a>),
}

impl DraftEntryAction {
    /// Resolves direct actions immediately; New Bill derives its guard from active form facts.
    pub fn begin<'a>(
        self,
        active: Option<(&'a ProvisionalDraft, ActiveBillFacts<'a>)>,
    ) -> DraftEntry<'a> {
        match self {
            Self::NewBill => match active {
                Some((draft, facts)) if facts.has_work() => {
                    DraftEntry::AwaitingChoice(PendingDraftChoice {
                        draft,
                        fresh_start: None,
                        existing_customer: facts.selected_customer,
                        existing_customer_name: facts.customer,
                    })
                }
                Some(_) | None => DraftEntry::Resolved(DraftEntryOutcome::OpenFresh),
            },
            Self::FreshBill | Self::Clear => DraftEntry::Resolved(DraftEntryOutcome::OpenFresh),
        }
    }

    pub fn begin_with_customer<'a>(
        self,
        active: Option<(&'a ProvisionalDraft, ActiveBillFacts<'a>)>,
        fresh_start: FreshBillStart<'a>,
    ) -> DraftEntry<'a> {
        match self {
            Self::NewBill => match active {
                Some((draft, facts)) if facts.has_work() => {
                    DraftEntry::AwaitingChoice(PendingDraftChoice {
                        draft,
                        fresh_start: Some(fresh_start),
                        existing_customer: facts.selected_customer,
                        existing_customer_name: facts.customer,
                    })
                }
                Some(_) | None => {
                    DraftEntry::Resolved(DraftEntryOutcome::OpenFreshWith(fresh_start.into_form()))
                }
            },
            Self::FreshBill | Self::Clear => DraftEntry::Resolved(DraftEntryOutcome::OpenFresh),
        }
    }
}

impl<'a> PendingDraftChoice<'a> {
    pub fn resolve(self, choice: DraftChoice) -> DraftEntryOutcome<'a> {
        match choice {
            DraftChoice::Cancel => DraftEntryOutcome::StayWithDraft(ActiveBillForm {
                draft: ActiveDraft::Existing(self.draft),
                customer: self.existing_customer,
                customer_name: self.existing_customer_name,
            }),
            DraftChoice::Discard => match self.fresh_start {
                Some(start) => DraftEntryOutcome::OpenFreshWith(start.into_form()),
                None => DraftEntryOutcome::OpenFresh,
            },
            DraftChoice::Continue => DraftEntryOutcome::OpenExisting(ActiveBillForm {
                draft: ActiveDraft::Existing(self.draft),
                customer: self.existing_customer,
                customer_name: self.existing_customer_name,
            }),
        }
    }
}

mod confirmation;
pub use confirmation::{BillEntryState, ConfirmationAttempt, ConfirmationOutcome};

#[cfg(test)]
mod tests;
