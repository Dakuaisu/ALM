use std::collections::BTreeMap;

use super::{ConfirmationError, ConfirmationFixture, Money, Role};
use crate::{BillDate, ProductId};

/// Current catalog stock valued at the current buy rate, not FIFO cost.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeadStockRow {
    pub product: ProductId,
    pub item: Option<String>,
    pub on_hand: u32,
    pub last_sale: Option<BillDate>,
    pub buy_rate: Money,
    pub locked_value: Money,
}

impl ConfirmationFixture {
    /// Projects stock last sold strictly before an already-resolved calendar cutoff.
    /// No clock or timezone conversion is performed. Ties preserve catalog order.
    ///
    /// # Errors
    /// Denies Managers and rejects inconsistent sale state or overflowing values.
    pub fn dead_stock_report(
        &self,
        role: Role,
        cutoff: BillDate,
    ) -> Result<Vec<DeadStockRow>, ConfirmationError> {
        if role != Role::Owner {
            return Err(ConfirmationError::AccessDenied);
        }
        let mut dates = BTreeMap::new();
        for (id, (review, _, _)) in &self.current_sales {
            if self
                .lifecycle
                .get(id)
                .ok_or(ConfirmationError::InvalidDraft)?
                .0
            {
                continue;
            }
            let Some(date) = review.draft.bill_date() else {
                continue;
            };
            let items = self
                .posted_sale_items
                .get(id)
                .ok_or(ConfirmationError::InvalidDraft)?;
            for item in items {
                dates
                    .entry(item.product_id.value())
                    .and_modify(|prior: &mut BillDate| {
                        if date.parts() > prior.parts() {
                            *prior = date;
                        }
                    })
                    .or_insert(date);
            }
        }
        let mut rows = Vec::new();
        for (product, on_hand, item, buy_rate) in self.catalog.dead_stock_catalog() {
            let last_sale = dates.get(&product.value()).copied();
            if on_hand == 0 || last_sale.is_some_and(|date| date.parts() >= cutoff.parts()) {
                continue;
            }
            let value = buy_rate
                .minor_units()
                .checked_mul(i64::from(on_hand))
                .ok_or(ConfirmationError::Overflow)?;
            rows.push(DeadStockRow {
                product,
                item: item.map(str::to_owned),
                on_hand,
                last_sale,
                buy_rate,
                locked_value: Money::from_minor_units(value),
            });
        }
        rows.sort_by_key(|row| std::cmp::Reverse(row.locked_value.minor_units()));
        rows.truncate(15);
        Ok(rows)
    }
}
