//! Read-only synthetic ordinary-sale GST projection. Rates are basis points.
use super::gst_defaults::defaults;
use super::{BalanceMode, ConfirmationError, ConfirmationFixture, DraftId, Money, Role};
use crate::{BillDate, ProductId};
use std::num::NonZeroU32;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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

fn exact(numerator: i64, multiplier: i64, divisor: i64) -> Result<i64, GstError> {
    let value = numerator
        .checked_mul(multiplier)
        .ok_or(GstError::Overflow)?;
    if value % divisor != 0 {
        return Err(GstError::UnsupportedRounding);
    }
    Ok(value / divisor)
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
        let preview = self
            .bill_preview(request.role, request.draft_id, BalanceMode::AsOnBillDate)
            .map_err(GstError::Preview)?;
        if preview.lines.len() != request.lines.len() || preview.loading.minor_units() != 0 {
            return Err(GstError::InvalidMetadata);
        }
        if preview.total.minor_units() % 100 != 0 {
            return Err(GstError::UnsupportedRounding);
        }
        let mut lines = Vec::with_capacity(preview.lines.len());
        let mut groups: Vec<GstGroup> = Vec::new();
        let mut taxable_total = 0_i64;
        for (line, metadata) in preview.lines.iter().zip(request.lines) {
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
            let combined = rate.checked_add(cess).ok_or(GstError::Overflow)?;
            let denominator = 10_000_i64 + i64::from(combined);
            let taxable = exact(line.amount.minor_units(), 10_000, denominator)?;
            let taxable_rate = exact(line.price.minor_units(), 10_000, denominator)?;
            taxable_total = taxable_total
                .checked_add(taxable)
                .ok_or(GstError::Overflow)?;
            let hsn = metadata
                .line_hsn
                .filter(|s| !s.is_empty())
                .or(metadata.product_hsn.filter(|s| !s.is_empty()))
                .unwrap_or(default_hsn);
            let tax = line
                .amount
                .minor_units()
                .checked_sub(taxable)
                .ok_or(GstError::Overflow)?;
            if let Some(group) = groups
                .iter_mut()
                .find(|g| g.hsn == hsn && g.combined_rate == combined)
            {
                group.taxable = Money::from_minor_units(
                    group
                        .taxable
                        .minor_units()
                        .checked_add(taxable)
                        .ok_or(GstError::Overflow)?,
                );
                group.tax = Money::from_minor_units(
                    group
                        .tax
                        .minor_units()
                        .checked_add(tax)
                        .ok_or(GstError::Overflow)?,
                );
            } else {
                groups.push(GstGroup {
                    hsn: hsn.to_owned(),
                    combined_rate: combined,
                    taxable: Money::from_minor_units(taxable),
                    tax: Money::from_minor_units(tax),
                    cgst: Money::from_minor_units(0),
                    sgst: Money::from_minor_units(0),
                    igst: Money::from_minor_units(0),
                });
            }
            lines.push(GstLine {
                product: line.product,
                name: metadata.name.to_owned(),
                quantity: line.quantity,
                hsn: hsn.to_owned(),
                rate,
                cess,
                inclusive_rate: line.price,
                taxable_rate: Money::from_minor_units(taxable_rate),
                inclusive: line.amount,
                taxable: Money::from_minor_units(taxable),
            });
        }
        let tax = preview
            .items_total
            .minor_units()
            .checked_sub(taxable_total)
            .ok_or(GstError::Overflow)?;
        // Legacy groups split the combined rate, including cess, as jurisdiction tax.
        for group in &mut groups {
            let (cgst, sgst, igst) = match request.jurisdiction {
                GstJurisdiction::Intra => {
                    let half = exact(group.tax.minor_units(), 1, 2)?;
                    (half, half, 0)
                }
                GstJurisdiction::Inter => (0, 0, group.tax.minor_units()),
            };
            group.cgst = Money::from_minor_units(cgst);
            group.sgst = Money::from_minor_units(sgst);
            group.igst = Money::from_minor_units(igst);
        }
        let (cgst, sgst, igst) = match request.jurisdiction {
            GstJurisdiction::Intra => {
                let half = exact(tax, 1, 2)?;
                (half, half, 0)
            }
            GstJurisdiction::Inter => (0, 0, tax),
        };
        sink(GstInvoice {
            invoice: preview.invoice,
            date: preview.date,
            customer: preview.customer,
            cancelled: preview.cancelled,
            jurisdiction: request.jurisdiction,
            lines,
            groups,
            taxable: Money::from_minor_units(taxable_total),
            tax: Money::from_minor_units(tax),
            cgst: Money::from_minor_units(cgst),
            sgst: Money::from_minor_units(sgst),
            igst: Money::from_minor_units(igst),
            total: preview.total,
        });
        Ok(())
    }
}
