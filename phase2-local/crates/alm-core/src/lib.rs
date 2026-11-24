//! Local synthetic slice for provisional drafts and role-scoped data access.
#![forbid(unsafe_code)]

mod bill_date;
mod bill_math;
pub mod confirmation;
mod crate_row_codec;
pub mod draft_entry;
mod draft_store;
mod drafts;
pub mod legacy;
pub mod preferences;
pub mod projection;
pub mod provisional_preview;
pub mod receive_payment;
pub mod synthetic_read_model;

use std::collections::BTreeMap;

pub use bill_date::{BillDate, InvalidBillDate, month_label};
#[doc(hidden)]
pub use crate_row_codec::{decode as decode_crate_row, encode as encode_crate_row};
pub use draft_store::{DraftStoreFailure, DurableDraftStore, InvalidStoredText};
pub use drafts::{
    CrateComponent, CrateRow, DraftCustomer, DraftCustomerId, DraftId, DraftLine, ProductId,
    ProvisionalDraft, SaleUnit,
};

/// Amount in minor currency units; a profit may be negative.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "demo-fixture", derive(serde::Serialize, serde::Deserialize))]
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

    /// Parses typed non-negative rupees with at most two decimals ("20", ".5", "1.", "120.50").
    /// Signs, exponents, separators and non-ASCII digits are rejected; nothing is rounded.
    pub fn parse_rupees(text: &str) -> Result<Self, RupeesError> {
        let text = text.trim();
        let (whole, fraction) = text.split_once('.').unwrap_or((text, ""));
        if (whole.is_empty() && fraction.is_empty())
            || fraction.len() > 2
            || !whole
                .bytes()
                .chain(fraction.bytes())
                .all(|b| b.is_ascii_digit())
        {
            return Err(RupeesError::Invalid);
        }
        let whole = if whole.is_empty() {
            0
        } else {
            whole.parse::<i64>().map_err(|_| RupeesError::Overflow)?
        };
        let mut digits = fraction.bytes().map(|digit| i64::from(digit - b'0'));
        let paise = 10 * digits.next().unwrap_or(0) + digits.next().unwrap_or(0);
        whole
            .checked_mul(100)
            .and_then(|value| value.checked_add(paise))
            .map(Self)
            .ok_or(RupeesError::Overflow)
    }

    /// Parses typed rupees that may carry one leading `-` or `+` (an advance is negative), as the
    /// HTML's `+value` reads an opening balance. The magnitude follows `parse_rupees`.
    pub fn parse_signed_rupees(text: &str) -> Result<Self, RupeesError> {
        let text = text.trim();
        match text.strip_prefix('-') {
            Some(rest) => Self::parse_rupees_unsigned_tail(rest)?
                .0
                .checked_neg()
                .map(Self)
                .ok_or(RupeesError::Overflow),
            None => Self::parse_rupees_unsigned_tail(text.strip_prefix('+').unwrap_or(text)),
        }
    }

    fn parse_rupees_unsigned_tail(text: &str) -> Result<Self, RupeesError> {
        if text.starts_with(char::is_whitespace) {
            return Err(RupeesError::Invalid);
        }
        Self::parse_rupees(text)
    }

    /// Exact rupee text that `parse_rupees` reads back ("120.50", "-0.49"), for form fields.
    pub fn to_rupee_text(self) -> String {
        let sign = if self.0 < 0 { "-" } else { "" };
        let paise = self.0.unsigned_abs();
        format!("{sign}{}.{:02}", paise / 100, paise % 100)
    }

    /// True for exactly zero paise (the HTML leaves such cells blank).
    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }

    /// Whole-rupee display value; half ties round toward positive infinity.
    pub const fn rounded_rupees(self) -> i64 {
        self.0.div_euclid(100) + if self.0.rem_euclid(100) >= 50 { 1 } else { 0 }
    }
}

/// Typed rupee text that is not an exact non-negative paise amount.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RupeesError {
    Invalid,
    Overflow,
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

#[cfg(test)]
mod money_tests {
    use super::{Money, RupeesError};

    #[test]
    fn decimal_rupees_convert_exactly_at_the_i64_boundary() {
        for (text, paise) in [
            ("0", 0),
            ("  .5 ", 50),
            (".01", 1),
            ("1.", 100),
            ("20.00", 2_000),
            ("92233720368547758.07", i64::MAX),
        ] {
            assert_eq!(
                Money::parse_rupees(text),
                Ok(Money::from_minor_units(paise))
            );
        }
        for text in [
            "", " ", ".", "+1", "-0.01", "1e2", "1.2.3", "0.001", "1,000", "١",
        ] {
            assert_eq!(Money::parse_rupees(text), Err(RupeesError::Invalid));
        }
        for paise in [0, 1, 50, 12_050, i64::MAX] {
            let money = Money::from_minor_units(paise);
            assert_eq!(Money::parse_rupees(&money.to_rupee_text()), Ok(money));
        }
        assert_eq!(Money::from_minor_units(-49).to_rupee_text(), "-0.49");
        assert_eq!(
            Money::from_minor_units(i64::MIN).to_rupee_text(),
            "-92233720368547758.08"
        );
        for text in [
            "92233720368547758.08",
            "92233720368547759",
            "9223372036854775808",
        ] {
            assert_eq!(Money::parse_rupees(text), Err(RupeesError::Overflow));
        }
    }
}
