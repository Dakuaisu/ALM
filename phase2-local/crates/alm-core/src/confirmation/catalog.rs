use std::collections::BTreeMap;

use super::{ConfirmationError, DraftId, Money, ProductId, ProvisionalDraft, Review};

mod book;
mod cancellation;
mod daily_movement;
mod edit;
mod line_edit;
mod physical_count;
mod receiving;

mod pricing;
mod product_delete;
mod product_entry;
mod product_list;
mod reorder;
mod stock_list;
mod stock_lots;

#[cfg(test)]
mod lot_layer_replay;

pub use daily_movement::{DailyMovementError, DailyMovementRow};
use physical_count::PhysicalCountState;
pub use physical_count::{
    PhysicalCount, PhysicalCountError, PhysicalCountInput, StockCountMovement, StockCountReason,
};
pub use pricing::{ProductPriceInput, ProductPricing};
use product_entry::ProductDetails;
pub use product_entry::{ProductEntry, ProductEntryError, ProductTaxOverrides, ResolvedProductTax};
pub use product_list::{PriceField, ProductList, ProductListRow, ProductText};
pub use reorder::{ReorderError, ReorderRow, ReorderStatus};
pub use stock_list::{StockList, StockListRow, StockStatus};
pub use stock_lots::{StockLotRow, StockLots, StockLotsError};

#[derive(Clone)]
#[cfg_attr(any(test, feature = "test-support"), derive(Debug, Eq, PartialEq))]
#[cfg_attr(feature = "demo-fixture", derive(serde::Serialize, serde::Deserialize))]
struct Product {
    stock: u32,
    seeded_opening: u32,
    pricing: ProductPricing,
    details: Option<ProductDetails>,
    lots: Vec<Lot>,
    physical_count: PhysicalCountState,
}

impl Product {
    fn item_name(&self) -> Option<&str> {
        self.details.as_ref().map(|details| details.item.as_str())
    }

    fn new(stock: u32, pricing: ProductPricing) -> Self {
        Self {
            stock,
            seeded_opening: stock,
            pricing,
            details: None,
            lots: vec![Lot {
                remaining: stock,
                unit_cost: None,
                date: None,
            }],
            physical_count: PhysicalCountState::default(),
        }
    }

    fn fifo_lot_indices(&self) -> Vec<usize> {
        // Sort indices, never storage: allocations refer to the original indices.
        let mut indices: Vec<_> = (0..self.lots.len()).collect();
        indices.sort_by_key(|&index| self.lots[index].date.map(crate::BillDate::parts));
        indices
    }
}

#[derive(Clone)]
#[cfg_attr(any(test, feature = "test-support"), derive(Debug, Eq, PartialEq))]
#[cfg_attr(feature = "demo-fixture", derive(serde::Serialize, serde::Deserialize))]
struct Lot {
    remaining: u32,
    unit_cost: Option<i64>,
    date: Option<crate::BillDate>,
}

#[derive(Clone)]
#[cfg_attr(any(test, feature = "test-support"), derive(Debug, Eq, PartialEq))]
#[cfg_attr(feature = "demo-fixture", derive(serde::Serialize, serde::Deserialize))]
struct AllocationState {
    #[cfg_attr(
        feature = "demo-fixture",
        serde(with = "crate::confirmation::serde_pairs::pairs")
    )]
    records: BTreeMap<(DraftId, usize, u64, usize), Allocation>,
    sale_costs: BTreeMap<DraftId, Option<i64>>,
    /// Per-line unit cost frozen by the first in-place edit (`costAtSale`), in paise.
    #[cfg_attr(
        feature = "demo-fixture",
        serde(with = "crate::confirmation::serde_pairs::pairs")
    )]
    line_units: BTreeMap<(DraftId, usize), Option<i64>>,
}

type Allocation = (u32, Option<i64>);

/// A bill's checked stock and lot moves, applied by [`Catalog::commit_sale`].
pub(super) struct StagedSale {
    products: BTreeMap<u64, Product>,
    records: Vec<((DraftId, usize, u64, usize), Allocation)>,
    sale: DraftId,
    cost: Option<i64>,
}

