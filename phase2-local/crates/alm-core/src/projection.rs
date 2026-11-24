//! Read-only projections the sales authority sends to a device: the posted rows each screen
//! shows, filtered by role before they leave the authority. The device stores them and builds
//! screen models from them with the functions here; it never posts from them.
use crate::confirmation::manual_cashbook::{CashBookAmounts, ManualCashBook};
use crate::confirmation::owner_billing::{BalanceSide, BillingDirectory, CustomerFilter};
use crate::confirmation::{
    ConfirmationError, ConfirmationFixture, ReceiptMode, SalesPeriod, StockStatus,
};
use crate::{BillDate, DraftCustomer, Money, Role};

pub mod book;

/// A posted bill as the Sales screen lists it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectedSale {
    pub draft_id: u64,
    pub invoice: u64,
    pub date: BillDate,
    pub customer: String,
    /// Item names, for the Sales search.
    pub items: Vec<String>,
    pub lines: u32,
    pub packets: u32,
    pub total: Money,
    pub paid: Money,
    pub online: bool,
    pub cancelled: bool,
    /// The customer's balance shown on the row (zero when cancelled).
    pub balance: Money,
}

impl ProjectedSale {
    /// The legacy Credit tag: any unpaid remainder.
    pub const fn is_credit(&self) -> bool {
        self.total.minor_units() > self.paid.minor_units()
    }
}

/// A separately dated customer receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectedReceipt {
    pub slot: u64,
    pub date: BillDate,
    pub customer: String,
    pub amount: Money,
    pub online: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectedCustomer {
    pub id: String,
    pub name: String,
    pub balance: Money,
    pub mobile: String,
    pub addr1: String,
    pub addr2: String,
}

impl ProjectedCustomer {
    /// The Customers list's address cell: `addr1`, then `, addr2` when present.
    pub fn address(&self) -> String {
        if self.addr2.is_empty() {
            self.addr1.clone()
        } else {
            format!("{}, {}", self.addr1, self.addr2)
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectedStock {
    pub product: u64,
    pub item: String,
    pub brand: String,
    pub on_hand: u32,
    pub reorder_level: u32,
}

/// A product a bill may select, with its provisional (Manager-visible) rate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectedProduct {
    pub id: u64,
    pub name: String,
    pub rate: Money,
}

/// A saved Cash Book day (the manual day-close figures).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProjectedCashDay {
    pub id: u64,
    pub date: BillDate,
    pub amounts: CashBookAmounts,
}

/// A product's selling terms as the Products and Rate List screens show them (no buy rate).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectedProductTerms {
    pub id: u64,
    pub item: String,
    pub brand: String,
    pub sell: Money,
    pub reorder_level: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectedSupplier {
    pub id: u64,
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectedCrate {
    pub id: u64,
    pub name: String,
    pub price: Money,
    pub components: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectedGodownRow {
    pub item: String,
    pub cases: u32,
    pub note: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectedGstBill {
    pub id: u64,
    pub number: u64,
    pub date: BillDate,
    pub customer: String,
    pub cancelled: bool,
}

/// A purchase order line; its rate is a buy rate, so orders are owner values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectedPurchaseLine {
    pub item: String,
    pub product: Option<u64>,
    pub ordered: u32,
    pub received: u32,
    pub remaining: u32,
    pub rate: Money,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectedPurchaseOrder {
    pub id: u64,
    pub supplier: String,
    pub date: BillDate,
    pub status: String,
    pub lines: Vec<ProjectedPurchaseLine>,
}

/// Owner-only values: never sent to a Manager device.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OwnerValues {
    /// Per active bill: its revenue and frozen FIFO cost; `None` when a cost is unknown.
    pub sale_costs: Vec<(u64, Money, Option<Money>)>,
    /// Total FIFO stock value; `None` when any lot cost is unknown.
    pub stock_value: Option<Money>,
    /// `(product, buy rate)`.
    pub buy_rates: Vec<(u64, Money)>,
    /// `(product, supplier, quoted rate)`.
    pub supplier_prices: Vec<(u64, u64, Money)>,
    pub purchase_orders: Vec<ProjectedPurchaseOrder>,
    /// The book rows every other Owner screen is computed from.
    pub book: book::OwnerBook,
}

/// Everything one account may see, as of one authority checkpoint.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProjectionSnapshot {
    /// A posted bill the sales list cannot represent yet (crate or checkout bills); the
    /// Sales screen reports it instead of listing a partial history.
    pub sales_unsupported: Option<u64>,
    pub sales: Vec<ProjectedSale>,
    pub receipts: Vec<ProjectedReceipt>,
    pub customers: Vec<ProjectedCustomer>,
    pub stock: Vec<ProjectedStock>,
    pub catalog: Vec<ProjectedProduct>,
    pub cash_days: Vec<ProjectedCashDay>,
    pub products: Vec<ProjectedProductTerms>,
    pub suppliers: Vec<ProjectedSupplier>,
    pub crates: Vec<ProjectedCrate>,
    pub godown: Vec<ProjectedGodownRow>,
    pub gst_bills: Vec<ProjectedGstBill>,
    pub owner: Option<OwnerValues>,
}

/// What the device applies in one transaction: the checkpoint reached and, when anything
/// changed since the device's checkpoint, the full replacement for its projection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionBatch {
    pub role: Role,
    pub checkpoint: u64,
    /// Milliseconds since the Unix epoch at the authority when the checkpoint was read.
    pub as_of_ms: u64,
    pub snapshot: Option<ProjectionSnapshot>,
}

impl ConfirmationFixture {
    /// The role-filtered snapshot for a device: the Owner gets every projected screen and the
    /// owner values; a Manager gets only the catalog and customer names it drafts with.
    ///
    /// # Errors
    /// Propagates read failures from the underlying screens.
    pub fn projection(
        &self,
        role: Role,
        directory: &BillingDirectory,
        cashbook: &ManualCashBook,
    ) -> Result<ProjectionSnapshot, ConfirmationError> {
        self.projection_snapshot(role, directory, cashbook)
    }
}

/// The Sales screen model over projected rows (`sales_history` semantics).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SalesView<'a> {
    pub rows: Vec<&'a ProjectedSale>,
    pub bills: usize,
    pub cancelled: usize,
    pub sales: Money,
    pub packets: u32,
    pub bill_paid: Money,
    pub receipt_paid: Money,
    pub total_collected: Money,
    /// Owner-only estimated profit; `None` without owner values or with an unknown cost.
    pub profit: Option<Money>,
}

