//! Read-only synthetic dashboard; role selection is not authentication.
#[cfg(test)]
mod tests;
pub(super) mod windows;

use super::{ConfirmationFixture, ProposedPayment, ReceiptMode};
use crate::{BillDate, DraftId, Money, Role};
use std::cmp::Reverse;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DashboardPeriod {
    Today,
    Week,
    Month,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DashboardError {
    AccessDenied,
    DateOutOfRange,
    UnsupportedRecord,
    Overflow,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DashboardWindow {
    pub start: BillDate,
    pub end: BillDate,
}
impl DashboardWindow {
    pub fn contains(self, date: BillDate) -> bool {
        self.start.parts() <= date.parts() && date.parts() <= self.end.parts()
    }
}

/// Index in this fixture's append-only receipt vector, not a persistent identifier.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DashboardReceiptId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DashboardSource {
    Bill { draft_id: DraftId, invoice: u64 },
    Receipt(DashboardReceiptId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DashboardDetail {
    pub source: DashboardSource,
    pub date: BillDate,
    pub mode: ReceiptMode,
    pub amount: Money,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DashboardSummary {
    pub window: DashboardWindow,
    pub bill_count: usize,
    pub sales: Money,
    pub bill_paid: Money,
    pub receipts: Money,
    pub collected: Money,
    pub cash: Money,
    pub online: Money,
    /// Audit inputs are not prior-period drilldown rows.
    pub bill_inputs: Vec<DashboardSource>,
    pub receipt_inputs: Vec<DashboardSource>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Dashboard {
    pub current: DashboardSummary,
    pub prior: DashboardSummary,
    pub sales_ids: Vec<DashboardSource>,
    pub collected_detail: Vec<DashboardDetail>,
}

fn add(target: &mut Money, value: Money) -> Result<(), DashboardError> {
    *target = Money::from_minor_units(
        target
            .minor_units()
            .checked_add(value.minor_units())
            .ok_or(DashboardError::Overflow)?,
    );
    Ok(())
}

impl ConfirmationFixture {
    /// Computes date-only, inclusive dashboard windows from the current fixture.
    /// Missing dates, payment amounts, receipt modes, and legacy approvals are unsupported;
    /// checkout and crate records are unsupported rather than guessed. Signed receipts are retained.
    /// Cancelled records are excluded. Receipt IDs are fixture-local append slots.
    pub fn dashboard(
        &self,
        role: Role,
        today: BillDate,
        period: DashboardPeriod,
    ) -> Result<Dashboard, DashboardError> {
        if role != Role::Owner {
            return Err(DashboardError::AccessDenied);
        }
        let [current, prior] = windows::windows(today, period)?;
        let (current, collected_detail) = self.dashboard_window(current)?;
        let (prior, _) = self.dashboard_window(prior)?;
        Ok(Dashboard {
            sales_ids: current.bill_inputs.clone(),
            current,
            prior,
            collected_detail,
        })
    }

    fn dashboard_window(
        &self,
        window: DashboardWindow,
    ) -> Result<(DashboardSummary, Vec<DashboardDetail>), DashboardError> {
        let zero = Money::from_minor_units(0);
        let mut summary = DashboardSummary {
            window,
            bill_count: 0,
            sales: zero,
            bill_paid: zero,
            receipts: zero,
            collected: zero,
            cash: zero,
            online: zero,
            bill_inputs: Vec::new(),
            receipt_inputs: Vec::new(),
        };
        let mut bills = Vec::new();
        let mut rows = Vec::new();
        for (id, (review, receipt, _)) in &self.current_sales {
            if self
                .lifecycle
                .get(id)
                .ok_or(DashboardError::UnsupportedRecord)?
                .0
            {
                continue;
            }
            let date = review
                .draft
                .bill_date()
                .ok_or(DashboardError::UnsupportedRecord)?;
            if !window.contains(date) {
                continue;
            }
            if review.checkout.is_some()
                || review.draft.crate_row().is_some()
                || matches!(review.payment, ProposedPayment::Legacy)
            {
                return Err(DashboardError::UnsupportedRecord);
            }
            let paid = receipt.received.ok_or(DashboardError::UnsupportedRecord)?;
            let source = DashboardSource::Bill {
                draft_id: *id,
                invoice: receipt.invoice,
            };
            summary.bill_count = summary
                .bill_count
                .checked_add(1)
                .ok_or(DashboardError::Overflow)?;
            add(&mut summary.sales, receipt.total)?;
            add(&mut summary.bill_paid, paid)?;
            bills.push((receipt.invoice, source));
            if paid.minor_units() > 0 {
                rows.push((
                    (date.parts(), receipt.invoice, 0, 0),
                    DashboardDetail {
                        source,
                        date,
                        mode: review.payment_mode,
                        amount: paid,
                    },
                ));
            }
        }
        bills.sort_by_key(|(invoice, _)| Reverse(*invoice));
        summary.bill_inputs = bills.into_iter().map(|(_, source)| source).collect();
        let mut receipt_inputs = Vec::new();
        for (slot, payment) in self.preview_payments.iter().enumerate() {
            if !window.contains(payment.date) {
                continue;
            }
            let mode = payment.mode.ok_or(DashboardError::UnsupportedRecord)?;
            let source = DashboardSource::Receipt(DashboardReceiptId(slot));
            add(&mut summary.receipts, payment.amount)?;
            receipt_inputs.push((slot, source));
            rows.push((
                (payment.date.parts(), payment.after_invoice, 1, slot),
                DashboardDetail {
                    source,
                    date: payment.date,
                    mode,
                    amount: payment.amount,
                },
            ));
        }
        receipt_inputs.sort_by_key(|(slot, _)| Reverse(*slot));
        summary.receipt_inputs = receipt_inputs
            .into_iter()
            .map(|(_, source)| source)
            .collect();
        rows.sort_by_key(|(key, _)| Reverse(*key));
        for (_, row) in &rows {
            let bucket = match row.mode {
                ReceiptMode::Cash => &mut summary.cash,
                ReceiptMode::Online => &mut summary.online,
            };
            add(bucket, row.amount)?;
        }
        summary.collected = summary.bill_paid;
        add(&mut summary.collected, summary.receipts)?;
        Ok((summary, rows.into_iter().map(|(_, row)| row).collect()))
    }
}
