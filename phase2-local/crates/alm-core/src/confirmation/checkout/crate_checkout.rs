use std::num::NonZeroU32;

use crate::{DraftLine, ProductId, SaleUnit};

use super::{
    CheckoutAmounts, CheckoutTerms, ConfirmationError, ConfirmationFixture, DraftId, Money,
    ProposedPayment, Review,
};

impl Review {
    /// A bill with crate rows for a named customer (`sell_crate`, or New Bill's crate rows beside
    /// ordinary lines): an ordinary bill for ledgers, the Cash Book, the Dashboard, the Sales
    /// list, edits (lines, crates and rates), cancellation and the bill image, as the HTML's
    /// crate lines.
    pub fn is_named_crate_sale(&self) -> bool {
        !self.draft.crate_rows().is_empty()
            && matches!(self.payment, ProposedPayment::Named(_))
            && matches!(self.draft.customer(), crate::DraftCustomer::Selected { .. })
    }

    /// The rate of crate row `index`, after the ordinary lines' prices.
    pub(in crate::confirmation) fn crate_rate(
        &self,
        index: usize,
    ) -> Result<Money, ConfirmationError> {
        if self.prices.len() != self.draft.lines().len() + self.draft.crate_rows().len() {
            return Err(ConfirmationError::InvalidDraft);
        }
        self.prices
            .get(self.draft.lines().len() + index)
            .copied()
            .ok_or(ConfirmationError::InvalidDraft)
    }

    /// Returns the packet demand of every crate component, crate rows in order and each row's
    /// components in their original order (`linePackets`), or `None` without crate rows.
    pub fn crate_packet_demands(&self) -> Result<Option<Vec<DraftLine>>, ConfirmationError> {
        let rows = self.draft.crate_rows();
        if rows.is_empty() {
            return Ok(None);
        }
        let packs = self
            .crate_packs
            .as_deref()
            .ok_or(ConfirmationError::InvalidDraft)?;
        let components = rows.iter().map(|row| row.components().len()).sum::<usize>();
        if rows.iter().any(|row| row.components().is_empty()) || components != packs.len() {
            return Err(ConfirmationError::InvalidDraft);
        }
        rows.iter()
            .flat_map(|row| row.components().iter().map(move |c| (row, c)))
            .zip(packs)
            .map(|((row, component), pack)| {
                let quantity =
                    u64::from(component.quantity.get()) * u64::from(row.quantity().get());
                let divisor = match component.unit {
                    SaleUnit::Piece => u64::from(pack.get()),
                    SaleUnit::Case => 1,
                };
                if !quantity.is_multiple_of(divisor) {
                    return Err(ConfirmationError::InvalidAmount);
                }
                let packets = u32::try_from(quantity / divisor)
                    .ok()
                    .and_then(NonZeroU32::new)
                    .ok_or(ConfirmationError::Overflow)?;
                Ok(DraftLine::new(component.product, packets))
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Some)
    }

    /// The crate rows' packets and amounts (`linePackets`, `lineAmt`): rate times crates.
    pub(in crate::confirmation) fn crate_totals(&self) -> Result<(u32, i64), ConfirmationError> {
        let Some(demands) = self.crate_packet_demands()? else {
            return Ok((0, 0));
        };
        let packets = demands.iter().try_fold(0_u32, |sum, demand| {
            sum.checked_add(demand.quantity().get())
                .ok_or(ConfirmationError::Overflow)
        })?;
        let mut subtotal = 0_i64;
        for (index, row) in self.draft.crate_rows().iter().enumerate() {
            subtotal = self
                .crate_rate(index)?
                .minor_units()
                .checked_mul(i64::from(row.quantity().get()))
                .and_then(|amount| subtotal.checked_add(amount))
                .ok_or(ConfirmationError::Overflow)?;
        }
        Ok((packets, subtotal))
    }
}

impl ConfirmationFixture {
    /// Sets the authority's rate per crate and invalidates prior reviews.
    pub fn set_crate_rate(&mut self, rate: Money) -> Result<(), ConfirmationError> {
        if rate.minor_units() < 0 {
            return Err(ConfirmationError::InvalidAmount);
        }
        self.terms_revision = self
            .terms_revision
            .checked_add(1)
            .ok_or(ConfirmationError::Overflow)?;
        self.crate_rate = Some(rate);
        Ok(())
    }

