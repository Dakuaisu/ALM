//! Canonical, immutable confirmation intents and outcomes. The text form is what the outbox
//! stores before sending and what the authority compares on a retry: equal text, equal intent.
mod codec;
mod outcome;
#[cfg(test)]
mod tests;
mod text;

use alm_core::confirmation::manual_cashbook::CashBookAmounts;
use alm_core::{BillDate, DraftId, Money, ProvisionalDraft};

/// A reviewed sale: the saved draft, the confirmed date and payment, and the unit price of
/// every line as reviewed. The authority rejects it as stale if any price changed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SaleIntent {
    pub draft: ProvisionalDraft,
    pub date: BillDate,
    pub paid: Money,
    pub prices: Vec<Money>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Intent {
    PostSale(SaleIntent),
    /// A crate sold from a saved template, with the template as reviewed.
    SellCrate(CrateSale),
    CancelSale {
        draft: DraftId,
    },
    RestoreSale {
        draft: DraftId,
    },
    /// `(quantity, rate in paise)` per existing line.
    EditSale {
        draft: DraftId,
        lines: Vec<(u32, i64)>,
    },
    RecordReceipt {
        customer: String,
        date: BillDate,
        amount: Money,
        online: bool,
    },
    /// Saves a Cash Book day (`id` edits an existing row).
    SaveCashDay {
        id: Option<u64>,
        date: BillDate,
        amounts: CashBookAmounts,
    },
    DeleteCashDay {
        id: u64,
    },
    /// Products add or edit (`new` for add) with the reviewed buy and sell of an edit.
    SaveProduct(ProductTerms),
    DeleteProduct {
        id: u64,
    },
    AddSupplier {
        name: String,
    },
    /// One product row of supplier quotes: `(supplier index, rate or clear)`.
    SetSupplierPrices {
        product: u64,
        rates: Vec<(u64, Option<Money>)>,
    },
    SavePurchaseOrder(PurchaseOrderTerms),
    /// `(quantity, rate)` per PO line, with each line's remaining quantity as reviewed.
    ReceivePurchaseOrder {
        po: u64,
        date: Option<BillDate>,
        today: BillDate,
        lines: Vec<(u32, Money)>,
        remaining: Vec<u32>,
    },
    /// "Close remaining", with each line's remaining quantity as reviewed.
    ClosePurchaseOrder {
        po: u64,
        remaining: Vec<u32>,
    },
    SaveCrate(CrateTerms),
    DeleteCrate {
        id: u64,
    },
    /// The Settings default reorder level, used when a product is saved with a blank level.
    SetReorderDefault {
        level: u32,
    },
    /// The device restored a backup file: the authority logs the HTML's `restore` entry.
    RecordRestore,
    /// Imports a legacy backup file as the core mapped it (`legacy::import::plan`): the
    /// authority places each record by its legacy ID and records a manifest and quarantine.
    ImportLegacy(Box<alm_core::legacy::import::ImportPlan>),
    /// The Owner's decision on one quarantined legacy record (import as corrected, or keep it
    /// excluded), with a reason. Never sent automatically.
    ResolveLegacyQuarantine(Box<alm_core::legacy::resolve::QuarantineDecision>),
    SaveOpening {
        customer: String,
        amount: Option<Money>,
        date: Option<BillDate>,
    },
    /// A manual godown row as typed (cases are validated by the authority).
    SaveGodown {
        item: String,
        cases: String,
        note: String,
    },
    /// A physical count against the on-hand quantity the form showed.
    CountStock {
        product: u64,
        counted: u32,
        fridge: Option<u32>,
        date: BillDate,
        reason: Option<String>,
        note: Option<String>,
        on_hand: u32,
    },
    /// A standalone GST bill record (`GstBill::to_record_text`), numbered by the authority.
    SaveGstBill {
        bill: String,
    },
    CancelGstBill {
        id: u64,
    },
    DeleteGstBill {
        id: u64,
    },
    /// Moves a bill or deletion marker from the device GST tables into the authority.
    ImportGstBill {
        bill: Option<String>,
        deleted: Option<u64>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductTerms {
    pub id: u64,
    pub new: bool,
    pub item: String,
    pub brand: String,
    pub buy: Money,
    pub sell: Money,
    pub reorder: Option<u32>,
    /// For an edit: the buy and sell rates the form was opened with.
    pub reviewed: Option<(Money, Money)>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PurchaseLineTerms {
    pub item: String,
    pub product: Option<u64>,
    pub ordered: u32,
    pub rate: Money,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PurchaseOrderTerms {
    /// `(saved supplier index, name)`.
    pub supplier: Option<(Option<u64>, String)>,
    pub date: Option<BillDate>,
    pub note: String,
    pub lines: Vec<PurchaseLineTerms>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CrateTerms {
    pub id: Option<u64>,
    pub name: String,
    pub price: Money,
    /// `(product, quantity, piece)`.
    pub components: Vec<(u64, u32, bool)>,
}

/// A crate sale: `quantity` crates of saved template `template` for a directory customer, with
/// the template's rate and components (`(product, quantity, piece)`) as the Owner reviewed them.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CrateSale {
    pub draft: DraftId,
    pub template: u64,
    pub quantity: u32,
    /// `(directory ID, name)`.
    pub customer: (String, String),
    pub date: BillDate,
    pub paid: Money,
    pub rate: Money,
    pub components: Vec<(u64, u32, bool)>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SaleReceipt {
    pub draft: DraftId,
    pub invoice: u64,
    pub quantity: u32,
    pub total: Money,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outcome {
    Sale(SaleReceipt),
    Done,
    GstNumber(u64),
    /// The authority's identity for a created record (supplier, PO, crate template).
    Created(u64),
}

/// What the authority holds for a request, checked against the intent the device stored.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Lookup {
    /// It committed with this intent: the stored outcome.
    Committed(Outcome),
    /// Nothing was committed under the request: retrying it with the same ID is safe.
    NotCommitted,
    /// The request ID was used for another intent.
    OtherIntent,
}
