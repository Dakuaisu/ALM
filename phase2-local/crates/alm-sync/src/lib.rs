//! The device's view of the sales authority: the server commands the app needs, shaped for the
//! hosted Supabase functions (`post_sale`, `allocate_gst_bill_number`) but with no transport.
//! `LocalAuthority` is a development simulation over the synthetic fixture, not a server.
#![forbid(unsafe_code)]

mod intent;
#[cfg(feature = "local-authority")]
mod local;
mod rest;
pub mod wire;

use alm_core::Role;
use alm_core::projection::ProjectionBatch;

pub use intent::{
    CrateSale, CrateTerms, Intent, Lookup, Outcome, ProductTerms, PurchaseLineTerms,
    PurchaseOrderTerms, SaleIntent, SaleReceipt,
};

use alm_core::confirmation::StockCountReason;

/// The authority's key for a standalone GST bill's number allocation.
pub fn gst_bill_key(id: u64) -> String {
    format!("gst-{id}")
}

/// The intent code of a stock count reason.
pub fn stock_count_code(reason: StockCountReason) -> &'static str {
    alm_core::projection::book::reason_code(reason)
}

/// Reads [`stock_count_code`] back.
pub fn stock_count_reason(code: &str) -> Option<StockCountReason> {
    alm_core::projection::book::reason_from_code(code)
}
#[cfg(feature = "local-authority")]
pub use local::{LocalAuthority, LocalAuthorityError};
pub use rest::RestAuthority;

/// A client-minted request identity, persisted with its intent before sending: 1–64 ASCII
/// letters, digits, `-`, `_` or `:` (the Supabase `sale_requests.request_id` bound).
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RequestId(String);

impl RequestId {
    /// # Errors
    /// Rejects empty, over-long or non-ASCII identities.
    pub fn new(value: &str) -> Result<Self, AuthorityError> {
        let ok = (1..=64).contains(&value.len())
            && value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b':'));
        if ok {
            Ok(Self(value.to_owned()))
        } else {
            Err(AuthorityError::Invalid)
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The prefix of the request IDs one device install mints for one account: `<account>:<device>`,
/// followed by `-<sequence>`. Two devices of one account never mint the same ID; the account is
/// shortened when needed so a 20-digit sequence still fits the 64-byte bound (the device ID
/// alone keeps it unique).
pub fn request_prefix(account: &str, device: &str) -> String {
    const SEQUENCE: usize = 21;
    let room = 64usize.saturating_sub(device.len() + 1 + SEQUENCE);
    let account: String = account
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
        .take(room)
        .collect();
    format!("{account}:{device}")
}

/// Why a command did not produce an outcome.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthorityError {
    /// The request ID was already used with a different intent (Supabase `ALM01`).
    Conflict,
    /// The draft is already posted under another request (`ALM02`).
    AlreadyPosted,
    /// The reviewed terms no longer match the authority's (`ALM03`): review again.
    StaleReview,
    /// A definite rejection with the authority's reason; nothing was committed.
    Rejected(String),
    /// The role may not run this command or read this projection.
    Denied,
    /// Malformed request or intent.
    Invalid,
    /// No response: the command may or may not have committed. Resolve by status lookup
    /// or an explicit retry with the same request ID.
    Unavailable,
}

impl AuthorityError {
    /// True when nothing was committed and a retry with the same intent cannot succeed.
    pub const fn is_definite(&self) -> bool {
        !matches!(self, Self::Unavailable)
    }
}

/// The server commands the app needs. Every effectful command carries a request ID; an
/// identical retry returns the original outcome, and a reused ID with another intent fails.
pub trait SalesAuthority {
    /// Runs one intent under `request`.
    ///
    /// # Errors
    /// See [`AuthorityError`].
    fn submit(
        &mut self,
        role: Role,
        request: &RequestId,
        intent: &Intent,
    ) -> Result<Outcome, AuthorityError>;

    /// The committed outcome of `request`, if the authority has one.
    ///
    /// # Errors
    /// Denies roles that may not see the outcome; `Unavailable` without a response.
    fn status(&self, role: Role, request: &RequestId) -> Result<Option<Outcome>, AuthorityError>;