    /// Each crate row's rate (its copied rate, else the authority's crate rate) and every
    /// component's pack, crate rows in order; both empty without crate rows.
    pub(in crate::confirmation) fn crate_terms(
        &self,
        draft: &crate::ProvisionalDraft,
    ) -> Result<(Vec<Money>, Vec<NonZeroU32>), ConfirmationError> {
        let mut rates = Vec::new();
        let mut packs = Vec::new();
        for row in draft.crate_rows() {
            rates.push(
                row.rate()
                    .or(self.crate_rate)
                    .ok_or(ConfirmationError::InvalidDraft)?,
            );
            for component in row.components() {
                packs.push(self.component_pack(component.product, component.unit)?);
            }
        }
        Ok((rates, packs))
    }

    /// Reviews a bill of crate rows only at the authority's loading charge (`sell_crate`).
    pub(super) fn build_crate_review(
        &self,
        id: DraftId,
        received: Money,
    ) -> Result<Review, ConfirmationError> {
        let (revision, draft) = self
            .drafts
            .get(&id)
            .ok_or(ConfirmationError::DraftNotFound)?;
        if draft.crate_rows().is_empty() || !draft.lines().is_empty() {
            return Err(ConfirmationError::InvalidDraft);
        }
        let (prices, packs) = self.crate_terms(draft)?;
        Ok(Review {
            draft: draft.clone(),
            draft_revision: *revision,
            terms_revision: self.terms_revision,
            prices,
            payment: ProposedPayment::Named(received),
            payment_mode: super::super::ReceiptMode::Cash,
            checkout: Some(CheckoutTerms {
                lines: Vec::new(),
                loading_rate: self.loading_rate,
            }),
            crate_packs: Some(packs),
        })
    }

    /// New Bill's crate line totals before posting: crates times the rate, and the authority's
    /// loading charge per packet of its components, as `sell_crate` will charge them.
    ///
    /// # Errors
    /// Denies Managers; rejects components without checkout terms or not dividing into whole
    /// packs, and overflow.
    pub fn crate_line_amounts(
        &self,
        role: crate::Role,
        row: &crate::CrateRow,
    ) -> Result<CheckoutAmounts, ConfirmationError> {
        if role != crate::Role::Owner {
            return Err(ConfirmationError::AccessDenied);
        }
        let mut draft = crate::ProvisionalDraft::new(DraftId::new(0), Vec::new());
        draft.add_crate_row(row.clone());
        let (prices, packs) = self.crate_terms(&draft)?;
        let review = Review {
            draft,
            draft_revision: 0,
            terms_revision: self.terms_revision,
            prices,
            payment: ProposedPayment::Named(Money::from_minor_units(0)),
            payment_mode: super::super::ReceiptMode::Cash,
            checkout: Some(CheckoutTerms {
                lines: Vec::new(),
                loading_rate: self.loading_rate,
            }),
            crate_packs: Some(packs),
        };
        review.amounts()
    }

    fn component_pack(
        &self,
        product: ProductId,
        unit: SaleUnit,
    ) -> Result<NonZeroU32, ConfirmationError> {
        match unit {
            SaleUnit::Case if self.catalog.stock(product).is_some() => Ok(NonZeroU32::MIN),
            SaleUnit::Case => Err(ConfirmationError::InvalidDraft),
            SaleUnit::Piece => self
                .checkout_products
                .get(&product.value())
                .map(|terms| terms.pack)
                .ok_or(ConfirmationError::InvalidDraft),
        }
    }
}
