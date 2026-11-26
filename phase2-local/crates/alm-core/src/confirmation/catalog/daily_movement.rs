use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::num::NonZeroI64;

use crate::confirmation::ConfirmationFixture;
use crate::{BillDate, ProductId, Role};

/// One product's daily billing movements, in whole packets. Location counts are not movements.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DailyMovementRow<'a> {
    pub product: ProductId,
    pub item: Option<&'a str>,
    pub opening: i64,
    /// PO receipts dated on the selected day, before adjustment folding.
    pub received: i64,
    /// Active sale packets dated on the selected day, before adjustment folding.
    pub dispatched: i64,
    pub adjustment: i64,
    /// Received and positive net adjustments, shown with `+`; `None` is a dash.
    pub display_received: Option<NonZeroI64>,
    /// Dispatched and the magnitude of negative net adjustments, shown with `−`;
    /// `None` is a dash.
    pub display_dispatched: Option<NonZeroI64>,
    pub closing: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DailyMovementError {
    AccessDenied,
    MissingSaleDate,
    Overflow,
}

#[derive(Default)]
struct Movements {
    received: i64,
    dispatched: i64,
    adjustment: i64,
}

#[derive(Default)]
struct ProductMovements {
    before: Movements,
    on: Movements,
}

enum Movement {
    Receipt(u32),
    Dispatch(u32),
    Adjustment(i64),
}

struct Register {
    day: BillDate,
    totals: BTreeMap<u64, ProductMovements>,
}

impl Register {
    fn record(
        &mut self,
        product: u64,
        event: (BillDate, Movement),
    ) -> Result<(), DailyMovementError> {
        let (date, movement) = event;
        let totals = match date.parts().cmp(&self.day.parts()) {
            Ordering::Less => &mut self.totals.entry(product).or_default().before,
            Ordering::Equal => &mut self.totals.entry(product).or_default().on,
            Ordering::Greater => return Ok(()),
        };
        let (bucket, delta) = match movement {
            Movement::Receipt(packets) => (&mut totals.received, i64::from(packets)),
            Movement::Dispatch(packets) => (&mut totals.dispatched, i64::from(packets)),
            Movement::Adjustment(delta) => (&mut totals.adjustment, delta),
        };
        *bucket = bucket
            .checked_add(delta)
            .ok_or(DailyMovementError::Overflow)?;
        Ok(())
    }
}

impl ConfirmationFixture {
    /// Opens the dated L12-02 register without mutating inventory or posting any movements.
    /// Rows are returned in product ID order. Dated PO logs follow the legacy register key:
    /// a stored product ID, or an exact current catalog name if absent. This can disagree
    /// with the local receiving path's stock attribution.
    ///
    /// # Errors
    /// Denies Manager before reading the fixture. Returns `MissingSaleDate` for undated
    /// active sale state and `Overflow` if any packet aggregation or balance overflows.
    pub fn open_daily_movement_register(
        &self,
        role: Role,
        day: BillDate,
    ) -> Result<Vec<DailyMovementRow<'_>>, DailyMovementError> {
        match role {
            Role::Owner => (),
            Role::Manager => return Err(DailyMovementError::AccessDenied),
        }
        let mut register = Register {
            day,
            totals: BTreeMap::new(),
        };
        for line in self.purchases.receipt_lines() {
            let Some(id) = line
                .line
                .product_id
                .or_else(|| self.catalog.product_id_by_exact_name(&line.line.item))
            else {
                continue;
            };
            for receipt in line.receipts() {
                register.record(
                    id.value(),
                    (receipt.date, Movement::Receipt(receipt.quantity.get())),
                )?;
            }
        }
        // Cancellation preserves the allocation records for undo, so use the live lifecycle.
        for (&id, (review, _, _)) in &self.current_sales {
            if self
                .lifecycle
                .get(&id)
                .is_some_and(|(cancelled, _)| *cancelled)
            {
                continue;
            }
            let date = review
                .draft
                .bill_date()
                .ok_or(DailyMovementError::MissingSaleDate)?;
            let sale = id;
            let keys = (sale, 0, 0, 0)..=(sale, usize::MAX, u64::MAX, usize::MAX);
            for (&(_, _, product, _), &(packets, _)) in self.catalog.allocations.records.range(keys)
            {
                register.record(product, (date, Movement::Dispatch(packets)))?;
            }
        }
        let mut rows = Vec::new();
        for (&id, product) in &self.catalog.products {
            for movement in &product.physical_count.movements {
                register.record(id, (movement.date, Movement::Adjustment(movement.delta)))?;
            }
            let ProductMovements { before, on } = register.totals.remove(&id).unwrap_or_default();
            let opening = i64::from(product.seeded_opening)
                .checked_add(before.received)
                .and_then(|stock| stock.checked_sub(before.dispatched))
                .and_then(|stock| stock.checked_add(before.adjustment))
                .ok_or(DailyMovementError::Overflow)?;
            let received = on
                .received
                .checked_add(on.adjustment.max(0))
                .ok_or(DailyMovementError::Overflow)?;
            let dispatched = on
                .dispatched
                .checked_sub(on.adjustment.min(0))
                .ok_or(DailyMovementError::Overflow)?;
            if opening == 0 && received == 0 && dispatched == 0 {
                continue;
            }
            let closing = opening
                .checked_add(received)
                .and_then(|stock| stock.checked_sub(dispatched))
                .ok_or(DailyMovementError::Overflow)?;
            rows.push(DailyMovementRow {
                product: ProductId::new(id),
                item: product.item_name(),
                opening,
                received: on.received,
                dispatched: on.dispatched,
                adjustment: on.adjustment,
                display_received: NonZeroI64::new(received),
                display_dispatched: NonZeroI64::new(dispatched),
                closing,
            });
        }
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Money, confirmation::StockCountMovement};

    #[test]
    fn l12_02_rejects_an_unrepresentable_display_dispatch() {
        let id = ProductId::new(1);
        let day = BillDate::new(2025, 2, 10).unwrap();
        let mut fixture = ConfirmationFixture::new(id, 0, Money::from_minor_units(1)).unwrap();
        let product = fixture.catalog.products.get_mut(&id.value()).unwrap();
        product.physical_count.movements.push(StockCountMovement {
            date: day,
            from: 0,
            to: 0,
            delta: i64::MIN,
            reason: None,
            note: None,
        });
        let before = fixture.clone();

        assert_eq!(
            fixture.open_daily_movement_register(Role::Owner, day),
            Err(DailyMovementError::Overflow)
        );
        assert_eq!(fixture, before);
    }
}