fn add(a: Money, b: Money) -> Result<Money, ConfirmationError> {
    a.minor_units()
        .checked_add(b.minor_units())
        .map(Money::from_minor_units)
        .ok_or(ConfirmationError::Overflow)
}

impl ProjectionSnapshot {
    /// Filters the projected sales by period and customer/item search and totals them.
    ///
    /// # Errors
    /// Denies Managers (OPEN-004); rejects total overflow.
    pub fn sales_view(
        &self,
        role: Role,
        today: BillDate,
        period: SalesPeriod,
        search: &str,
    ) -> Result<SalesView<'_>, ConfirmationError> {
        if role != Role::Owner {
            return Err(ConfirmationError::AccessDenied);
        }
        if let Some(id) = self.sales_unsupported {
            return Err(ConfirmationError::UnsupportedSalesRecord(
                crate::DraftId::new(id),
            ));
        }
        let bounds = match period {
            SalesPeriod::Today => (Some(today), Some(today)),
            SalesPeriod::ThisMonth => (Some(today.first_of_month()), None),
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
        let mut view = SalesView {
            rows: Vec::new(),
            bills: 0,
            cancelled: 0,
            sales: zero,
            packets: 0,
            bill_paid: zero,
            receipt_paid: zero,
            total_collected: zero,
            profit: None,
        };
        let (mut revenue, mut cost, mut known) = (zero, zero, true);
        for sale in &self.sales {
            if !in_range(sale.date)
                || !(matches(&sale.customer) || sale.items.iter().any(|i| matches(i)))
            {
                continue;
            }
            view.rows.push(sale);
            if sale.cancelled {
                view.cancelled += 1;
                continue;
            }
            view.bills += 1;
            view.sales = add(view.sales, sale.total)?;
            view.bill_paid = add(view.bill_paid, sale.paid)?;
            view.packets = view
                .packets
                .checked_add(sale.packets)
                .ok_or(ConfirmationError::Overflow)?;
            match self.owner.as_ref().and_then(|owner| {
                owner
                    .sale_costs
                    .iter()
                    .find(|(id, _, _)| *id == sale.draft_id)
            }) {
                Some((_, sale_revenue, Some(sale_cost))) => {
                    revenue = add(revenue, *sale_revenue)?;
                    cost = add(cost, *sale_cost)?;
                }
                _ => known = false,
            }
        }
        for receipt in &self.receipts {
            let party = self
                .sales
                .iter()
                .any(|sale| sale.customer == receipt.customer && matches(&sale.customer))
                || matches(&receipt.customer);
            if in_range(receipt.date) && (query.is_empty() || party) {
                view.receipt_paid = add(view.receipt_paid, receipt.amount)?;
            }
        }
        view.total_collected = add(view.bill_paid, view.receipt_paid)?;
        if self.owner.is_some() && known {
            view.profit = Some(Money::from_minor_units(
                revenue
                    .minor_units()
                    .checked_sub(cost.minor_units())
                    .ok_or(ConfirmationError::Overflow)?,
            ));
        }
        Ok(view)
    }

