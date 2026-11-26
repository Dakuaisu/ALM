//! New Bill's one pricing result (`lineAmt` per row, `subtotal`, `totalPkts × loadingRate`):
//! every row amount, the loading charge and the bill amount New Bill shows come from here.

use crate::provisional_preview::{ProvisionalBillPreview, ProvisionalPreviewError};
use crate::{Money, ProvisionalDraft};

use super::super::{ConfirmationError, ConfirmationFixture, ProposedPayment, ReceiptMode, Review};
use super::BillingDirectory;

/// One crate row as New Bill shows it: its rate and `rate × crates`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NewBillCrate {
    pub rate: Money,
    pub amount: Money,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NewBillPricing {
    /// The ordinary lines in draft order, with their GST breakdown.
    pub items: ProvisionalBillPreview,
    /// The crate rows in draft order.
    pub crates: Vec<NewBillCrate>,
    /// Packets charged for loading: the lines' quantities and the crates' component packets.
    pub packets: u32,
    /// Every row's amount added up (`subtotal`).
    pub items_total: Money,
    pub loading: Money,
    pub total: Money,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NewBillPricingError {
    Items(ProvisionalPreviewError),
    Crates(ConfirmationError),
}

impl From<ProvisionalPreviewError> for NewBillPricingError {
    fn from(error: ProvisionalPreviewError) -> Self {
        Self::Items(error)
    }
}

impl From<ConfirmationError> for NewBillPricingError {
    fn from(error: ConfirmationError) -> Self {
        Self::Crates(error)
    }
}

impl BillingDirectory {
    /// Prices a New Bill draft: ordinary lines at the provisional rates (the directory's, or a
    /// Manager's pulled catalog), crate rows at their copied rate (else the authority's crate
    /// rate), loading at the draft's rate per packet; the total is the rows plus loading.
    ///
    /// # Errors
    /// Rejects unknown products, crate components without a pack or not dividing into whole
    /// packets, and overflow.
    pub fn new_bill_pricing(
        &self,
        authority: &ConfirmationFixture,
        catalog: Option<&[crate::projection::ProjectedProduct]>,
        draft: &ProvisionalDraft,
    ) -> Result<NewBillPricing, NewBillPricingError> {
        let ordinary = ProvisionalDraft::new(draft.id(), draft.lines().to_vec());
        let items = match catalog {
            Some(catalog) => self.catalog_preview(catalog, &ordinary)?,
            None => self.preview(authority, &ordinary)?,
        };
        let mut crates_only = ProvisionalDraft::new(draft.id(), Vec::new());
        for row in draft.crate_rows() {
            crates_only.add_crate_row(row.clone());
        }
        let (rates, packs) = authority.crate_terms(&crates_only)?;
        let review = Review {
            draft: crates_only,
            draft_revision: 0,
            terms_revision: 0,
            prices: rates.clone(),
            payment: ProposedPayment::Legacy,
            payment_mode: ReceiptMode::Cash,
            checkout: None,
            crate_packs: (!rates.is_empty()).then_some(packs),
        };
        let (crate_packets, _) = review.crate_totals()?;
        let mut crates = Vec::with_capacity(rates.len());
        for (row, rate) in draft.crate_rows().iter().zip(rates) {
            let amount = rate
                .minor_units()
                .checked_mul(i64::from(row.quantity().get()))
                .ok_or(ConfirmationError::Overflow)?;
            crates.push(NewBillCrate {
                rate,
                amount: Money::from_minor_units(amount),
            });
        }
        let line_packets = draft.lines().iter().try_fold(0_u32, |sum, line| {
            sum.checked_add(line.quantity().get())
                .ok_or(ConfirmationError::Overflow)
        })?;
        let packets = line_packets
            .checked_add(crate_packets)
            .ok_or(ConfirmationError::Overflow)?;
        let items_total =
            crates
                .iter()
                .try_fold(items.totals.inclusive.minor_units(), |sum, row| {
                    sum.checked_add(row.amount.minor_units())
                        .ok_or(ConfirmationError::Overflow)
                })?;
        let loading = draft
            .loading_rate()
            .minor_units()
            .checked_mul(i64::from(packets))
            .ok_or(ConfirmationError::Overflow)?;
        let total = items_total
            .checked_add(loading)
            .ok_or(ConfirmationError::Overflow)?;
        Ok(NewBillPricing {
            items,
            crates,
            packets,
            items_total: Money::from_minor_units(items_total),
            loading: Money::from_minor_units(loading),
            total: Money::from_minor_units(total),
        })
    }
}
