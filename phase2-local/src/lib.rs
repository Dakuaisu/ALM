//! Local synthetic slice for provisional drafts and role-scoped data access.
#![forbid(unsafe_code)]

mod drafts;

use std::collections::BTreeMap;

pub use drafts::{DraftId, DraftLine, ProductId, ProvisionalDraft};

/// Amount in minor currency units; a profit may be negative.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Money(i64);

impl Money {
    /// Constructs an amount without rounding or conversion.
    pub const fn from_minor_units(units: i64) -> Self {
        Self(units)
    }

    /// Returns the amount in minor units.
    pub const fn minor_units(self) -> i64 {
        self.0
    }
}

/// Synthetic roles selected by the trusted local harness, not authenticated accounts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Role {
    /// All operations available in this slice, including total profit.
    Owner,
    /// Provisional draft operations only.
    Manager,
}

/// The session lacks permission; the denial contains no report data.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AccessDenied;

/// Trusted local harness backing the role-scoped sessions. Not a client payload.
pub struct LocalSales {
    drafts: BTreeMap<DraftId, ProvisionalDraft>,
    total_profit: Money,
}

impl LocalSales {
    /// Starts an empty draft store with a supplied synthetic profit total.
    pub const fn new(total_profit: Money) -> Self {
        Self {
            drafts: BTreeMap::new(),
            total_profit,
        }
    }

    /// Selects a role in the trusted test harness. This does not authenticate a user.
    pub const fn session(&mut self, role: Role) -> SalesSession<'_> {
        SalesSession { sales: self, role }
    }
}

/// The role-scoped local API. No posting or reporting export is exposed.
pub struct SalesSession<'a> {
    sales: &'a mut LocalSales,
    role: Role,
}

impl SalesSession<'_> {
    /// Saves or replaces an in-memory draft with the same draft ID.
    pub fn save_draft(&mut self, draft: ProvisionalDraft) {
        self.sales.drafts.insert(draft.id(), draft);
    }

    /// Reopens a draft without changing it or posting a sale.
    pub fn reopen_draft(&self, id: DraftId) -> Option<&ProvisionalDraft> {
        self.sales.drafts.get(&id)
    }

    /// Returns the synthetic aggregate only to Owner.
    ///
    /// # Errors
    /// Returns [`AccessDenied`] to Manager without returning the aggregate.
    pub const fn total_profit(&self) -> Result<Money, AccessDenied> {
        match self.role {
            Role::Owner => Ok(self.sales.total_profit),
            Role::Manager => Err(AccessDenied),
        }
    }
}