#[derive(Clone)]
#[cfg_attr(any(test, feature = "test-support"), derive(Debug, Eq, PartialEq))]
#[cfg_attr(feature = "demo-fixture", derive(serde::Serialize, serde::Deserialize))]
pub struct Catalog {
    products: BTreeMap<u64, Product>,
    product_order: Vec<ProductId>,
    allocations: AllocationState,
}

impl Catalog {
    pub(super) fn dead_stock_catalog(
        &self,
    ) -> impl Iterator<Item = (ProductId, u32, Option<&str>, Money)> + '_ {
        self.product_order.iter().filter_map(|&id| {
            let product = self.products.get(&id.value())?;
            Some((
                id,
                product.stock,
                product.item_name(),
                product.pricing.buy(),
            ))
        })
    }

    pub(super) fn new(id: ProductId, stock: u32, price: Money) -> Self {
        Self {
            products: BTreeMap::from([(
                id.value(),
                Product::new(stock, ProductPricing::at_sell(price)),
            )]),
            product_order: vec![id],
            allocations: AllocationState {
                records: BTreeMap::new(),
                sale_costs: BTreeMap::new(),
                line_units: BTreeMap::new(),
            },
        }
    }

    fn add(&mut self, id: ProductId, product: Product) -> Result<(), ConfirmationError> {
        if product.pricing.sell().minor_units() < 0 {
            return Err(ConfirmationError::InvalidAmount);
        }
        if self.products.contains_key(&id.value()) {
            return Err(ConfirmationError::InvalidDraft);
        }
        self.products.insert(id.value(), product);
        self.product_order.push(id);
        Ok(())
    }

    pub(super) fn set_price(
        &mut self,
        id: ProductId,
        price: Money,
    ) -> Result<(), ConfirmationError> {
        if price.minor_units() < 0 {
            return Err(ConfirmationError::InvalidAmount);
        }
        self.products
            .get_mut(&id.value())
            .ok_or(ConfirmationError::InvalidDraft)?
            .pricing
            .apply(ProductPriceInput::Sell(price))
    }

    pub(super) fn stock(&self, id: ProductId) -> Option<u32> {
        self.products.get(&id.value()).map(|product| product.stock)
    }

    pub(super) fn sell_rate(&self, id: ProductId) -> Option<Money> {
        self.products
            .get(&id.value())
            .map(|product| product.pricing.sell())
    }

    pub(super) fn item_name(&self, id: ProductId) -> Option<&str> {
        self.products.get(&id.value()).and_then(Product::item_name)
    }

    pub(super) fn has_product(&self, id: ProductId) -> bool {
        self.products.contains_key(&id.value())
    }

    /// The product's current sell price, as `fillGstItem` reads `p.sellRate`.
    pub(super) fn sell_price(&self, id: ProductId) -> Money {
        self.products
            .get(&id.value())
            .map_or(Money::from_minor_units(0), |product| product.pricing.sell())
    }

    pub(super) fn product_id_by_exact_name(&self, item: &str) -> Option<ProductId> {
        self.products.iter().find_map(|(&id, product)| {
            (product.item_name() == Some(item)).then_some(ProductId::new(id))
        })
    }

    pub(super) fn prices(&self, draft: &ProvisionalDraft) -> Result<Vec<Money>, ConfirmationError> {
        if draft.lines().is_empty() {
            return Err(ConfirmationError::InvalidDraft);
        }
        draft
            .lines()
            .iter()
            .map(|line| {
                self.sell_rate(line.product_id())
                    .ok_or(ConfirmationError::InvalidDraft)
            })
            .collect()
    }

    /// A bill's stock and lot moves, checked against a copy of the products only; the
    /// allocation records are not copied (they grow with every bill), only added on commit.
    pub(super) fn stage_sale(
        &self,
        review: &Review,
    ) -> Result<(StagedSale, u32, i64), ConfirmationError> {
        let mut products = self.products.clone();
        let mut records = Vec::new();
        let amounts = review.amounts()?;
        let mut sale_cost = 0_i64;
        let mut cost_known = true;
        // Ordinary lines first, then every crate component as its own demand (OPEN-010).
        let ordinary = review.draft.lines().len();
        let crate_demands = review.crate_packet_demands()?.unwrap_or_default();
        let lines = review.draft.lines().iter().chain(&crate_demands);
        for (line_index, line) in lines.enumerate() {
            let packets = if line_index < ordinary {
                review.line_values(line_index)?.0
            } else {
                line.quantity().get()
            };
            let product = products
                .get_mut(&line.product_id().value())
                .ok_or(ConfirmationError::InvalidDraft)?;
            product.stock = product
                .stock
                .checked_sub(packets)
                .ok_or(ConfirmationError::InsufficientStock)?;
            let mut remaining = packets;
            for lot_index in product.fifo_lot_indices() {
                let lot = &mut product.lots[lot_index];
                let taken = remaining.min(lot.remaining);
                if taken == 0 {
                    continue;
                }
                let cost = match lot.unit_cost {
                    Some(unit_cost) => {
                        let cost = unit_cost
                            .checked_mul(i64::from(taken))
                            .ok_or(ConfirmationError::Overflow)?;
                        sale_cost = sale_cost
                            .checked_add(cost)
                            .ok_or(ConfirmationError::Overflow)?;
                        Some(cost)
                    }
                    None => {
                        cost_known = false;
                        None
                    }
                };
                lot.remaining = lot
                    .remaining
                    .checked_sub(taken)
                    .ok_or(ConfirmationError::Overflow)?;
                remaining = remaining
                    .checked_sub(taken)
                    .ok_or(ConfirmationError::Overflow)?;
                records.push((
                    (
                        review.draft.id(),
                        line_index,
                        line.product_id().value(),
                        lot_index,
                    ),
                    (taken, cost),
                ));
                if remaining == 0 {
                    break;
                }
            }
            if remaining != 0 {
                return Err(ConfirmationError::InsufficientStock);
            }
        }
        Ok((
            StagedSale {
                products,
                records,
                sale: review.draft.id(),
                cost: cost_known.then_some(sale_cost),
            },
            amounts.packets,
            amounts.total.minor_units(),
        ))
    }

    /// Applies a staged bill.
    pub(super) fn commit_sale(&mut self, staged: StagedSale) {
        self.products = staged.products;
        self.allocations.records.extend(staged.records);
        self.allocations.sale_costs.insert(staged.sale, staged.cost);
    }

    /// [`Self::stage_sale`] applied to a copy of the whole catalog.
    pub(super) fn stage(&self, review: &Review) -> Result<(Self, u32, i64), ConfirmationError> {
        let (sale, quantity, total) = self.stage_sale(review)?;
        let mut staged = self.clone();
        staged.commit_sale(sale);
        Ok((staged, quantity, total))
    }

    pub fn seed_lots(
        &mut self,
        id: ProductId,
        lots: &[(u32, Money)],
    ) -> Result<(), ConfirmationError> {
        let mut stock = 0_u32;
        let mut ordered = Vec::new();
        for &(quantity, cost) in lots {
            let unit_cost = cost.minor_units();
            if unit_cost < 0 {
                return Err(ConfirmationError::InvalidAmount);
            }
            stock = stock
                .checked_add(quantity)
                .ok_or(ConfirmationError::Overflow)?;
            ordered.push(Lot {
                remaining: quantity,
                unit_cost: Some(unit_cost),
                date: None,
            });
        }
        let product = self
            .products
            .get_mut(&id.value())
            .ok_or(ConfirmationError::InvalidDraft)?;
        product.stock = stock;
        product.seeded_opening = stock;
        product.lots = ordered;
        Ok(())
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn lot_snapshot(&self, id: ProductId) -> Vec<u32> {
        self.products[&id.value()]
            .lots
            .iter()
            .map(|lot| lot.remaining)
            .collect()
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn allocation_snapshot(&self) -> Vec<(u64, usize, usize, u32, Option<i64>)> {
        self.allocations
            .records
            .iter()
            .map(|(&(draft, line, _, lot), &(quantity, cost))| {
                (draft.value(), line, lot, quantity, cost)
            })
            .collect()
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn allocated_cost(&self, id: super::DraftId) -> Option<Option<i64>> {
        self.allocations.sale_costs.get(&id).copied()
    }
}

impl super::ConfirmationFixture {
    /// Looks up the current catalog name without case folding or trimming the query.
    pub fn find_product_by_exact_name(
        &self,
        role: crate::Role,
        item: &str,
    ) -> Result<Option<ProductId>, ProductEntryError> {
        product_entry::owner(role)?;
        Ok(self.catalog.product_id_by_exact_name(item))
    }
}
