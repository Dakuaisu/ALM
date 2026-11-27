use std::fmt::Write;
use std::num::NonZeroU64;

use crate::confirmation::ConfirmationFixture;
use crate::confirmation::estimate::display_date;
use crate::confirmation::image_handoff::{format_amount, group_indian_digits};
use crate::{BillDate, Money, ProductId, Role};

use super::Catalog;

#[cfg(test)]
mod owner_projection;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReorderError {
    AccessDenied,
    Overflow,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReorderStatus {
    Out,
    Low,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReorderRow {
    pub product: ProductId,
    pub item: String,
    pub brand: String,
    pub on_hand: u32,
    pub reorder_level: u32,
    pub suggested_quantity: NonZeroU64,
    pub best_supplier: Option<String>,
    pub unit_rate: Money,
    pub estimated_cost: Money,
    pub status: ReorderStatus,
}

impl ReorderRow {
    pub fn estimated_cost_display(&self) -> String {
        let cents = self.estimated_cost.minor_units();
        let rupees = cents.div_euclid(100) + i64::from(cents.rem_euclid(100) >= 50);
        let sign = if rupees < 0 { "-" } else { "" };
        format!("₹{sign}{}", group_indian_digits(rupees.unsigned_abs()))
    }
}

struct Candidate<'a> {
    product: ProductId,
    item: &'a str,
    brand: &'a str,
    on_hand: u32,
    reorder_level: u32,
    suggested_quantity: NonZeroU64,
    buy_rate: Money,
}

impl Catalog {
    pub(in crate::confirmation) fn order_planner_stock(
        &self,
        default_reorder: u32,
    ) -> impl Iterator<Item = (ProductId, u32, u32)> + '_ {
        self.product_order.iter().filter_map(move |&id| {
            let product = self.products.get(&id.value())?;
            let reorder_level = product
                .details
                .as_ref()
                .map_or(default_reorder, |details| details.reorder_level);
            Some((id, product.stock, reorder_level))
        })
    }
}

fn gate(role: Role) -> Result<(), super::ReorderError> {
    crate::permissions::require(role, crate::permissions::Capability::Reorder)
        .map_err(|_| ReorderError::AccessDenied)
}

impl ConfirmationFixture {
    /// Projects billing stock and Owner-only costs without changing the fixture.
    ///
    /// # Errors
    /// Denies Managers before reading stock and rejects costs that cannot fit in `Money`.
    pub fn open_reorder_planner(&self, role: Role) -> Result<Vec<super::ReorderRow>, ReorderError> {
        gate(role)?;
        let columns = self.purchases.price_suppliers();
        // DEC-034: a Manager's reorder list holds quantities only, no supplier or cost.
        let owner = role == Role::Owner;
        self.reorder_candidates()
            .into_iter()
            .map(|candidate| {
                let quote = self
                    .purchases
                    .reorder_quote(candidate.product, &columns)
                    .filter(|_| owner);
                let unit_rate = quote.map_or(candidate.buy_rate, |(_, rate)| rate);
                let unit_rate = if owner {
                    unit_rate
                } else {
                    Money::from_minor_units(0)
                };
                let quantity = i64::try_from(candidate.suggested_quantity.get())
                    .map_err(|_| ReorderError::Overflow)?;
                let estimated_cost = unit_rate
                    .minor_units()
                    .checked_mul(quantity)
                    .ok_or(ReorderError::Overflow)?;
                Ok(ReorderRow {
                    product: candidate.product,
                    item: candidate.item.to_owned(),
                    brand: candidate.brand.to_owned(),
                    on_hand: candidate.on_hand,
                    reorder_level: candidate.reorder_level,
                    suggested_quantity: candidate.suggested_quantity,
                    best_supplier: quote.map(|(name, _)| name.to_owned()),
                    unit_rate,
                    estimated_cost: Money::from_minor_units(estimated_cost),
                    status: if candidate.on_hand == 0 {
                        super::ReorderStatus::Out
                    } else {
                        super::ReorderStatus::Low
                    },
                })
            })
            .collect()
    }

    /// Sends the exact Copy List text once to a supplied capture callback, including an empty list.
    ///
    /// # Errors
    /// Denies Managers without invoking the callback.
    pub fn copy_reorder_list(
        &self,
        role: Role,
        today: BillDate,
        sink: &mut impl FnMut(String),
    ) -> Result<(), ReorderError> {
        gate(role)?;
        let columns = self.purchases.price_suppliers();
        let mut text = format!("*ALM Reorder List* ({})\n\n", display_date(today));
        for candidate in self.reorder_candidates() {
            let quantity = group_indian_digits(candidate.suggested_quantity.get());
            let _ = write!(text, "{} — {quantity} pkt", candidate.item);
            if let Some((name, rate)) = self
                .purchases
                .reorder_quote(candidate.product, &columns)
                .filter(|_| role == Role::Owner)
            {
                let _ = write!(text, " ({name} ₹{})", format_amount(rate));
            }
            text.push('\n');
        }
        sink(text);
        Ok(())
    }

    fn reorder_candidates(&self) -> Vec<Candidate<'_>> {
        let mut candidates: Vec<_> = self
            .catalog
            .product_order
            .iter()
            .filter_map(|&id| {
                let product = self.catalog.products.get(&id.value())?;
                let details = product.details.as_ref()?;
                let reorder_level = details.reorder_level;
                let on_hand = product.stock;
                if on_hand > reorder_level {
                    return None;
                }
                let (item, buy_rate) = self.supplier_price_product(id)?;
                let refill = u64::from(reorder_level) * 3 - u64::from(on_hand);
                Some(Candidate {
                    product: id,
                    item,
                    brand: &details.brand,
                    on_hand,
                    reorder_level,
                    suggested_quantity: NonZeroU64::new(refill).unwrap_or(NonZeroU64::MIN),
                    buy_rate,
                })
            })
            .collect();
        candidates.sort_by_key(|candidate| candidate.on_hand);
        candidates
    }
}
