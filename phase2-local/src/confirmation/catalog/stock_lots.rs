use std::num::NonZeroU32;

use crate::confirmation::ConfirmationFixture;
use crate::{BillDate, Money, ProductId, Role};

/// One active FIFO layer. The catalog does not retain original quantities or source references.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StockLotRow {
    /// Undated layers have no receipt date.
    pub date: Option<BillDate>,
    pub remaining: NonZeroU32,
    pub unit_cost: Option<Money>,
    /// Remaining quantity times unit cost, or `None` if the cost is unknown.
    pub value: Option<Money>,
}

/// Read-only snapshot of stock on hand and active lot values for a product.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StockLots {
    /// Catalog on-hand quantity, independent of valuation.
    pub on_hand: u32,
    /// FIFO order: undated layers first, then oldest date first, with stored order on ties.
    pub rows: Vec<StockLotRow>,
    /// Sum of active lot values; `None` if any active cost is unknown. Empty stock is worth zero.
    pub total_value: Option<Money>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StockLotsError {
    AccessDenied,
    ProductNotFound,
    Overflow,
}

impl ConfirmationFixture {
    /// Opens the synthetic Stock Lots projection without changing the fixture.
    ///
    /// # Errors
    /// Denies Manager before checking product existence. For Owner, returns an error if the
    /// product is missing or a known lot value or sum of known values overflows.
    pub fn open_stock_lots(&self, role: Role, id: ProductId) -> Result<StockLots, StockLotsError> {
        match role {
            Role::Owner => (),
            Role::Manager => return Err(StockLotsError::AccessDenied),
        }
        let product = self
            .catalog
            .products
            .get(&id.value())
            .ok_or(StockLotsError::ProductNotFound)?;
        let mut rows = Vec::new();
        let mut total = 0_i64;
        let mut all_costs_known = true;
        for index in product.fifo_lot_indices() {
            let lot = &product.lots[index];
            let Some(remaining) = NonZeroU32::new(lot.remaining) else {
                continue;
            };
            let value = match lot.unit_cost {
                Some(cost) => {
                    let value = cost
                        .checked_mul(i64::from(remaining.get()))
                        .ok_or(StockLotsError::Overflow)?;
                    total = total.checked_add(value).ok_or(StockLotsError::Overflow)?;
                    Some(Money::from_minor_units(value))
                }
                None => {
                    all_costs_known = false;
                    None
                }
            };
            rows.push(StockLotRow {
                date: lot.date,
                remaining,
                unit_cost: lot.unit_cost.map(Money::from_minor_units),
                value,
            });
        }
        Ok(StockLots {
            on_hand: product.stock,
            rows,
            total_value: all_costs_known.then_some(Money::from_minor_units(total)),
        })
    }
}
