//! Stock screen projection (legacy VIEWS.stock): billing stock, reorder status and FIFO value.
use crate::confirmation::ConfirmationFixture;
use crate::{Money, ProductId, Role};

use super::StockLotsError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StockStatus {
    Out,
    Low,
    Ok,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StockListRow {
    pub product: ProductId,
    pub item: String,
    pub brand: String,
    pub on_hand: u32,
    pub reorder_level: u32,
    pub status: StockStatus,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StockList {
    /// Filtered rows, lowest stock first (stable on catalog order).
    pub rows: Vec<StockListRow>,
    pub skus: usize,
    pub in_stock: usize,
    /// At or below reorder level but not out.
    pub low: usize,
    pub out: usize,
    /// Sum of FIFO lot values for every product; `None` if any active lot cost is unknown.
    pub value: Option<Money>,
}

impl ConfirmationFixture {
    /// Owner-only stock list. `query` matches item names case-insensitively; `low_only` keeps
    /// rows at or below their reorder level. Counts and value cover every product.
    ///
    /// # Errors
    /// Denies Managers before reading and rejects value overflow.
    pub fn stock_list(
        &self,
        role: Role,
        query: &str,
        low_only: bool,
    ) -> Result<StockList, StockLotsError> {
        if role != Role::Owner {
            return Err(StockLotsError::AccessDenied);
        }
        let needle = query.trim().to_lowercase();
        let mut list = StockList {
            rows: Vec::new(),
            skus: 0,
            in_stock: 0,
            low: 0,
            out: 0,
            value: Some(Money::from_minor_units(0)),
        };
        let stock: Vec<_> = self
            .catalog
            .order_planner_stock(self.product_reorder_default)
            .collect();
        for (product, on_hand, reorder_level) in stock {
            let status = if on_hand == 0 {
                StockStatus::Out
            } else if on_hand <= reorder_level {
                StockStatus::Low
            } else {
                StockStatus::Ok
            };
            list.skus += 1;
            match status {
                StockStatus::Out => list.out += 1,
                StockStatus::Low => list.low += 1,
                StockStatus::Ok => {}
            }
            if on_hand > 0 {
                list.in_stock += 1;
            }
            let lots = self.open_stock_lots(role, product)?;
            list.value = match (list.value, lots.total_value) {
                (Some(sum), Some(value)) => Some(Money::from_minor_units(
                    sum.minor_units()
                        .checked_add(value.minor_units())
                        .ok_or(StockLotsError::Overflow)?,
                )),
                _ => None,
            };
            let item = self
                .catalog
                .item_name(product)
                .unwrap_or_default()
                .to_owned();
            if !item.to_lowercase().contains(&needle) || (low_only && on_hand > reorder_level) {
                continue;
            }
            let brand = self
                .catalog
                .products
                .get(&product.value())
                .and_then(|p| p.details.as_ref())
                .map(|details| details.brand.clone())
                .unwrap_or_default();
            list.rows.push(StockListRow {
                product,
                item,
                brand,
                on_hand,
                reorder_level,
                status,
            });
        }
        list.rows.sort_by_key(|row| row.on_hand);
        Ok(list)
    }
}

#[cfg(test)]
mod tests {
    use super::StockStatus;
    use crate::confirmation::StockLotsError;
    use crate::confirmation::demo_fixture::fixture;
    use crate::{ProductId, Role};

    #[test]
    fn stock_list_counts_status_and_fifo_value_without_effects() {
        let mut fixture = fixture().unwrap();
        let mut form = fixture
            .open_product_entry(Role::Owner, ProductId::new(2))
            .unwrap();
        form.reorder_level = Some(40);
        fixture.save_product_entry(Role::Owner, &form).unwrap();
        let before = fixture.clone();
        let list = fixture.stock_list(Role::Owner, "", false).unwrap();
        assert_eq!(fixture, before);
        let rows: Vec<_> = list
            .rows
            .iter()
            .map(|row| (row.item.as_str(), row.on_hand, row.status))
            .collect();
        assert_eq!(
            rows,
            [
                ("Synthetic Soda", 40, StockStatus::Low),
                ("Synthetic Lime", 50, StockStatus::Ok)
            ]
        );
        assert_eq!((list.skus, list.in_stock, list.low, list.out), (2, 2, 1, 0));
        assert_eq!(list.value.unwrap().minor_units(), 50 * 7_000 + 40 * 3_800);
        let low = fixture.stock_list(Role::Owner, " soda", true).unwrap();
        assert_eq!(low.rows.len(), 1);
        assert_eq!(low.value, list.value);
        assert!(
            fixture
                .stock_list(Role::Owner, "lime", true)
                .unwrap()
                .rows
                .is_empty()
        );
        assert_eq!(
            fixture.stock_list(Role::Manager, "", false),
            Err(StockLotsError::AccessDenied)
        );
    }
}
