use super::ActiveBillForm;
use crate::confirmation::{
    ConfirmationAck, ConfirmationError, PendingError, Receipt, RequestId, Review,
};
use crate::{ProvisionalDraft, Role, SalesSession};

/// In-memory bill entry and preview. Clearing entry does not delete any SQLite drafts
/// or pending approvals, and neither the active slot nor the preview survives restart.
#[derive(Default)]
pub struct BillEntryState {
    active: Option<ProvisionalDraft>,
    preview: Option<Receipt>,
}

pub enum ConfirmationAttempt<'a> {
    Submit {
        request: RequestId,
        review: &'a Review,
    },
    Retry {
        request: RequestId,
    },
}

#[derive(Debug, Eq, PartialEq)]
pub enum ConfirmationOutcome {
    Missing,
    Rejected(ConfirmationError),
    Uncorrelated,
    Acknowledged(Receipt),
}

impl BillEntryState {
    pub fn open(&mut self, form: &ActiveBillForm<'_>) {
        self.active = Some(form.snapshot());
    }

    pub const fn active_draft(&self) -> Option<&ProvisionalDraft> {
        self.active.as_ref()
    }

    pub const fn preview(&self) -> Option<&Receipt> {
        self.preview.as_ref()
    }

    /// Sends an explicit approval or retry. A matched authoritative reply changes the
    /// entry and preview only if the sent snapshot was the exact active draft for this attempt.
    pub fn confirm(
        &mut self,
        session: &mut SalesSession<'_>,
        attempt: ConfirmationAttempt<'_>,
        send: impl FnOnce(RequestId, &Review) -> Result<Option<ConfirmationAck>, ConfirmationError>,
    ) -> Result<ConfirmationOutcome, PendingError> {
        if session.role != Role::Owner {
            return Err(PendingError::AccessDenied);
        }
        match &attempt {
            ConfirmationAttempt::Submit { review, .. }
                if self.active_draft() != Some(review.draft()) =>
            {
                return Err(PendingError::ActiveDraftMismatch);
            }
            ConfirmationAttempt::Submit { .. } | ConfirmationAttempt::Retry { .. } => {}
        }
        let send = |request, review: &Review| {
            let bound_to_active = self.active.as_ref() == Some(review.draft());
            match send(request, review) {
                Ok(None) => ConfirmationOutcome::Missing,
                Err(error) => ConfirmationOutcome::Rejected(error),
                Ok(Some(ack)) => match ack.into_receipt_for(request, review) {
                    None => ConfirmationOutcome::Uncorrelated,
                    Some(receipt) => {
                        if bound_to_active {
                            self.active = None;
                            self.preview = Some(receipt.clone());
                        }
                        ConfirmationOutcome::Acknowledged(receipt)
                    }
                },
            }
        };
        match attempt {
            ConfirmationAttempt::Submit { request, review } => {
                session.confirm_pending(request, review, send)
            }
            ConfirmationAttempt::Retry { request } => session.retry_pending(request, send),
        }
    }
}
