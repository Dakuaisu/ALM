//! In-place quantity edit of a posted bill line (legacy `editBill` → `saveBill`). Stock and lots
//! follow the new quantity: extra packets are taken oldest lot first, fewer packets go back to
//! this line's newest lots first. Profit keeps the line's frozen unit cost
//! (`costAtSale = round(cost / qty, 2)`) times the new quantity, as the HTML does.
use super::{Catalog, ConfirmationError};

/// Rounds `cost / qty` to whole paise, halves away from zero (`Math.round` on positive costs).
fn unit_cost(cost: i64, quantity: u32) -> Option<i64> {
    let qty = i64::from(quantity);
    if qty == 0 {
        return None;
    }
    Some((cost * 2 + qty).div_euclid(qty * 2))
}

impl Catalog {
    /// Moves `line` of sale `sale` from `from` to `to` packets and refreezes the sale cost from
    /// the per-line frozen unit costs of `quantities` (every ordinary line, in order).
    pub(in crate::confirmation) fn stage_line_edit(
        &self,
        sale: u64,
        lines: &[(u64, u32, u32)],
    ) -> Result<Self, ConfirmationError> {
        let mut staged = self.clone();
        let mut total = Some(0_i64);
        for (line, &(product, from, to)) in lines.iter().enumerate() {
            let line_cost = self.line_cost(sale, line);
            let frozen = match self.allocations.line_units.get(&(sale, line)) {
                Some(unit) => *unit,
                None => line_cost.and_then(|cost| unit_cost(cost, from)),
            };
            staged.allocations.line_units.insert((sale, line), frozen);
            if to > from {
                staged.take(sale, line, product, to - from)?;
            } else if to < from {
                staged.release(sale, line, product, from - to)?;
            }
            total = match (total, frozen) {
                (Some(sum), Some(unit)) => Some(
                    unit.checked_mul(i64::from(to))
                        .and_then(|cost| sum.checked_add(cost))
                        .ok_or(ConfirmationError::Overflow)?,
                ),
                _ => None,
            };
        }
        staged.allocations.sale_costs.insert(sale, total);
        Ok(staged)
    }

    fn line_cost(&self, sale: u64, line: usize) -> Option<i64> {
        self.allocations
            .records
            .iter()
            .filter(|((s, l, _, _), _)| *s == sale && *l == line)
            .try_fold(0_i64, |sum, (_, (_, cost))| sum.checked_add((*cost)?))
    }

    fn take(
        &mut self,
        sale: u64,
        line: usize,
        product: u64,
        packets: u32,
    ) -> Result<(), ConfirmationError> {
        let item = self
            .products
            .get_mut(&product)
            .ok_or(ConfirmationError::InvalidDraft)?;
        item.stock = item
            .stock
            .checked_sub(packets)
            .ok_or(ConfirmationError::InsufficientStock)?;
        let mut remaining = packets;
        for lot_index in item.fifo_lot_indices() {
            let lot = &mut item.lots[lot_index];
            let taken = remaining.min(lot.remaining);
            if taken == 0 {
                continue;
            }
            lot.remaining -= taken;
            remaining -= taken;
            let cost = lot
                .unit_cost
                .map(|unit| {
                    unit.checked_mul(i64::from(taken))
                        .ok_or(ConfirmationError::Overflow)
                })
                .transpose()?;
            let entry = self
                .allocations
                .records
                .entry((sale, line, product, lot_index))
                .or_insert((0, Some(0)));
            entry.0 = entry
                .0
                .checked_add(taken)
                .ok_or(ConfirmationError::Overflow)?;
            entry.1 = match (entry.1, cost) {
                (Some(a), Some(b)) => Some(a.checked_add(b).ok_or(ConfirmationError::Overflow)?),
                _ => None,
            };
            if remaining == 0 {
                return Ok(());
            }
        }
        Err(ConfirmationError::InsufficientStock)
    }

    fn release(
        &mut self,
        sale: u64,
        line: usize,
        product: u64,
        packets: u32,
    ) -> Result<(), ConfirmationError> {
        let item = self
            .products
            .get_mut(&product)
            .ok_or(ConfirmationError::InvalidDraft)?;
        let mut order = item.fifo_lot_indices();
        order.reverse();
        let mut remaining = packets;
        for lot_index in order {
            let key = (sale, line, product, lot_index);
            let Some(&(held, cost)) = self.allocations.records.get(&key) else {
                continue;
            };
            let back = remaining.min(held);
            let lot = &mut item.lots[lot_index];
            lot.remaining = lot
                .remaining
                .checked_add(back)
                .ok_or(ConfirmationError::Overflow)?;
            item.stock = item
                .stock
                .checked_add(back)
                .ok_or(ConfirmationError::Overflow)?;
            remaining -= back;
            if back == held {
                self.allocations.records.remove(&key);
            } else {
                let unit = lot.unit_cost;
                let kept = held - back;
                let cost = match (cost, unit) {
                    (Some(_), Some(unit)) => Some(unit * i64::from(kept)),
                    _ => None,
                };
                self.allocations.records.insert(key, (kept, cost));
            }
            if remaining == 0 {
                return Ok(());
            }
        }
        Err(ConfirmationError::InvalidDraft)
    }
}

#[cfg(test)]
mod tests {
    use super::unit_cost;

    #[test]
    fn frozen_unit_cost_rounds_like_math_round() {
        assert_eq!(unit_cost(7_000, 1), Some(7_000));
        assert_eq!(unit_cost(1_001, 2), Some(501));
        assert_eq!(unit_cost(1_003, 3), Some(334));
        assert_eq!(unit_cost(5, 0), None);
    }
}
