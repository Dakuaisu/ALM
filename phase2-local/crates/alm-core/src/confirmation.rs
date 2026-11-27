//! Synthetic in-memory confirmation fixture, not a production posting authority.
use std::collections::BTreeMap;
use std::num::NonZeroU32;

use crate::{DraftId, Money, ProductId, ProvisionalDraft, Role};

mod archive_records;
pub mod bill_image;
mod bill_image_details;
pub mod invoice_image;
pub use bill_image_details::BillImageDetails;
mod cancellation;
mod cashbook;
mod catalog;
mod checkout;
mod counter;
mod crate_list;
mod crate_templates;
mod customer_accounts;
mod customer_ageing;
mod customer_opening;
mod dashboard;
#[cfg(any(test, feature = "demo-fixture"))]
pub mod demo_fixture;
mod edit;
pub(crate) mod estimate;
mod estimate_view;
mod gst_defaults;
pub(crate) mod gst_invoice;
mod gst_standalone;
mod gst_standalone_totals;
mod image_handoff;
pub mod manual_cashbook;
mod manual_godown;
pub mod owner_billing;
mod owner_book;
mod payment;
#[cfg(not(feature = "test-support"))]
mod pending;
#[cfg(feature = "test-support")]
#[doc(hidden)]
pub mod pending;
mod posting;
mod preview;
mod projection_snapshot;
pub(crate) mod purchase_orders;
pub mod rate_list;
mod reporting;
mod sales_history;
mod sales_output;
#[cfg(feature = "demo-fixture")]
pub(crate) mod serde_pairs;
#[cfg(feature = "test-support")]
mod test_support;
pub use crate::SaleUnit;
pub use cashbook::{CashBookDay, CashBookEntry, CashBookKind};
use catalog::Catalog;
pub use catalog::{
    DailyMovementError, DailyMovementRow, PhysicalCount, PhysicalCountError, PhysicalCountInput,
    PriceField, ProductEntry, ProductEntryError, ProductList, ProductListRow, ProductPriceInput,
    ProductPricing, ProductTaxOverrides, ProductText, ReorderError, ReorderRow, ReorderStatus,
    ResolvedProductTax, StockCountMovement, StockCountReason, StockList, StockListRow, StockLotRow,
    StockLots, StockLotsError, StockStatus,
};
pub use checkout::{CheckoutAmounts, CheckoutTerms, LineTerms, ProductTerms};
pub use counter::{Counter, CounterAttention, CounterBill, CounterRecent, CounterTag, CreationKey};
pub use crate_list::{CrateListRow, crate_component_from_text};
use crate_templates::CrateTemplateBook;
pub use crate_templates::{CrateTemplate, CrateTemplateError, CrateTemplateId};
pub use customer_accounts::{
    CustomerLedger, CustomerLedgerEntry, CustomerLedgerKind, CustomerReceiptInput,
    CustomerReceiptMetadata, ReceiptMode,
};
pub use customer_ageing::{AgeingBucket, AgeingReport, AgeingRow};
pub use customer_opening::{OpeningBalance, OpeningLedger, OpeningLedgerEntry, OpeningLedgerKind};
pub use dashboard::{
    ChartPoint, DashboardInsights, Delta, Donut, DonutSlice, LineChart, TopProduct,
};
pub use dashboard::{
    Dashboard, DashboardDetail, DashboardError, DashboardPeriod, DashboardReceiptId,
    DashboardSource, DashboardSummary, DashboardWindow,
};
pub use edit::{EditChanges, LineEdit, SaleEdit};
pub use estimate::{
    EstimateDisplay, EstimateError, EstimateLineDisplay, EstimateRequest, PopupDocument,
    PrintIntent,
};
pub use gst_invoice::{
    GstError, GstGroup, GstInvoice, GstJurisdiction, GstLine, GstLineMetadata, GstRequest,
};
pub use gst_standalone::{
    GstBill, GstBillDraft, GstBillError, GstBillFields, GstBillId, GstBillInvoice, GstBillLedger,
    GstBillLine, GstBillLineInput, GstBillListRow, GstBillNumber, GstBillStore, GstBillStoreError,
    GstBillTotals, GstCustomerContact, GstHsnSummary, GstInvoiceLine, GstInvoiceParty, GstItemFill,
    GstLineFields, GstUnit, gst_percent_text, parse_gst_percent,
};
pub use image_handoff::{
    ImageFile, ImageHandoff, ImageHandoffRequest, InvoiceHandoff, InvoiceHandoffRequest,
    InvoiceImageAttempts, ShareRejected, TextHandoffIntent,
};
pub use manual_godown::{
    GodownCases, ManualGodownError, ManualGodownInput, ManualGodownList, ManualGodownRow,
};
pub use payment::ProposedPayment;
pub use pending::PendingError;
use preview::PreviewPayment;
pub use preview::{BalanceMode, BillPreview, PreviewLine, PreviousBalance};
use purchase_orders::PurchaseBook;
pub use purchase_orders::order_planner;
pub use purchase_orders::{
    FreshPurchaseOrderForm, FreshPurchaseOrderLine, PurchaseBusiness, PurchaseDraft, PurchaseError,
    PurchaseLine, PurchaseOrder, PurchaseOrderHandoff, PurchaseOrderId, PurchaseOrderList,
    PurchaseOrderNoteEdit, PurchaseOrderRow, PurchaseOrderSelection, PurchaseOrderStatus,
    PurchaseOrderView, PurchaseOutputRequest, PurchasePasteWarnings, PurchaseProduct,
    PurchaseProductDisplay, PurchaseProgressLine, PurchaseReceipt, PurchaseSupplier,
    PurchaseSupplierContact, PurchaseSupplierId, PurchaseWhatsAppRequest, ReceiveForm,
    ReceiveFormError, ReceiveFormRow, ReceiveLine, ReceiveRequest, ReceiveResult,
    SavedPurchaseLine, SavedPurchaseSupplier, SupplierBookRow, SupplierPrice,
    SupplierPriceComparison, SupplierPriceEdit, SupplierPriceError, SupplierPriceRow, parse_quote,
};
pub use reporting::{CustomerRank, DeadStockRow, MonthlyProfit, ReportsError, dead_stock_cutoff};
pub use sales_history::{
    SalesCollected, SalesDetail, SalesHistory, SalesPeriod, SalesProfit, SalesProfitError,
    SalesReceiptId, SalesRow, SalesSource,
};
pub use sales_output::{SalesOutput, SalesOutputRow, SalesPreview};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "demo-fixture", derive(serde::Serialize, serde::Deserialize))]
pub struct RequestId(pub(crate) DraftId);

