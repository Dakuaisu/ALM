use std::collections::BTreeMap;

use super::{ConfirmationError, Money, ProductId, ProvisionalDraft, Review};

mod cancellation;
mod edit;
mod receiving;

mod pricing;
mod product_entry;

pub use pricing::{ProductPriceInput, ProductPricing};
use product_entry::ProductDetails;
pub use product_entry::{ProductEntry, ProductEntryError, ProductTaxOverrides, ResolvedProductTax};

#[derive(Clone)]
#[cfg_attr(test, derive(Debug, Eq, PartialEq))]
struct Product {
    stock: u32,
    pricing: ProductPricing,
    details: Option<ProductDetails>,
    lots: Vec<Lot>,
}

impl Product {
    fn item_name(&self) -> Option<&str> {
        self.details.as_ref().map(|details| details.item.as_str())
    }

    fn new(stock: u32, pricing: ProductPricing) -> Self {
        Self {
            stock,
            pricing,
            details: None,
            lots: vec![Lot {
                remaining: stock,
                unit_cost: None,
                date: None,
            }],
        }
    }
}

#[derive(Clone)]
#[cfg_attr(test, derive(Debug, Eq, PartialEq))]
struct Lot {
    remaining: u32,
    unit_cost: Option<i64>,
    date: Option<crate::BillDate>,
}

#[derive(Clone)]
#[cfg_attr(test, derive(Debug, Eq, PartialEq))]
struct AllocationState {
    records: BTreeMap<(u64, usize, u64, usize), Allocation>,
    sale_costs: BTreeMap<u64, Option<i64>>,
}

type Allocation = (u32, Option<i64>);

#[derive(Clone)]
#[cfg_attr(test, derive(Debug, Eq, PartialEq))]
pub(super) struct Catalog {
    products: BTreeMap<u64, Product>,
    allocations: AllocationState,
}

impl Catalog {
    pub(super) fn new(id: ProductId, stock: u32, price: Money) -> Self {
        Self {
            products: BTreeMap::from([(
                id.value(),
                Product::new(stock, ProductPricing::at_sell(price)),
            )]),
            allocations: AllocationState {
                records: BTreeMap::new(),
                sale_costs: BTreeMap::new(),
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

    pub(super) fn item_name(&self, id: ProductId) -> Option<&str> {
        self.products.get(&id.value()).and_then(Product::item_name)
    }

    pub(super) fn prices(&self, draft: &ProvisionalDraft) -> Result<Vec<Money>, ConfirmationError> {
        if draft.lines().is_empty() || draft.crate_row().is_some() {
            return Err(ConfirmationError::InvalidDraft);
        }
        draft
            .lines()
            .iter()
            .map(|line| {
                self.products
                    .get(&line.product_id().value())
                    .map(|p| p.pricing.sell())
                    .ok_or(ConfirmationError::InvalidDraft)
            })
            .collect()
    }

    pub(super) fn stage(&self, review: &Review) -> Result<(Self, u32, i64), ConfirmationError> {
        let mut staged = self.clone();
        let amounts = review.amounts()?;
        let mut sale_cost = 0_i64;
        let mut cost_known = true;
        let crate_demands = review.crate_packet_demands()?;
        let lines = crate_demands.as_deref().unwrap_or(review.draft.lines());
        for (line_index, line) in lines.iter().enumerate() {
            let packets = if crate_demands.is_some() {
                line.quantity().get()
            } else {
                review.line_values(line_index)?.0
            };
            let product = staged
                .products
                .get_mut(&line.product_id().value())
                .ok_or(ConfirmationError::InvalidDraft)?;
            product.stock = product
                .stock
                .checked_sub(packets)
                .ok_or(ConfirmationError::InsufficientStock)?;
            let mut remaining = packets;
            // Sort indices, never the lots: stored allocation indices must remain stable.
            let mut indices: Vec<_> = (0..product.lots.len()).collect();
            indices.sort_by_key(|&index| product.lots[index].date.map(crate::BillDate::parts));
            for lot_index in indices {
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
                staged.allocations.records.insert(
                    (
                        review.draft.id().value(),
                        line_index,
                        line.product_id().value(),
                        lot_index,
                    ),
                    (taken, cost),
                );
                if remaining == 0 {
                    break;
                }
            }
            if remaining != 0 {
                return Err(ConfirmationError::InsufficientStock);
            }
        }
        staged
            .allocations
            .sale_costs
            .insert(review.draft.id().value(), cost_known.then_some(sale_cost));
        Ok((staged, amounts.packets, amounts.total.minor_units()))
    }

    pub(super) fn seed_lots(
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
        product.lots = ordered;
        Ok(())
    }

    #[cfg(test)]
    pub(super) fn lot_snapshot(&self, id: ProductId) -> Vec<u32> {
        self.products[&id.value()]
            .lots
            .iter()
            .map(|lot| lot.remaining)
            .collect()
    }

    #[cfg(test)]
    pub(super) fn allocation_snapshot(&self) -> Vec<(u64, usize, usize, u32, Option<i64>)> {
        self.allocations
            .records
            .iter()
            .map(|(&(draft, line, _, lot), &(quantity, cost))| (draft, line, lot, quantity, cost))
            .collect()
    }

    #[cfg(test)]
    pub(super) fn allocated_cost(&self, id: super::DraftId) -> Option<Option<i64>> {
        self.allocations.sale_costs.get(&id.value()).copied()
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
        Ok(self.catalog.products.iter().find_map(|(&id, product)| {
            (product.item_name() == Some(item)).then_some(ProductId::new(id))
        }))
    }
}
