//! Read-only synthetic ordinary-sale GST projection. Rates are basis points.
use super::gst_defaults::defaults;
use super::{BalanceMode, ConfirmationError, ConfirmationFixture, DraftId, Money, Role};
use crate::bill_math::ExactGstError;
use crate::{BillDate, ProductId};
use std::num::NonZeroU32;

#[path = "gst_invoice_calculation.rs"]
mod calculation;
pub(crate) use calculation::{GstCalculationHeader, GstResolvedLine, calculate_gst_invoice};

#[cfg(test)]
#[path = "gst_invoice_tests.rs"]
mod tests;

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum GstJurisdiction {
    Intra,
    Inter,
}

#[derive(Clone, Copy, Debug)]
pub struct GstLineMetadata<'a> {
    pub product: ProductId,
    pub name: &'a str,
    pub product_rate: Option<u32>,
    pub product_cess: u32,
    pub product_hsn: Option<&'a str>,
    pub line_rate: Option<u32>,
    pub line_hsn: Option<&'a str>,
}

#[derive(Clone, Copy, Debug)]
pub struct GstRequest<'a> {
    pub role: Role,
    pub draft_id: DraftId,
    /// Supplied by the caller after resolving seller and place-of-supply state.
    pub jurisdiction: GstJurisdiction,
    pub lines: &'a [GstLineMetadata<'a>],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GstError {
    Preview(ConfirmationError),
    InvalidMetadata,
    UnsupportedRounding,
    Overflow,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GstLine {
    pub product: ProductId,
    pub name: String,
    pub quantity: NonZeroU32,
    pub hsn: String,
    pub rate: u32,
    pub cess: u32,
    pub inclusive_rate: Money,
    pub taxable_rate: Money,
    pub inclusive: Money,
    pub taxable: Money,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GstGroup {
    pub hsn: String,
    pub combined_rate: u32,
    pub taxable: Money,
    pub tax: Money,
    pub cgst: Money,
    pub sgst: Money,
    pub igst: Money,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GstInvoice {
    pub invoice: u64,
    pub date: BillDate,
    pub customer: String,
    pub cancelled: bool,
    pub jurisdiction: GstJurisdiction,
    pub lines: Vec<GstLine>,
    pub groups: Vec<GstGroup>,
    pub taxable: Money,
    pub tax: Money,
    pub cgst: Money,
    pub sgst: Money,
    pub igst: Money,
    pub total: Money,
}

impl From<ExactGstError> for GstError {
    fn from(error: ExactGstError) -> Self {
        match error {
            ExactGstError::UnsupportedRounding => Self::UnsupportedRounding,
            ExactGstError::Overflow => Self::Overflow,
        }
    }
}

impl ConfirmationFixture {
    /// Emits a typed allowlisted projection only after every calculation succeeds.
    ///
    /// # Errors
    /// Rejects unauthorized access, unsupported rounding and mismatched metadata.
    pub fn emit_gst_invoice(
        &self,
        request: GstRequest<'_>,
        sink: &mut impl FnMut(GstInvoice),
    ) -> Result<(), GstError> {
        crate::permissions::require(request.role, crate::permissions::Capability::GstSales)
            .map_err(|_| GstError::Preview(ConfirmationError::AccessDenied))?;
        let preview = self
            .bill_preview(request.role, request.draft_id, BalanceMode::AsOnBillDate)
            .map_err(GstError::Preview)?;
        let header = GstCalculationHeader {
            invoice: preview.invoice,
            date: preview.date,
            customer: &preview.customer,
            cancelled: preview.cancelled,
            jurisdiction: request.jurisdiction,
            line_count: preview.lines.len(),
            items_total: preview.items_total,
            loading: preview.loading,
            total: preview.total,
        };
        // Check cardinality before zip truncates the input, as in the original path.
        if preview.lines.len() != request.lines.len() {
            return Err(GstError::InvalidMetadata);
        }
        let resolved = preview
            .lines
            .iter()
            .zip(request.lines)
            .map(|(line, metadata)| {
                if line.product != metadata.product || metadata.name.is_empty() {
                    return Err(GstError::InvalidMetadata);
                }
                let (default_rate, default_hsn) = defaults(metadata.name);
                let rate = metadata
                    .line_rate
                    .unwrap_or(metadata.product_rate.unwrap_or(default_rate));
                let cess = if metadata.line_rate.is_some() {
                    0
                } else {
                    metadata.product_cess
                };
                let hsn = metadata
                    .line_hsn
                    .filter(|s| !s.is_empty())
                    .or(metadata.product_hsn.filter(|s| !s.is_empty()))
                    .unwrap_or(default_hsn);
                Ok(GstResolvedLine {
                    product: line.product,
                    name: metadata.name,
                    quantity: line.quantity,
                    hsn,
                    rate,
                    cess,
                    price: line.price,
                    amount: line.amount,
                })
            });
        sink(calculate_gst_invoice(header, resolved)?);
        Ok(())
    }
}