impl RequestId {
    pub const fn new(value: u64) -> Self {
        Self(DraftId::new(value))
    }

    /// The request that posts `draft` itself, as the authority's `post_sale` does.
    pub const fn for_draft(draft: DraftId) -> Self {
        Self(draft)
    }

    #[cfg(feature = "test-support")]
    #[doc(hidden)]
    pub const fn value(self) -> u64 {
        self.0.value()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfirmationError {
    AccessDenied,
    InvalidAmount,
    InvalidDraft,
    UnsupportedSalesRecord(DraftId),
    DraftNotFound,
    MissingBillableItems,
    MissingCustomerName,
    MissingBillDate,
    StaleReview,
    RequestConflict,
    DraftAlreadyPosted,
    InsufficientStock,
    Overflow,
}

/// A review can only be produced by the fixture and cannot be altered by its caller.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "demo-fixture", derive(serde::Serialize, serde::Deserialize))]
pub struct Review {
    draft: ProvisionalDraft,
    draft_revision: u64,
    terms_revision: u64,
    prices: Vec<Money>,
    payment: ProposedPayment,
    payment_mode: ReceiptMode,
    checkout: Option<CheckoutTerms>,
    crate_packs: Option<Vec<NonZeroU32>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "demo-fixture", derive(serde::Serialize, serde::Deserialize))]
pub struct Receipt {
    pub draft_id: DraftId,
    pub invoice: u64,
    pub quantity: u32,
    pub total: Money,
    pub received: Option<Money>,
    pub due: Option<Money>,
    pub customer_balance: Option<Money>,
}

/// The product identity and catalog text recorded when a line becomes part of a posted bill.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "demo-fixture", derive(serde::Serialize, serde::Deserialize))]
pub struct PostedLineItem {
    pub product_id: ProductId,
    /// Anonymous catalog products have no item text.
    pub item: Option<String>,
}

/// An authority-issued receipt bound to the request and the exact approved review.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfirmationAck {
    request: RequestId,
    review: Review,
    receipt: Receipt,
}

