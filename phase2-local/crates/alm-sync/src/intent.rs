//! Canonical, immutable confirmation intents and outcomes. The text form is what the outbox
//! stores before sending and what the authority compares on a retry: equal text, equal intent.
mod codec;
mod outcome;
#[cfg(test)]
mod tests;
mod text;

use alm_core::confirmation::manual_cashbook::CashBookAmounts;
use alm_core::{BillDate, Money, ProvisionalDraft};

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
    CancelSale {
        draft: u64,
    },
    RestoreSale {
        draft: u64,
    },
    /// `(quantity, rate in paise)` per existing line.
    EditSale {
        draft: u64,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SaleReceipt {
    pub draft: u64,
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
