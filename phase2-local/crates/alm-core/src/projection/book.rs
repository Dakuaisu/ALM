//! The Owner's book rows: everything the Owner screens and reviews are computed from (Products,
//! Suppliers and quotes, Order Planner, Purchase Orders, Crates, Godown, Reorder, Rate List,
//! Reports, Dashboard, GST Sales, Counter, the priced draft review), as plain rows an authority
//! can send. The device rebuilds a read-only core view from them
//! ([`crate::confirmation::ConfirmationFixture::from_owner_book`]); every screen's arithmetic
//! stays in the core. Never sent to a Manager.
use crate::confirmation::manual_cashbook::CashBookAmounts;
use crate::confirmation::{GstBill, StockCountMovement};
use crate::{BillDate, Money};

mod json;
pub use json::{
    BookFormatError, from_json, from_sections, manager_json, manager_sections, reason_code,
    reason_from_code, to_json, to_sections,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BookSettings {
    /// The product new bills default to.
    pub default_product: u64,
    pub reorder_default: u32,
    pub loading_rate: Money,
    pub crate_rate: Option<Money>,
    pub next_invoice: u64,
    /// The stock movement epoch (cancel and restore require none since the bill's change).
    pub movement: u64,
    /// Sum of active bill totals and of their received amounts.
    pub ledger: Money,
    pub received_ledger: Money,
    /// The next Cash Book row number.
    pub next_cash: u64,
}

impl Default for BookSettings {
    fn default() -> Self {
        let zero = Money::from_minor_units(0);
        Self {
            default_product: 0,
            reorder_default: 5,
            loading_rate: zero,
            crate_rate: None,
            next_invoice: 1,
            movement: 0,
            ledger: zero,
            received_ledger: zero,
            next_cash: 0,
        }
    }
}

/// One FIFO layer, in the product's storage order (allocations refer to this index).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BookLot {
    pub date: Option<BillDate>,
    pub remaining: u32,
    pub unit_cost: Option<Money>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BookProductDetails {
    pub item: String,
    pub brand: String,
    pub reorder_level: u32,
    pub gst_rate: Option<u32>,
    pub cess: Option<u32>,
    pub hsn: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BookProduct {
    pub id: u64,
    /// `None` for an anonymous catalog product.
    pub details: Option<BookProductDetails>,
    pub buy: Money,
    pub margin: Money,
    pub sell: Money,
    pub stock: u32,
    pub seeded_opening: u32,
    pub fridge: u32,
    pub lots: Vec<BookLot>,
    pub counts: Vec<StockCountMovement>,
}

/// Authority checkout terms for a product: `(product, pack, piece price, case price)`.
pub type BookCheckoutTerms = (u64, u32, Money, Money);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BookCustomerRef {
    Unassigned,
    Typed(String),
    Selected { id: String, name: String },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BookPayment {
    Legacy,
    Named(Money),
    WalkInCash(Money),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BookSaleLine {
    pub product: u64,
    pub quantity: u32,
}

/// Packets one sale line took from one lot, and their cost when known.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BookAllocation {
    pub line: u64,
    pub product: u64,
    pub lot: u64,
    pub quantity: u32,
    pub cost: Option<Money>,
}

/// A crate row: `(product, quantity, piece)` per component and the template's name and rate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BookCrateRow {
    pub quantity: u32,
    pub components: Vec<(u64, u32, bool)>,
    pub price: Option<(String, Money)>,
}

/// Checkout terms a bill was reviewed under: `(piece, pack, piece price, case price)` per line.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BookCheckout {
    pub lines: Vec<(bool, u32, Money, Money)>,
    pub loading_rate: Money,
}

/// A posted bill as the authority holds it now (after any edit), in first-posting order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BookSale {
    pub draft: crate::DraftId,
    pub invoice: u64,
    pub customer: BookCustomerRef,
    pub date: Option<BillDate>,
    pub loading_rate: Money,
    pub lines: Vec<BookSaleLine>,
    /// The reviewed unit prices: one per line, then each crate row's rate.
    pub prices: Vec<Money>,
    /// Crate rows beside the lines, in order.
    pub crate_rows: Vec<BookCrateRow>,
    pub checkout: Option<BookCheckout>,
    /// Packs per crate component (crate rows in order), frozen at review.
    pub crate_packs: Option<Vec<u32>>,
    pub payment: BookPayment,
    pub online: bool,
    pub quantity: u32,
    pub total: Money,
    pub received: Option<Money>,
    pub due: Option<Money>,
    /// The customer's balance right after the bill was posted or last edited.
    pub customer_balance: Option<Money>,
    pub edits: u64,
    pub cancelled: bool,
    /// The movement epoch of the bill's last change.
    pub epoch: u64,
    pub creation_key: Option<u64>,
    /// `(product, item text)` per billed line, as captured when the line was posted.
    pub items: Vec<(u64, Option<String>)>,
    /// The frozen FIFO cost; `None` when unknown.
    pub cost: Option<Money>,
    /// `(line, unit cost)` frozen by the first in-place edit.
    pub line_units: Vec<(u64, Option<Money>)>,
    pub allocations: Vec<BookAllocation>,
}

