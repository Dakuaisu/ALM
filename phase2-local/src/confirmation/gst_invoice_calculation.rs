use std::num::NonZeroU32;

use super::{GstError, GstGroup, GstInvoice, GstJurisdiction, GstLine};
use crate::bill_math::{exact_intra_half, exact_taxable};
use crate::{BillDate, Money, ProductId};

pub(crate) struct GstCalculationHeader<'a> {
    pub invoice: u64,
    pub date: BillDate,
    pub customer: &'a str,
    pub cancelled: bool,
    pub jurisdiction: GstJurisdiction,
    pub line_count: usize,
    pub items_total: Money,
    pub loading: Money,
    pub total: Money,
}

pub(crate) struct GstResolvedLine<'a> {
    pub product: ProductId,
    pub name: &'a str,
    pub quantity: NonZeroU32,
    pub hsn: &'a str,
    pub rate: u32,
    pub cess: u32,
    pub price: Money,
    pub amount: Money,
}

pub(crate) fn calculate_gst_invoice<'a>(
    header: GstCalculationHeader<'_>,
    input: impl ExactSizeIterator<Item = Result<GstResolvedLine<'a>, GstError>>,
) -> Result<GstInvoice, GstError> {
    if header.line_count != input.len() || header.loading.minor_units() != 0 {
        return Err(GstError::InvalidMetadata);
    }
    if header.total.minor_units() % 100 != 0 {
        return Err(GstError::UnsupportedRounding);
    }
    let mut lines = Vec::with_capacity(header.line_count);
    let mut groups: Vec<GstGroup> = Vec::new();
    let mut taxable_total = 0_i64;
    for line in input {
        let line = line?;
        let combined = line.rate.checked_add(line.cess).ok_or(GstError::Overflow)?;
        let taxable = exact_taxable(line.amount, combined)?;
        let taxable_rate = exact_taxable(line.price, combined)?;
        taxable_total = taxable_total
            .checked_add(taxable)
            .ok_or(GstError::Overflow)?;
        let tax = line
            .amount
            .minor_units()
            .checked_sub(taxable)
            .ok_or(GstError::Overflow)?;
        if let Some(group) = groups
            .iter_mut()
            .find(|g| g.hsn == line.hsn && g.combined_rate == combined)
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
                hsn: line.hsn.to_owned(),
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
            name: line.name.to_owned(),
            quantity: line.quantity,
            hsn: line.hsn.to_owned(),
            rate: line.rate,
            cess: line.cess,
            inclusive_rate: line.price,
            taxable_rate: Money::from_minor_units(taxable_rate),
            inclusive: line.amount,
            taxable: Money::from_minor_units(taxable),
        });
    }
    let tax = header
        .items_total
        .minor_units()
        .checked_sub(taxable_total)
        .ok_or(GstError::Overflow)?;
    // Legacy groups split the combined rate, including cess, as jurisdiction tax.
    for group in &mut groups {
        let (cgst, sgst, igst) = match header.jurisdiction {
            GstJurisdiction::Intra => {
                let half = exact_intra_half(group.tax.minor_units())?;
                (half, half, 0)
            }
            GstJurisdiction::Inter => (0, 0, group.tax.minor_units()),
        };
        group.cgst = Money::from_minor_units(cgst);
        group.sgst = Money::from_minor_units(sgst);
        group.igst = Money::from_minor_units(igst);
    }
    let (cgst, sgst, igst) = match header.jurisdiction {
        GstJurisdiction::Intra => {
            let half = exact_intra_half(tax)?;
            (half, half, 0)
        }
        GstJurisdiction::Inter => (0, 0, tax),
    };
    Ok(GstInvoice {
        invoice: header.invoice,
        date: header.date,
        customer: header.customer.to_owned(),
        cancelled: header.cancelled,
        jurisdiction: header.jurisdiction,
        lines,
        groups,
        taxable: Money::from_minor_units(taxable_total),
        tax: Money::from_minor_units(tax),
        cgst: Money::from_minor_units(cgst),
        sgst: Money::from_minor_units(sgst),
        igst: Money::from_minor_units(igst),
        total: header.total,
    })
}
