//! Provisional pricing for the fixed synthetic sales catalog.

use std::num::NonZeroU32;

use crate::bill_math::{ExactGstError, checked_line_amount, exact_intra_half, exact_taxable};
use crate::{Money, ProductId, ProvisionalDraft};

pub(crate) const LIME_ID: ProductId = ProductId::new(1);
pub(crate) const LIME_NAME: &str = "Synthetic Lime";
pub(crate) const LIME_INCLUSIVE_PRICE: Money = Money::from_minor_units(11_800);
pub(crate) const LIME_GST_BASIS_POINTS: u32 = 1_800;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PreviewAmounts {
    pub inclusive: Money,
    pub taxable: Money,
    pub cgst: Money,
    pub sgst: Money,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProvisionalPreviewLine {
    pub product_id: ProductId,
    pub name: &'static str,
    pub quantity: NonZeroU32,
    pub inclusive_unit_price: Money,
    pub gst_basis_points: u32,
    pub amounts: PreviewAmounts,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProvisionalBillPreview {
    pub lines: Vec<ProvisionalPreviewLine>,
    pub totals: PreviewAmounts,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProvisionalPreviewError {
    UnknownProduct(ProductId),
    UnsupportedCrateRow,
    UnsupportedRounding,
    Overflow,
}

impl From<ExactGstError> for ProvisionalPreviewError {
    fn from(error: ExactGstError) -> Self {
        match error {
            ExactGstError::UnsupportedRounding => Self::UnsupportedRounding,
            ExactGstError::Overflow => Self::Overflow,
        }
    }
}

/// Projects ordinary draft lines in order, with zero totals for an empty draft.
///
/// # Errors
/// Rejects unknown products, crate rows, and amounts that cannot be computed exactly.
pub fn preview(
    draft: &ProvisionalDraft,
) -> Result<ProvisionalBillPreview, ProvisionalPreviewError> {
    with_catalog(draft, |id| {
        (id == LIME_ID).then_some((LIME_NAME, LIME_INCLUSIVE_PRICE, LIME_GST_BASIS_POINTS))
    })
}

/// `totalPkts(items) * loadingRate`: the draft's loading charge over its ordinary lines.
///
/// # Errors
/// Rejects overflow.
pub fn loading_charge(draft: &ProvisionalDraft) -> Result<Money, ProvisionalPreviewError> {
    let packets = draft.lines().iter().try_fold(0_i64, |sum, line| {
        sum.checked_add(i64::from(line.quantity().get()))
            .ok_or(ProvisionalPreviewError::Overflow)
    })?;
    draft
        .loading_rate()
        .minor_units()
        .checked_mul(packets)
        .map(Money::from_minor_units)
        .ok_or(ProvisionalPreviewError::Overflow)
}

/// The loading charge and the bill amount (items total plus loading) for New Bill.
///
/// # Errors
/// Rejects overflow.
pub fn with_loading(
    draft: &ProvisionalDraft,
    totals: &PreviewAmounts,
) -> Result<(Money, Money), ProvisionalPreviewError> {
    let loading = loading_charge(draft)?;
    let grand = totals
        .inclusive
        .minor_units()
        .checked_add(loading.minor_units())
        .ok_or(ProvisionalPreviewError::Overflow)?;
    Ok((loading, Money::from_minor_units(grand)))
}

pub(crate) fn with_catalog(
    draft: &ProvisionalDraft,
    lookup: impl Fn(ProductId) -> Option<(&'static str, Money, u32)>,
) -> Result<ProvisionalBillPreview, ProvisionalPreviewError> {
    if draft.crate_row().is_some() {
        return Err(ProvisionalPreviewError::UnsupportedCrateRow);
    }

    let mut lines = Vec::with_capacity(draft.lines().len());
    let mut inclusive_total = 0_i64;
    let mut taxable_total = 0_i64;
    let mut half_tax_total = 0_i64;
    for line in draft.lines() {
        let product_id = line.product_id();
        let (name, rate, gst_basis_points) =
            lookup(product_id).ok_or(ProvisionalPreviewError::UnknownProduct(product_id))?;
        let quantity = line.quantity();
        let inclusive =
            checked_line_amount(rate, quantity.get()).ok_or(ProvisionalPreviewError::Overflow)?;
        let taxable = exact_taxable(inclusive, gst_basis_points)?;
        let tax = inclusive
            .minor_units()
            .checked_sub(taxable)
            .ok_or(ProvisionalPreviewError::Overflow)?;
        let half_tax = exact_intra_half(tax)?;

        inclusive_total = inclusive_total
            .checked_add(inclusive.minor_units())
            .ok_or(ProvisionalPreviewError::Overflow)?;
        taxable_total = taxable_total
            .checked_add(taxable)
            .ok_or(ProvisionalPreviewError::Overflow)?;
        half_tax_total = half_tax_total
            .checked_add(half_tax)
            .ok_or(ProvisionalPreviewError::Overflow)?;
        lines.push(ProvisionalPreviewLine {
            product_id,
            name,
            quantity,
            inclusive_unit_price: rate,
            gst_basis_points,
            amounts: PreviewAmounts {
                inclusive,
                taxable: Money::from_minor_units(taxable),
                cgst: Money::from_minor_units(half_tax),
                sgst: Money::from_minor_units(half_tax),
            },
        });
    }

    Ok(ProvisionalBillPreview {
        lines,
        totals: PreviewAmounts {
            inclusive: Money::from_minor_units(inclusive_total),
            taxable: Money::from_minor_units(taxable_total),
            cgst: Money::from_minor_units(half_tax_total),
            sgst: Money::from_minor_units(half_tax_total),
        },
    })
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use super::{PreviewAmounts, ProvisionalBillPreview, ProvisionalPreviewError, preview};
    use crate::{CrateRow, DraftId, DraftLine, Money, ProductId, ProvisionalDraft};

    #[test]
    fn previews_three_limes_with_exact_intrastate_gst() {
        // Given a draft with no customer or bill date.
        let product = ProductId::new(1);
        let quantity = NonZeroU32::new(3).unwrap();
        let draft = ProvisionalDraft::new(DraftId::new(7), vec![DraftLine::new(product, quantity)]);

        // When the ordinary draft is previewed.
        let ProvisionalBillPreview { lines, totals } = preview(&draft).unwrap();

        // Then the line and bill totals contain the exact synthetic sell amounts.
        let [line] = lines.as_slice() else {
            panic!("expected one preview line");
        };
        assert_eq!(line.product_id, product);
        assert_eq!(line.name, "Synthetic Lime");
        assert_eq!(line.quantity, quantity);
        assert_eq!(line.inclusive_unit_price.minor_units(), 11_800);
        assert_eq!(line.gst_basis_points, 1_800);
        assert_eq!(line.amounts.inclusive.minor_units(), 35_400);
        assert_eq!(line.amounts.taxable.minor_units(), 30_000);
        assert_eq!(line.amounts.cgst.minor_units(), 2_700);
        assert_eq!(line.amounts.sgst.minor_units(), 2_700);
        assert_eq!(totals, line.amounts);
    }

    #[test]
    fn preserves_and_sums_repeated_lines() {
        // Given two separate lines for the same catalog product.
        let product = ProductId::new(1);
        let quantities = [NonZeroU32::new(2).unwrap(), NonZeroU32::MIN];
        let draft = ProvisionalDraft::new(
            DraftId::new(7),
            quantities.map(|qty| DraftLine::new(product, qty)).to_vec(),
        );

        // When the draft is previewed.
        let result = preview(&draft).unwrap();

        // Then neither line is merged or reordered, and both contribute to the totals.
        assert!(result.lines.iter().map(|line| line.quantity).eq(quantities));
        assert_eq!(result.totals.inclusive.minor_units(), 35_400);
        assert_eq!(result.totals.taxable.minor_units(), 30_000);
        assert_eq!(result.totals.cgst.minor_units(), 2_700);
        assert_eq!(result.totals.sgst.minor_units(), 2_700);
    }

    #[test]
    fn rejects_unknown_product_without_returning_a_partial_preview() {
        // Given a valid line followed by a product outside the fixed catalog.
        let unknown = ProductId::new(2);
        let draft = ProvisionalDraft::new(
            DraftId::new(7),
            vec![
                DraftLine::new(ProductId::new(1), NonZeroU32::MIN),
                DraftLine::new(unknown, NonZeroU32::MIN),
            ],
        );

        // When the draft is previewed.
        let result = preview(&draft);

        // Then it returns a typed failure rather than pricing only the known product.
        assert_eq!(
            result,
            Err(ProvisionalPreviewError::UnknownProduct(unknown))
        );
    }

    #[test]
    fn rejects_a_crate_row_instead_of_omitting_it() {
        // Given an ordinary draft that also has a crate row.
        let mut draft = ProvisionalDraft::new(
            DraftId::new(7),
            vec![DraftLine::new(ProductId::new(1), NonZeroU32::MIN)],
        );
        draft
            .add_crate_row(CrateRow::new(NonZeroU32::MIN, vec![]))
            .unwrap();

        // When the draft is previewed.
        let result = preview(&draft);

        // Then the unsupported row prevents a partial preview.
        assert_eq!(result, Err(ProvisionalPreviewError::UnsupportedCrateRow));
    }

    #[test]
    fn empty_draft_has_zero_totals() {
        // Given an empty draft that is still being edited.
        let draft = ProvisionalDraft::new(DraftId::new(7), vec![]);
        let zero = Money::from_minor_units(0);

        // When the draft is previewed.
        let result = preview(&draft).unwrap();

        // Then no amounts or lines are invented.
        assert_eq!(
            result,
            ProvisionalBillPreview {
                lines: vec![],
                totals: PreviewAmounts {
                    inclusive: zero,
                    taxable: zero,
                    cgst: zero,
                    sgst: zero,
                },
            }
        );
    }
}