/// Trusted synthetic fixture. Role selection is not authentication.
#[cfg_attr(
    any(test, feature = "test-support"),
    derive(Clone, Debug, Eq, PartialEq)
)]
// The development authority simulation rolls back a failed command by restoring a copy.
#[cfg_attr(
    all(feature = "demo-fixture", not(any(test, feature = "test-support"))),
    derive(Clone)
)]
#[cfg_attr(feature = "demo-fixture", derive(serde::Serialize, serde::Deserialize))]
pub struct ConfirmationFixture {
    product: ProductId,
    catalog: Catalog,
    product_reorder_default: u32,
    checkout_products: BTreeMap<u64, ProductTerms>,
    checkout_approvals: BTreeMap<DraftId, Review>,
    loading_rate: Money,
    /// The shop's New Bill loading charge per packet, which a Manager's drafts start with.
    #[cfg_attr(feature = "demo-fixture", serde(default = "zero_money"))]
    bill_loading: Money,
    crate_rate: Option<Money>,
    crate_templates: CrateTemplateBook,
    terms_revision: u64,
    draft_revision: u64,
    drafts: BTreeMap<DraftId, (u64, ProvisionalDraft)>,
    ledger: i64,
    received_ledger: i64,
    registered_customers: std::collections::BTreeSet<String>,
    /// The number of the next customer the authority adds (`cust-<n>`).
    #[cfg_attr(feature = "demo-fixture", serde(default))]
    next_customer: u64,
    customer_balances: BTreeMap<String, i64>,
    customer_openings: BTreeMap<String, customer_opening::StoredOpening>,
    next_invoice: u64,
    requests: BTreeMap<RequestId, (Review, Receipt)>,
    posted: BTreeMap<DraftId, RequestId>,
    current_sales: BTreeMap<DraftId, (Review, Receipt, u64)>,
    first_posts: Vec<(DraftId, Option<CreationKey>)>,
    posted_sale_items: BTreeMap<DraftId, Vec<PostedLineItem>>,
    preview_payments: Vec<PreviewPayment>,
    lifecycle: BTreeMap<DraftId, (bool, u64)>,
    movement_epoch: u64,
    gst_bills: BTreeMap<GstBillId, GstBill>,
    gst_deleted: std::collections::BTreeSet<GstBillId>,
    purchases: PurchaseBook,
    manual_godown: BTreeMap<String, ManualGodownRow>,
}

impl ConfirmationFixture {
    /// Creates an authority with synthetic lots in trusted oldest-first order.
    ///
    /// # Errors
    /// Rejects negative amounts or a total stock exceeding the quantity range.
    pub fn with_ordered_lots(
        product: ProductId,
        lots: &[(u32, Money)],
        unit_price: Money,
    ) -> Result<Self, ConfirmationError> {
        let mut fixture = Self::new(product, 0, unit_price)?;
        fixture.catalog.seed_lots(product, lots)?;
        Ok(fixture)
    }

    pub fn new(
        product: ProductId,
        stock: u32,
        unit_price: Money,
    ) -> Result<Self, ConfirmationError> {
        if unit_price.minor_units() < 0 {
            return Err(ConfirmationError::InvalidAmount);
        }
        Ok(Self {
            product,
            catalog: Catalog::new(product, stock, unit_price),
            product_reorder_default: 5,
            checkout_products: BTreeMap::new(),
            checkout_approvals: BTreeMap::new(),
            loading_rate: Money::from_minor_units(0),
            bill_loading: Money::from_minor_units(0),
            crate_rate: None,
            crate_templates: CrateTemplateBook::default(),
            terms_revision: 0,
            draft_revision: 0,
            drafts: BTreeMap::new(),
            ledger: 0,
            received_ledger: 0,
            registered_customers: std::collections::BTreeSet::new(),
            next_customer: 1,
            customer_balances: BTreeMap::new(),
            customer_openings: BTreeMap::new(),
            next_invoice: 1,
            requests: BTreeMap::new(),
            posted: BTreeMap::new(),
            current_sales: BTreeMap::new(),
            first_posts: Vec::new(),
            posted_sale_items: BTreeMap::new(),
            preview_payments: Vec::new(),
            lifecycle: BTreeMap::new(),
            movement_epoch: 0,
            gst_bills: BTreeMap::new(),
            gst_deleted: std::collections::BTreeSet::new(),
            purchases: PurchaseBook::default(),
            manual_godown: BTreeMap::new(),
        })
    }

    /// Registers the current draft from the trusted fixture authority.
    pub fn register_draft(&mut self, draft: ProvisionalDraft) -> Result<(), ConfirmationError> {
        let revision = self
            .draft_revision
            .checked_add(1)
            .ok_or(ConfirmationError::Overflow)?;
        self.drafts.insert(draft.id(), (revision, draft));
        self.draft_revision = revision;
        Ok(())
    }

