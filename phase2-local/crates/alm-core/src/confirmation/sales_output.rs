use super::estimate::estimate_html;
use super::{
    BalanceMode, ConfirmationError, ConfirmationFixture, EstimateDisplay, EstimateError,
    EstimateRequest, ImageHandoffRequest, InvoiceHandoff, InvoiceHandoffRequest,
    InvoiceImageAttempts,
};
use crate::{DraftCustomer, DraftId, Role};

/// Synthetic output inputs; contacts are resolved against the selected posted sale.
#[derive(Clone, Copy)]
pub struct SalesOutput<'a> {
    pub business_name: &'a str,
    pub tagline: &'a str,
    pub balance_mode: BalanceMode,
    pub display: EstimateDisplay<'a>,
    pub contacts: &'a [(&'a str, &'a str)],
}

#[derive(Clone, Copy)]
pub struct SalesOutputRow<'a> {
    fixture: &'a ConfirmationFixture,
    id: DraftId,
}

pub struct SalesPreview<'f, 'a> {
    row: SalesOutputRow<'f>,
    output: SalesOutput<'a>,
    html: String,
}

impl ConfirmationFixture {
    /// Selects a stored Sales row without exposing unposted drafts.
    pub fn sales_output_row(
        &self,
        role: Role,
        id: DraftId,
    ) -> Result<SalesOutputRow<'_>, ConfirmationError> {
        if role != Role::Owner {
            return Err(ConfirmationError::AccessDenied);
        }
        self.current_sales
            .get(&id)
            .ok_or(ConfirmationError::DraftNotFound)?;
        Ok(SalesOutputRow { fixture: self, id })
    }
}

impl<'f> SalesOutputRow<'f> {
    /// Reopens the selected stored sale using the existing estimate markup.
    pub fn view<'a>(&self, output: SalesOutput<'a>) -> Result<SalesPreview<'f, 'a>, EstimateError> {
        let request = EstimateRequest {
            role: Role::Owner,
            draft_id: self.id,
            balance_mode: output.balance_mode,
            display: output.display,
        };
        let preview = self.fixture.preview_estimate(request)?;
        Ok(SalesPreview {
            row: *self,
            output,
            html: estimate_html(&preview, output.display, None),
        })
    }

    /// The direct row action exists only for active sales.
    pub fn whatsapp<'a>(
        &self,
        output: SalesOutput<'_>,
        attempts: InvoiceImageAttempts<
            impl FnOnce() -> Option<&'a [u8]>,
            impl FnOnce() -> Option<&'a [u8]>,
        >,
        sinks: &mut impl InvoiceHandoff,
    ) -> Result<(), EstimateError> {
        let cancelled = self
            .fixture
            .lifecycle
            .get(&self.id)
            .ok_or(EstimateError::Preview(ConfirmationError::DraftNotFound))?
            .0;
        if cancelled {
            return Err(EstimateError::Preview(ConfirmationError::InvalidDraft));
        }
        self.handoff(output, attempts, sinks)
    }

    fn handoff<'a>(
        &self,
        output: SalesOutput<'_>,
        attempts: InvoiceImageAttempts<
            impl FnOnce() -> Option<&'a [u8]>,
            impl FnOnce() -> Option<&'a [u8]>,
        >,
        sinks: &mut impl InvoiceHandoff,
    ) -> Result<(), EstimateError> {
        let (review, _, _) = self
            .fixture
            .current_sales
            .get(&self.id)
            .ok_or(EstimateError::Preview(ConfirmationError::DraftNotFound))?;
        let contact = match review.draft.customer() {
            DraftCustomer::Selected { id, .. } => output
                .contacts
                .iter()
                .copied()
                .find(|(other, _)| *other == id.value()),
            DraftCustomer::Typed(_) | DraftCustomer::Unassigned => None,
        };
        self.fixture.handoff_invoice(
            InvoiceHandoffRequest {
                image: ImageHandoffRequest {
                    role: Role::Owner,
                    draft_id: self.id,
                    business_name: output.business_name,
                    saved_customer_mobile: contact,
                },
                tagline: output.tagline,
                balance_mode: output.balance_mode,
                display: output.display,
            },
            attempts,
            sinks,
        )
    }
}

impl SalesPreview<'_, '_> {
    pub fn html(&self) -> &str {
        &self.html
    }

    /// Captures the preview control even when the stored sale is cancelled.
    pub fn whatsapp<'a>(
        &self,
        attempts: InvoiceImageAttempts<
            impl FnOnce() -> Option<&'a [u8]>,
            impl FnOnce() -> Option<&'a [u8]>,
        >,
        sinks: &mut impl InvoiceHandoff,
    ) -> Result<(), EstimateError> {
        self.row.handoff(self.output, attempts, sinks)
    }
}
