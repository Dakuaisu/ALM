//! Read-only projections over the trusted in-memory fixture, not a posting authority.
use super::{ConfirmationError, ConfirmationFixture, CreationKey, Receipt, ReceiptMode};
use crate::{BillDate, DraftCustomer, DraftId, Money, Role};
use std::cmp::Reverse;

mod profit;
pub use profit::{SalesProfit, SalesProfitError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SalesPeriod {
    Today,
    ThisMonth,
    All,
    Custom((Option<BillDate>, Option<BillDate>)),
}

/// One-based append position, valid only within this fixture; not a persisted ID.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SalesReceiptId(pub usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SalesSource {
    Bill(DraftId),
    Receipt(SalesReceiptId),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SalesRow {
    pub bill: Receipt,
    pub date: BillDate,
    pub cancelled: bool,
    pub remainder: Money,
    pub balance: Money,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SalesDetail {
    pub source: SalesSource,
    pub amount: Money,
    pub mode: ReceiptMode,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SalesCollected {
    pub bill_ids: Vec<DraftId>,
    pub receipt_ids: Vec<SalesReceiptId>,
    pub details: Vec<SalesDetail>,
    pub cash: Money,
    pub online: Money,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SalesHistory {
    pub rows: Vec<SalesRow>,
    pub active_packets: u32,
    pub sales: Money,
    pub bill_paid: Money,
    pub receipt_paid: Money,
    pub total_collected: Money,
    pub profit: Result<SalesProfit, SalesProfitError>,
    pub collected: SalesCollected,
}

fn add(a: Money, b: Money) -> Result<Money, ConfirmationError> {
    a.minor_units()
        .checked_add(b.minor_units())
        .map(Money::from_minor_units)
        .ok_or(ConfirmationError::Overflow)
}
fn subtract(a: Money, b: Money) -> Result<Money, ConfirmationError> {
    a.minor_units()
        .checked_sub(b.minor_units())
        .map(Money::from_minor_units)
        .ok_or(ConfirmationError::Overflow)
}

impl ConfirmationFixture {
    /// Computes Owner-only history from raw fixture records; no state is changed.
    /// Only ordinary dated bills are supported; in-range checkout and crate bills are rejected
    /// before search. Rows sort by stored creation key, missing as zero, with newest post first
    /// on ties. Receipt names are not stored: search uses selected-ID names in existing bills,
    /// which cannot resolve receipt-only registered IDs and may not reflect customer renames.
    /// Collected detail ordering uses the synthetic date, invoice anchor and append slot, not
    /// wall-clock timestamps. Receipt identifiers are fixture-local append positions.
    /// # Errors
    /// Denies Managers and rejects unsupported in-range records, inconsistent records or overflow.
    /// Unknown frozen cost makes only the profit section unavailable.
    pub fn sales_history(
        &self,
        role: Role,
        today: BillDate,
        period: SalesPeriod,
        search: &str,
    ) -> Result<SalesHistory, ConfirmationError> {
        if role != Role::Owner {
            return Err(ConfirmationError::AccessDenied);
        }
        let bounds = match period {
            SalesPeriod::Today => (Some(today), Some(today)),
            SalesPeriod::ThisMonth => {
                let (y, m, _) = today.parts();
                (
                    Some(BillDate::new(y, m, 1).map_err(|_| ConfirmationError::InvalidDraft)?),
                    None,
                )
            }
            SalesPeriod::All => (None, None),
            SalesPeriod::Custom(bounds) => bounds,
        };
        let in_range = |d: BillDate| {
            bounds.0.is_none_or(|a| d.parts() >= a.parts())
                && bounds.1.is_none_or(|b| d.parts() <= b.parts())
        };
        let query = search.to_lowercase();
        let matches = |s: &str| s.to_lowercase().contains(&query);
        let zero = Money::from_minor_units(0);
        let mut result = SalesHistory {
            rows: Vec::new(),
            active_packets: 0,
            sales: zero,
            bill_paid: zero,
            receipt_paid: zero,
            total_collected: zero,
            profit: Ok(SalesProfit {
                bill_ids: Vec::new(),
                revenue: zero,
                cost: zero,
                profit: zero,
            }),
            collected: SalesCollected {
                bill_ids: Vec::new(),
                receipt_ids: Vec::new(),
                details: Vec::new(),
                cash: zero,
                online: zero,
            },
        };
        let mut ordered = Vec::new();
        for (id, key) in self.first_posts.iter().rev() {
            let (review, bill, _) = self
                .current_sales
                .get(id)
                .ok_or(ConfirmationError::InvalidDraft)?;
            let date = review
                .draft
                .bill_date()
                .ok_or(ConfirmationError::InvalidDraft)?;
            if !in_range(date) {
                continue;
            }
            if review.checkout.is_some() || review.draft.crate_row().is_some() {
                return Err(ConfirmationError::UnsupportedSalesRecord(*id));
            }
            let (name, customer) = match review.draft.customer() {
                DraftCustomer::Selected { id, name } => (name.as_str(), Some(id)),
                DraftCustomer::Typed(name) => (name.as_str(), None),
                DraftCustomer::Unassigned => ("", None),
            };
            let items = self
                .posted_sale_items
                .get(id)
                .ok_or(ConfirmationError::InvalidDraft)?;
            if !(matches(name)
                || items
                    .iter()
                    .any(|item| item.item.as_deref().is_some_and(matches)))
            {
                continue;
            }
            let cancelled = self
                .lifecycle
                .get(id)
                .ok_or(ConfirmationError::InvalidDraft)?
                .0;
            let paid = bill.received.unwrap_or(zero);
            let remainder = subtract(bill.total, paid)?;
            let balance = if cancelled {
                zero
            } else {
                customer.map_or(remainder, |id| self.customer_due(id.value()))
            };
            ordered.push((
                key.unwrap_or(CreationKey::new(0)),
                SalesRow {
                    bill: bill.clone(),
                    date,
                    cancelled,
                    remainder,
                    balance,
                },
                review.payment_mode,
            ));
        }
        ordered.sort_by_key(|(key, _, _)| Reverse(*key));
        let mut details = Vec::new();
        for (_, row, mode) in ordered {
            if !row.cancelled {
                let id = row.bill.draft_id;
                let paid = row.bill.received.unwrap_or(zero);
                result.active_packets = result
                    .active_packets
                    .checked_add(row.bill.quantity)
                    .ok_or(ConfirmationError::Overflow)?;
                result.sales = add(result.sales, row.bill.total)?;
                result.bill_paid = add(result.bill_paid, paid)?;
                result.collected.bill_ids.push(id);
                if paid.minor_units() > 0 {
                    details.push((
                        (row.date.parts(), row.bill.invoice, 0),
                        SalesDetail {
                            source: SalesSource::Bill(id),
                            amount: paid,
                            mode,
                        },
                    ));
                }
            }
            result.rows.push(row);
        }
        for (slot, payment) in self.preview_payments.iter().enumerate() {
            let party_matches =
                self.current_sales
                    .values()
                    .any(|(review, _, _)| match review.draft.customer() {
                        DraftCustomer::Selected { id, name } => {
                            id.value() == payment.customer_id && matches(name)
                        }
                        DraftCustomer::Typed(_) | DraftCustomer::Unassigned => false,
                    });
            if !in_range(payment.date) || !(query.is_empty() || party_matches) {
                continue;
            }
            let id = SalesReceiptId(slot.checked_add(1).ok_or(ConfirmationError::Overflow)?);
            result.collected.receipt_ids.push(id);
            result.receipt_paid = add(result.receipt_paid, payment.amount)?;
            details.push((
                (payment.date.parts(), payment.after_invoice, id.0),
                SalesDetail {
                    source: SalesSource::Receipt(id),
                    amount: payment.amount,
                    mode: payment.mode.ok_or(ConfirmationError::InvalidDraft)?,
                },
            ));
        }
        details.sort_by_key(|(key, _)| Reverse(*key));
        for (_, detail) in details {
            let bucket = match detail.mode {
                ReceiptMode::Cash => &mut result.collected.cash,
                ReceiptMode::Online => &mut result.collected.online,
            };
            *bucket = add(*bucket, detail.amount)?;
            result.collected.details.push(detail);
        }
        result.total_collected = add(result.bill_paid, result.receipt_paid)?;
        result.profit = profit::calculate(self, &result.rows)?;
        Ok(result)
    }
}
