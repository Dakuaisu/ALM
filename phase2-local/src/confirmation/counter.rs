use std::cmp::Reverse;

use super::{ConfirmationError, ConfirmationFixture};
use crate::{BillDate, DraftCustomerId, DraftId, Role};

#[cfg(test)]
#[path = "counter/attention_tests.rs"]
mod attention_tests;

/// Caller-supplied creation order for the in-memory fixture, independent of document date and invoice.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CreationKey(u64);

impl CreationKey {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CounterBill {
    pub draft_id: DraftId,
    pub invoice: u64,
}

/// Count-only Owner attention indicators.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CounterAttention {
    pub low_stock: usize,
    pub open_purchase_orders: usize,
    pub debtors: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Counter {
    pub billed_today: Vec<CounterBill>,
    pub billed_today_count: usize,
    pub recents: Vec<CounterBill>,
    pub debtor_ids: Vec<DraftCustomerId>,
    pub attention: CounterAttention,
}

impl ConfirmationFixture {
    /// Reads the Owner's Counter without changing fixture state.
    /// Active bills dated `today` are listed in first-post order; undated bills are eligible only for recents.
    /// Recents take up to six active bills by descending creation key, treating a missing key as zero.
    /// Equal keys keep the newest first post first, matching prepend followed by a stable sort.
    /// Debtors are registered accounts with a positive balance after rounding to whole rupees.
    ///
    /// # Errors
    /// Denies Managers before reading and rejects inconsistent fixture sale records.
    pub fn counter(&self, role: Role, today: BillDate) -> Result<Counter, ConfirmationError> {
        match role {
            Role::Owner => {}
            Role::Manager => return Err(ConfirmationError::AccessDenied),
        }
        let mut billed_today = Vec::new();
        let mut recents = Vec::new();
        for (id, key) in self.first_posts.iter().rev() {
            let (review, receipt, _) = self
                .current_sales
                .get(id)
                .ok_or(ConfirmationError::InvalidDraft)?;
            if self
                .lifecycle
                .get(id)
                .ok_or(ConfirmationError::InvalidDraft)?
                .0
            {
                continue;
            }
            let row = CounterBill {
                draft_id: *id,
                invoice: receipt.invoice,
            };
            if review.draft.bill_date() == Some(today) {
                billed_today.push(row);
            }
            recents.push((key.unwrap_or(CreationKey(0)), row));
        }
        billed_today.reverse();
        recents.sort_by_key(|(key, _)| Reverse(*key));
        recents.truncate(6);
        let debtor_ids: Vec<_> = self
            .registered_customers
            .iter()
            .filter_map(|id| {
                let paise = self.customer_balances.get(id).copied().unwrap_or_default();
                let rounded = paise.div_euclid(100) + i64::from(paise.rem_euclid(100) >= 50);
                (rounded > 0).then(|| DraftCustomerId::new(id.clone()))
            })
            .collect();
        let attention = CounterAttention {
            low_stock: self
                .catalog
                .order_planner_stock(self.product_reorder_default)
                .filter(|(_, stock, reorder)| stock <= reorder)
                .count(),
            open_purchase_orders: self.purchases.receivable_count(),
            debtors: debtor_ids.len(),
        };
        Ok(Counter {
            billed_today_count: billed_today.len(),
            billed_today,
            recents: recents.into_iter().map(|(_, row)| row).collect(),
            debtor_ids,
            attention,
        })
    }
}