    /// Gives Owner a current review of a reopened draft without posting it.
    /// An identical draft keeps its revision; a changed draft replaces it.
    ///
    /// # Errors
    /// Denies non-Owners before registration. Rejects drafts that do not contain
    /// known, nonempty product lines, or if a new revision would overflow.
    pub fn reconnect_review(
        &mut self,
        role: Role,
        draft: ProvisionalDraft,
    ) -> Result<Review, ConfirmationError> {
        match role {
            Role::Owner => {}
            Role::Manager => return Err(ConfirmationError::AccessDenied),
        }
        let id = draft.id();
        if self
            .drafts
            .get(&id)
            .is_none_or(|(_, current)| current != &draft)
        {
            self.catalog.prices(&draft)?;
            self.register_draft(draft)?;
        }
        self.review(role, id)
    }

    /// Changes the price for a synthetic product and invalidates prior reviews.
    pub fn set_product_price(
        &mut self,
        product: ProductId,
        price: Money,
    ) -> Result<(), ConfirmationError> {
        let revision = self
            .terms_revision
            .checked_add(1)
            .ok_or(ConfirmationError::Overflow)?;
        self.catalog.set_price(product, price)?;
        self.terms_revision = revision;
        Ok(())
    }

    pub fn review(&self, role: Role, draft_id: DraftId) -> Result<Review, ConfirmationError> {
        if role != Role::Owner {
            return Err(ConfirmationError::AccessDenied);
        }
        let (revision, draft) = self
            .drafts
            .get(&draft_id)
            .ok_or(ConfirmationError::DraftNotFound)?;
        self.review_draft(draft, *revision)
    }

    fn review_draft(
        &self,
        draft: &ProvisionalDraft,
        revision: u64,
    ) -> Result<Review, ConfirmationError> {
        let (prices, crate_packs) = self.sale_terms(draft)?;
        Ok(Review {
            draft: draft.clone(),
            draft_revision: revision,
            terms_revision: self.terms_revision,
            prices,
            payment: ProposedPayment::Legacy,
            payment_mode: ReceiptMode::Cash,
            checkout: None,
            crate_packs,
        })
    }

    /// A bill's prices (each ordinary line's sell rate, then each crate row's rate) and its
    /// crate components' packs (`None` without crate rows).
    pub(crate) fn sale_terms(
        &self,
        draft: &ProvisionalDraft,
    ) -> Result<(Vec<Money>, Option<Vec<std::num::NonZeroU32>>), ConfirmationError> {
        // Crate rows alone (HTML `saveBill` whose lines are all crates) as New Bill copied them:
        // each with its name and rate. An unpriced crate row alone is a `sell_crate` checkout.
        let crate_only = draft.lines().is_empty()
            && !draft.crate_rows().is_empty()
            && draft
                .crate_rows()
                .iter()
                .all(|row| row.name().is_some() && row.rate().is_some());
        let mut prices = if crate_only {
            Vec::new()
        } else {
            self.catalog.prices(draft)?
        };
        if draft.crate_rows().is_empty() {
            return Ok((prices, None));
        }
        let (rates, packs) = self.crate_terms(draft)?;
        prices.extend(rates);
        Ok((prices, Some(packs)))
    }

    /// The shop's New Bill loading charge per packet (DEC-034: a Manager's drafts start with it).
    pub const fn bill_loading(&self) -> Money {
        self.bill_loading
    }

    /// Sets [`Self::bill_loading`] (Owner only).
    ///
    /// # Errors
    /// Denies a Manager; rejects a negative charge.
    pub fn set_bill_loading(&mut self, role: Role, rate: Money) -> Result<(), ConfirmationError> {
        crate::permissions::require(role, crate::permissions::Capability::SetLoadingDefault)
            .map_err(|_| ConfirmationError::AccessDenied)?;
        if rate.minor_units() < 0 {
            return Err(ConfirmationError::InvalidAmount);
        }
        self.bill_loading = rate;
        Ok(())
    }

    pub const fn ledger(&self) -> Money {
        Money::from_minor_units(self.ledger)
    }
    pub const fn received_ledger(&self) -> Money {
        Money::from_minor_units(self.received_ledger)
    }

    /// Returns the number of receipts posted by the fixture, not counting replays.
    pub fn posted_count(&self) -> usize {
        self.requests.len()
    }
}

#[cfg(feature = "demo-fixture")]
const fn zero_money() -> Money {
    Money::from_minor_units(0)
}

#[cfg(test)]
mod tests;