    /// The Customers screen over projected balances (`customer_book` semantics).
    ///
    /// # Errors
    /// Denies Managers (OPEN-004); rejects total overflow.
    pub fn customer_view(
        &self,
        role: Role,
        filter: &CustomerFilter,
    ) -> Result<CustomerView<'_>, ConfirmationError> {
        if role != Role::Owner {
            return Err(ConfirmationError::AccessDenied);
        }
        let needle = match filter {
            CustomerFilter::Name(text) => text.trim().to_lowercase(),
            CustomerFilter::All | CustomerFilter::Due => String::new(),
        };
        let zero = Money::from_minor_units(0);
        let mut view = CustomerView {
            rows: Vec::new(),
            receivable: zero,
            advances: zero,
            debtors: 0,
            customers: self.customers.len(),
        };
        for customer in &self.customers {
            let side = BalanceSide::of(customer.balance)?;
            match side {
                BalanceSide::Due(amount) => view.receivable = add(view.receivable, amount)?,
                BalanceSide::Advance(amount) => view.advances = add(view.advances, amount)?,
                BalanceSide::Settled => {}
            }
            if crate::confirmation::owner_billing::is_debtor(customer.balance) {
                view.debtors += 1;
            }
            let keep = match filter {
                CustomerFilter::All => true,
                CustomerFilter::Due => customer.balance.minor_units() > 0,
                // `c.name.toLowerCase().includes(q) || (c.mobile || "").includes(custQ)`.
                CustomerFilter::Name(text) => {
                    customer.name.to_lowercase().contains(&needle)
                        || customer.mobile.contains(text.as_str())
                }
            };
            if keep {
                view.rows.push((customer, side));
            }
        }
        view.rows.sort_by(|a, b| a.0.name.cmp(&b.0.name));
        Ok(view)
    }

    /// The Stock screen over projected quantities (`stock_list` semantics) with the owner
    /// FIFO value.
    ///
    /// # Errors
    /// Denies Managers (OPEN-004: stock quantities without values are undecided).
    pub fn stock_view(
        &self,
        role: Role,
        query: &str,
        low_only: bool,
    ) -> Result<StockView<'_>, ConfirmationError> {
        if role != Role::Owner {
            return Err(ConfirmationError::AccessDenied);
        }
        let needle = query.trim().to_lowercase();
        let mut view = StockView {
            rows: Vec::new(),
            skus: self.stock.len(),
            in_stock: 0,
            low: 0,
            out: 0,
            value: self.owner.as_ref().and_then(|owner| owner.stock_value),
        };
        for row in &self.stock {
            let status = if row.on_hand == 0 {
                StockStatus::Out
            } else if row.on_hand <= row.reorder_level {
                StockStatus::Low
            } else {
                StockStatus::Ok
            };
            match status {
                StockStatus::Out => view.out += 1,
                StockStatus::Low => view.low += 1,
                StockStatus::Ok => {}
            }
            if row.on_hand > 0 {
                view.in_stock += 1;
            }
            if row.item.to_lowercase().contains(&needle)
                && !(low_only && row.on_hand > row.reorder_level)
            {
                view.rows.push((row, status));
            }
        }
        view.rows.sort_by_key(|(row, _)| row.on_hand);
        Ok(view)
    }
}

/// The freshness label for projected screens: "as of 24-Nov-2026 10:42 UTC".
pub fn as_of_text(ms: u64) -> String {
    let seconds = ms / 1000;
    let days = i64::try_from(seconds / 86_400).unwrap_or(0);
    let minute = seconds % 86_400 / 60;
    // Civil date from days since 1970-01-01 (proleptic Gregorian).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    let date = u16::try_from(year)
        .ok()
        .zip(u8::try_from(month).ok())
        .zip(u8::try_from(day).ok())
        .and_then(|((y, m), d)| BillDate::new(y, m, d).ok())
        .map_or_else(|| "?".to_owned(), BillDate::display);
    format!("as of {date} {:02}:{:02} UTC", minute / 60, minute % 60)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CustomerView<'a> {
    pub rows: Vec<(&'a ProjectedCustomer, BalanceSide)>,
    pub receivable: Money,
    pub advances: Money,
    pub debtors: usize,
    pub customers: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StockView<'a> {
    pub rows: Vec<(&'a ProjectedStock, StockStatus)>,
    pub skus: usize,
    pub in_stock: usize,
    pub low: usize,
    pub out: usize,
    pub value: Option<Money>,
}

pub(crate) fn customer_name(customer: &DraftCustomer) -> String {
    match customer {
        DraftCustomer::Selected { name, .. } | DraftCustomer::Typed(name) => name.clone(),
        DraftCustomer::Unassigned => String::new(),
    }
}

pub(crate) const fn online(mode: ReceiptMode) -> bool {
    matches!(mode, ReceiptMode::Online)
}

#[cfg(test)]
mod tests {
    #[test]
    fn as_of_text_reads_utc_wall_time() {
        assert_eq!(super::as_of_text(0), "as of 01-Jan-1970 00:00 UTC");
        assert_eq!(
            super::as_of_text(1_795_999_320_000),
            "as of 30-Nov-2026 00:42 UTC"
        );
    }
}