/// A separately dated customer receipt, in recording order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BookReceipt {
    pub customer: String,
    pub amount: Money,
    pub date: BillDate,
    /// The last invoice number issued when it was recorded.
    pub after_invoice: u64,
    pub online: Option<bool>,
    /// `(selected name, creation key)` when recorded through Receive Payment.
    pub selected: Option<(String, u64)>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BookCustomer {
    pub id: String,
    pub registered: bool,
    pub balance: Option<Money>,
    pub opening: Option<(Money, Option<BillDate>)>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BookOrderReceipt {
    pub date: BillDate,
    pub quantity: u32,
    pub rate: Money,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BookOrderLine {
    pub item: String,
    pub product: Option<u64>,
    pub ordered: u32,
    pub received: u32,
    pub closed: u32,
    pub rate: Money,
    pub receipts: Vec<BookOrderReceipt>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BookOrder {
    pub id: u64,
    pub supplier: u64,
    pub supplier_name: String,
    pub date: BillDate,
    pub note: String,
    pub cancelled: bool,
    pub lines: Vec<BookOrderLine>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BookPurchasing {
    /// `(product, item, buy rate)` known to purchasing, in catalog order.
    pub products: Vec<(u64, String, Money)>,
    /// `(id, name)`.
    pub suppliers: Vec<(u64, String)>,
    /// The suppliers shown as price-comparison columns, in column order.
    pub price_suppliers: Vec<u64>,
    /// `(product, running average receipt cost)`.
    pub costs: Vec<(u64, Money)>,
    /// `(product, [(supplier name, rate)])` in quote order.
    pub quotes: Vec<(u64, Vec<(String, Money)>)>,
    pub orders: Vec<BookOrder>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BookCrate {
    pub id: u64,
    pub name: String,
    pub price: Money,
    /// `(product, quantity, piece)`.
    pub components: Vec<(u64, u32, bool)>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BookCrates {
    pub last_id: u64,
    pub templates: Vec<BookCrate>,
    /// `(product, piece MRP)`.
    pub piece_mrps: Vec<(u64, Money)>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BookGodownRow {
    pub item: String,
    pub cases: u32,
    pub note: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BookCashDay {
    pub id: u64,
    pub date: BillDate,
    pub amounts: CashBookAmounts,
}

/// One authority-recorded action (`logAct`): when, from which device and account, the HTML's
/// action type and the parts of its message.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActivityRow {
    pub at_ms: u64,
    pub device: String,
    pub account: String,
    /// `bill-save`, `bill-edit`, `bill-cancel`, `bill-restore`, `payment`, `grn`, `adjust`,
    /// `prod-delete`, `crate-save`, `crate-edit`, `crate-delete`, `gst-save`, `gst-edit`,
    /// `gst-cancel`, `gst-delete` or `restore`.
    pub kind: String,
    pub subject: String,
    pub amount: Option<Money>,
    pub detail: String,
}

/// `fmtN` for whole numbers: en-IN digit grouping (12,34,567).
pub fn indian_number(value: i64) -> String {
    let digits = value.unsigned_abs().to_string();
    let (head, tail) = digits.split_at(digits.len().saturating_sub(3));
    let mut out = String::new();
    for (i, c) in head.chars().enumerate() {
        if i > 0 && (head.len() - i) % 2 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if !head.is_empty() {
        out.push(',');
    }
    out.push_str(tail);
    if value < 0 { format!("-{out}") } else { out }
}

/// `moneyR`: rupees rounded with `Math.round` (halves toward +∞), grouped en-IN.
pub fn money_r(value: Money) -> String {
    let paise = value.minor_units();
    let rupees = (paise + 50).div_euclid(100);
    format!("₹{}", indian_number(rupees))
}

impl ActivityRow {
    /// The HTML's `logAct` message for this action.
    pub fn message(&self) -> String {
        let amount = self.amount.map(money_r).unwrap_or_default();
        match self.kind.as_str() {
            "bill-save" | "bill-edit" | "bill-cancel" => format!("{} {amount}", self.subject),
            "payment" => format!("{} {amount} ({})", self.subject, self.detail),
            "grn" => format!("{} — goods received", self.subject),
            "adjust" => format!("{}: {}", self.subject, self.detail),
            _ => self.subject.clone(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OwnerBook {
    pub settings: BookSettings,
    /// In catalog order.
    pub products: Vec<BookProduct>,
    pub checkout_terms: Vec<BookCheckoutTerms>,
    pub sales: Vec<BookSale>,
    pub receipts: Vec<BookReceipt>,
    /// In ID order.
    pub customers: Vec<BookCustomer>,
    pub purchasing: BookPurchasing,
    pub crates: BookCrates,
    /// In item (lowercase) order.
    pub godown: Vec<BookGodownRow>,
    /// In bill ID order.
    pub gst_bills: Vec<GstBill>,
    pub gst_deleted: Vec<u64>,
    /// In row number order.
    pub cash_days: Vec<BookCashDay>,
    /// In recording order (kept by the authority, not by the core state).
    pub activity: Vec<ActivityRow>,
    /// Legacy imports: manifests and the records placed (kept by the authority).
    pub legacy: crate::legacy::import::LegacyImports,
}

#[cfg(test)]
mod tests {
    use super::{ActivityRow, indian_number, money_r};
    use crate::Money;

    #[test]
    fn numbers_and_rupees_follow_fmtn_and_money_r() {
        assert_eq!(indian_number(0), "0");
        assert_eq!(indian_number(999), "999");
        assert_eq!(indian_number(1_000), "1,000");
        assert_eq!(indian_number(1_234_567), "12,34,567");
        assert_eq!(indian_number(-12_345), "-12,345");
        assert_eq!(money_r(Money::from_minor_units(12_050)), "₹121");
        assert_eq!(money_r(Money::from_minor_units(12_049)), "₹120");
        assert_eq!(money_r(Money::from_minor_units(-150)), "₹-1");
        let row = |kind: &str, amount: Option<i64>, detail: &str| ActivityRow {
            at_ms: 0,
            device: String::new(),
            account: String::new(),
            kind: kind.into(),
            subject: "#3 Synthetic Customer".into(),
            amount: amount.map(Money::from_minor_units),
            detail: detail.into(),
        };
        assert_eq!(
            row("bill-save", Some(36_150), "").message(),
            "#3 Synthetic Customer ₹362"
        );
        assert_eq!(
            row("bill-restore", None, "").message(),
            "#3 Synthetic Customer"
        );
        assert_eq!(
            row("payment", Some(1_000), "Cash").message(),
            "#3 Synthetic Customer ₹10 (Cash)"
        );
        assert_eq!(
            row("adjust", None, "40 → 37 (Damage / leakage)").message(),
            "#3 Synthetic Customer: 40 → 37 (Damage / leakage)"
        );
    }
}