    /// Resolves an intent whose outcome is unknown: whether `request` committed with exactly
    /// `intent`. Has no effect.
    ///
    /// # Errors
    /// Denies roles that may not see the outcome; `Unavailable` without a response.
    fn lookup(
        &self,
        role: Role,
        request: &RequestId,
        intent: &Intent,
    ) -> Result<Lookup, AuthorityError>;

    /// Allocates (or returns the already allocated) standalone GST invoice number for a bill.
    ///
    /// # Errors
    /// Denies Managers; `Unavailable` without a response.
    fn allocate_gst_number(&mut self, role: Role, bill_id: &str) -> Result<u64, AuthorityError>;

    /// The role-filtered projection since `checkpoint`: no snapshot when nothing changed.
    ///
    /// # Errors
    /// `Unavailable` without a response.
    fn pull(&self, role: Role, checkpoint: Option<u64>) -> Result<ProjectionBatch, AuthorityError>;

    /// Posts a reviewed sale.
    ///
    /// # Errors
    /// As [`SalesAuthority::submit`].
    fn post_sale(
        &mut self,
        role: Role,
        request: &RequestId,
        sale: &SaleIntent,
    ) -> Result<SaleReceipt, AuthorityError> {
        match self.submit(role, request, &Intent::PostSale(sale.clone()))? {
            Outcome::Sale(receipt) => Ok(receipt),
            Outcome::Done | Outcome::GstNumber(_) | Outcome::Created(_) => {
                Err(AuthorityError::Invalid)
            }
        }
    }

    /// Sells `sale.quantity` crates of a saved template (each component taken from its FIFO
    /// lots as its own demand).
    ///
    /// # Errors
    /// As [`SalesAuthority::submit`].
    fn sell_crate(
        &mut self,
        role: Role,
        request: &RequestId,
        sale: &CrateSale,
    ) -> Result<SaleReceipt, AuthorityError> {
        match self.submit(role, request, &Intent::SellCrate(sale.clone()))? {
            Outcome::Sale(receipt) => Ok(receipt),
            Outcome::Done | Outcome::GstNumber(_) | Outcome::Created(_) => {
                Err(AuthorityError::Invalid)
            }
        }
    }

    /// Cancels (`cancel`) or restores a posted bill.
    ///
    /// # Errors
    /// As [`SalesAuthority::submit`].
    fn set_cancelled(
        &mut self,
        role: Role,
        request: &RequestId,
        draft: u64,
        cancel: bool,
    ) -> Result<(), AuthorityError> {
        let intent = if cancel {
            Intent::CancelSale { draft }
        } else {
            Intent::RestoreSale { draft }
        };
        self.submit(role, request, &intent).map(drop)
    }

    /// Replaces each posted line's quantity and rate (`lines` as `(quantity, rate paise)`).
    ///
    /// # Errors
    /// As [`SalesAuthority::submit`].
    fn edit_sale(
        &mut self,
        role: Role,
        request: &RequestId,
        draft: u64,
        lines: Vec<(u32, i64)>,
    ) -> Result<SaleReceipt, AuthorityError> {
        match self.submit(role, request, &Intent::EditSale { draft, lines })? {
            Outcome::Sale(receipt) => Ok(receipt),
            Outcome::Done | Outcome::GstNumber(_) | Outcome::Created(_) => {
                Err(AuthorityError::Invalid)
            }
        }
    }

    /// Records a separately dated customer receipt.
    ///
    /// # Errors
    /// As [`SalesAuthority::submit`].
    fn record_receipt(
        &mut self,
        role: Role,
        request: &RequestId,
        customer: &str,
        date: alm_core::BillDate,
        amount: alm_core::Money,
        online: bool,
    ) -> Result<(), AuthorityError> {
        let intent = Intent::RecordReceipt {
            customer: customer.to_owned(),
            date,
            amount,
            online,
        };
        self.submit(role, request, &intent).map(drop)
    }

    /// Saves (or, with `id`, edits) a Cash Book day close.
    ///
    /// # Errors
    /// As [`SalesAuthority::submit`].
    fn save_cash_day(
        &mut self,
        role: Role,
        request: &RequestId,
        id: Option<u64>,
        date: alm_core::BillDate,
        amounts: alm_core::confirmation::manual_cashbook::CashBookAmounts,
    ) -> Result<(), AuthorityError> {
        self.submit(role, request, &Intent::SaveCashDay { id, date, amounts })
            .map(drop)
    }
}
