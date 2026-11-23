//! Local synthetic slice for provisional drafts and role-scoped data access.
#![forbid(unsafe_code)]

mod bill_date;
mod bill_math;
pub mod confirmation;
mod crate_row_codec;
pub mod draft_entry;
mod draft_store;
mod drafts;
pub mod preferences;
pub mod provisional_preview;
pub mod receive_payment;
pub mod synthetic_read_model;

use std::collections::BTreeMap;

pub use bill_date::{BillDate, InvalidBillDate};
#[doc(hidden)]
pub use crate_row_codec::{decode as decode_crate_row, encode as encode_crate_row};
pub use draft_store::{DraftStoreFailure, DurableDraftStore, InvalidStoredText};
pub use drafts::{
    CrateComponent, CrateRow, CrateRowAlreadyPresent, DraftCustomer, DraftCustomerId, DraftId,
    DraftLine, ProductId, ProvisionalDraft, SaleUnit,
};

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

    /// Whole-rupee display value; half ties round toward positive infinity.
    pub const fn rounded_rupees(self) -> i64 {
        self.0.div_euclid(100) + if self.0.rem_euclid(100) >= 50 { 1 } else { 0 }
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
    drafts: DraftStore,
    total_profit: Money,
}

enum DraftStore {
    Memory(BTreeMap<DraftId, ProvisionalDraft>),
    Durable(Box<dyn DurableDraftStore>),
}

impl LocalSales {
    /// Starts an empty draft store with a supplied synthetic profit total.
    pub const fn new(total_profit: Money) -> Self {
        Self {
            drafts: DraftStore::Memory(BTreeMap::new()),
            total_profit,
        }
    }

    /// Backs drafts with a durable store; the storage crate supplies the SQLite one.
    #[doc(hidden)]
    pub fn with_durable_drafts(store: Box<dyn DurableDraftStore>, total_profit: Money) -> Self {
        Self {
            drafts: DraftStore::Durable(store),
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
    /// Saves or replaces a draft with the same draft ID. SQLite saves commit before returning.
    ///
    /// # Errors
    /// Returns a storage error if a durable save cannot be completed.
    pub fn save_draft(&mut self, draft: ProvisionalDraft) -> Result<(), DraftStoreFailure> {
        match &mut self.sales.drafts {
            DraftStore::Memory(drafts) => {
                drafts.insert(draft.id(), draft);
                Ok(())
            }
            DraftStore::Durable(store) => store.save(&draft),
        }
    }

    /// Reopens a draft without changing it or posting a sale.
    ///
    /// # Errors
    /// Returns a storage error on a failed read, rather than reporting a missing draft.
    pub fn reopen_draft(&self, id: DraftId) -> Result<Option<ProvisionalDraft>, DraftStoreFailure> {
        match &self.sales.drafts {
            DraftStore::Memory(drafts) => Ok(drafts.get(&id).cloned()),
            DraftStore::Durable(store) => store.reopen(id),
        }
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
