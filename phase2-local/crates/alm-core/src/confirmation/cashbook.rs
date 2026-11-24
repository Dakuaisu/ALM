use super::{
    ConfirmationError, ConfirmationFixture, DraftId, Money, ProposedPayment, ReceiptMode, Role,
};
use crate::{BillDate, DraftCustomer};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CashBookKind {
    Bill { draft_id: DraftId, invoice: u64 },
    Receipt,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CashBookEntry {
    pub kind: CashBookKind,
    pub mode: ReceiptMode,
    pub amount: Money,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CashBookDay {
    pub entries: Vec<CashBookEntry>,
    pub cash: Money,
    pub online: Money,
    pub cash_rupees: i64,
    pub online_rupees: i64,
    pub count: usize,
}

impl ConfirmationFixture {
    /// Projects the system totals for one document date in this trusted fixture.
    ///
    /// Named bill payments and standalone receipts use their selected mode.
    /// This role-scoped projection is not authentication or a Manager surface.
    ///
    /// # Errors
    /// Denies non-Owners and rejects unsupported records or arithmetic overflow.
    pub fn cashbook_day_close(
        &self,
        role: Role,
        date: BillDate,
    ) -> Result<CashBookDay, ConfirmationError> {
        if role != Role::Owner {
            return Err(ConfirmationError::AccessDenied);
        }
        let mut entries = Vec::new();
        for (id, (review, receipt, _)) in &self.current_sales {
            if self
                .lifecycle
                .get(id)
                .ok_or(ConfirmationError::InvalidDraft)?
                .0
            {
                continue;
            }
            let bill_date = review
                .draft
                .bill_date()
                .ok_or(ConfirmationError::InvalidDraft)?;
            if bill_date != date {
                continue;
            }
            if (review.checkout.is_some() || review.draft.crate_row().is_some())
                && !review.is_named_crate_sale()
            {
                return Err(ConfirmationError::InvalidDraft);
            }
            match (&review.payment, review.draft.customer()) {
                (ProposedPayment::Named(_), DraftCustomer::Selected { .. }) => {}
                (ProposedPayment::WalkInCash(_), DraftCustomer::Typed(name))
                    if name == "Local - Neighbor" => {}
                _ => return Err(ConfirmationError::InvalidDraft),
            }
            let paid = receipt.received.ok_or(ConfirmationError::InvalidDraft)?;
            if paid.minor_units() > 0 {
                entries.push(CashBookEntry {
                    kind: CashBookKind::Bill {
                        draft_id: *id,
                        invoice: receipt.invoice,
                    },
                    mode: review.payment_mode,
                    amount: paid,
                });
            }
        }
        for payment in &self.preview_payments {
            if payment.date != date {
                continue;
            }
            entries.push(CashBookEntry {
                kind: CashBookKind::Receipt,
                mode: payment.mode.ok_or(ConfirmationError::InvalidDraft)?,
                amount: payment.amount,
            });
        }
        let mut cash = 0i64;
        let mut online = 0i64;
        for entry in &entries {
            let bucket = match entry.mode {
                ReceiptMode::Cash => &mut cash,
                ReceiptMode::Online => &mut online,
            };
            *bucket = bucket
                .checked_add(entry.amount.minor_units())
                .ok_or(ConfirmationError::Overflow)?;
        }
        let round = |amount: i64| amount.div_euclid(100) + i64::from(amount.rem_euclid(100) >= 50);
        Ok(CashBookDay {
            count: entries.len(),
            entries,
            cash: Money::from_minor_units(cash),
            online: Money::from_minor_units(online),
            cash_rupees: round(cash),
            online_rupees: round(online),
        })
    }
}
