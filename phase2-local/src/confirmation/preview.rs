use std::num::NonZeroU32;

use super::{ConfirmationError, ConfirmationFixture, DraftId, Money, ProposedPayment, Role};
use crate::bill_math::checked_line_amount;
use crate::{BillDate, DraftCustomer, ProductId};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BalanceMode {
    AsOnBillDate,
    Latest,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreviewLine {
    pub product: ProductId,
    pub quantity: NonZeroU32,
    pub price: Money,
    pub amount: Money,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PreviousBalance {
    pub amount: Money,
    pub date: Option<BillDate>,
}

/// Explicitly allowlisted bill data; never includes cost or profit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BillPreview {
    pub invoice: u64,
    pub date: BillDate,
    pub customer: String,
    pub lines: Vec<PreviewLine>,
    pub items_quantity: u32,
    pub items_total: Money,
    pub loading: Money,
    pub total: Money,
    pub paid: Money,
    pub cancelled: bool,
    pub can_toggle: bool,
    pub balance_mode: BalanceMode,
    pub balance: Money,
    pub previous_balance: Option<PreviousBalance>,
}

/// Synthetic separate receipt, placed after an invoice in fixture posting order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct PreviewPayment {
    pub(super) customer_id: String,
    pub(super) amount: Money,
    pub(super) date: BillDate,
    pub(super) after_invoice: u64,
}

impl ConfirmationFixture {
    /// Projects the current saved regular bill without mutating fixture state.
    /// Amounts are exact minor units; legacy rounding parity is not established.
    ///
    /// # Errors
    /// Denies managers and rejects missing, unsupported, or overflowing data.
    pub fn bill_preview(
        &self,
        role: Role,
        id: DraftId,
        mode: BalanceMode,
    ) -> Result<BillPreview, ConfirmationError> {
        if role != Role::Owner {
            return Err(ConfirmationError::AccessDenied);
        }
        let (review, receipt, _) = self
            .current_sales
            .get(&id)
            .ok_or(ConfirmationError::DraftNotFound)?;
        if review.checkout.is_some() || review.draft.crate_row().is_some() {
            return Err(ConfirmationError::InvalidDraft);
        }
        let amounts = review.amounts()?;
        if amounts.total != receipt.total {
            return Err(ConfirmationError::InvalidDraft);
        }
        let (customer, customer_id) = match (review.draft.customer(), review.payment) {
            (DraftCustomer::Selected { id, name }, ProposedPayment::Named(_)) => {
                (name.clone(), Some(id.value()))
            }
            (DraftCustomer::Typed(name), ProposedPayment::WalkInCash(_))
                if name == "Local - Neighbor" =>
            {
                (name.clone(), None)
            }
            _ => return Err(ConfirmationError::InvalidDraft),
        };
        let date = review
            .draft
            .bill_date()
            .ok_or(ConfirmationError::InvalidDraft)?;
        let paid = receipt.received.ok_or(ConfirmationError::InvalidDraft)?;
        let cancelled = self
            .lifecycle
            .get(&id)
            .ok_or(ConfirmationError::InvalidDraft)?
            .0;
        let lines = review
            .draft
            .lines()
            .iter()
            .enumerate()
            .map(|(index, line)| {
                let (quantity, price) = review.line_values(index)?;
                let amount =
                    checked_line_amount(price, quantity).ok_or(ConfirmationError::Overflow)?;
                Ok(PreviewLine {
                    product: line.product_id(),
                    quantity: line.quantity(),
                    price,
                    amount,
                })
            })
            .collect::<Result<Vec<_>, ConfirmationError>>()?;
        let (previous, previous_date) = if let Some(customer_id) = customer_id {
            self.preview_balance(customer_id, Some(receipt.invoice))?
        } else {
            (0, None)
        };
        let own_due = receipt
            .total
            .minor_units()
            .checked_sub(paid.minor_units())
            .ok_or(ConfirmationError::Overflow)?;
        let use_latest = customer_id.is_some() && mode == BalanceMode::Latest;
        let balance = if use_latest {
            self.preview_balance(customer_id.ok_or(ConfirmationError::InvalidDraft)?, None)?
                .0
        } else {
            previous
                .checked_add(own_due)
                .ok_or(ConfirmationError::Overflow)?
        };
        Ok(BillPreview {
            invoice: receipt.invoice,
            date,
            customer,
            lines,
            items_quantity: amounts.packets,
            items_total: amounts.subtotal,
            loading: amounts.loading,
            total: receipt.total,
            paid,
            cancelled,
            can_toggle: customer_id.is_some(),
            balance_mode: if use_latest {
                BalanceMode::Latest
            } else {
                BalanceMode::AsOnBillDate
            },
            balance: Money::from_minor_units(balance),
            previous_balance: (!use_latest && previous != 0).then_some(PreviousBalance {
                amount: Money::from_minor_units(previous),
                date: previous_date,
            }),
        })
    }

    fn preview_balance(
        &self,
        customer: &str,
        before: Option<u64>,
    ) -> Result<(i64, Option<BillDate>), ConfirmationError> {
        let mut balance = 0_i64;
        let mut date: Option<BillDate> = None;
        for (id, (review, receipt, _)) in &self.current_sales {
            let DraftCustomer::Selected { id: other, .. } = review.draft.customer() else {
                continue;
            };
            if other.value() != customer
                || before.is_some_and(|invoice| receipt.invoice >= invoice)
                || self
                    .lifecycle
                    .get(id)
                    .ok_or(ConfirmationError::InvalidDraft)?
                    .0
            {
                continue;
            }
            let paid = receipt
                .received
                .ok_or(ConfirmationError::InvalidDraft)?
                .minor_units();
            let due = receipt
                .total
                .minor_units()
                .checked_sub(paid)
                .ok_or(ConfirmationError::Overflow)?;
            balance = balance
                .checked_add(due)
                .ok_or(ConfirmationError::Overflow)?;
            let candidate = review
                .draft
                .bill_date()
                .ok_or(ConfirmationError::InvalidDraft)?;
            if date.is_none_or(|old| candidate.to_string() > old.to_string()) {
                date = Some(candidate);
            }
        }
        for payment in &self.preview_payments {
            if payment.customer_id != customer
                || before.is_some_and(|invoice| payment.after_invoice >= invoice)
            {
                continue;
            }
            balance = balance
                .checked_sub(payment.amount.minor_units())
                .ok_or(ConfirmationError::Overflow)?;
            if date.is_none_or(|old| payment.date.to_string() > old.to_string()) {
                date = Some(payment.date);
            }
        }
        Ok((balance, date))
    }
}
