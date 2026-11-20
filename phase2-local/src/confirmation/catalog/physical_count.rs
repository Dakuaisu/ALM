//! L11-02, traced to `ALM_Business_Manager.html` `adjustStock` (8184–8264) and `stockOf`
//! (1129–1148). Legacy counts accept finite signed/fractional values, round the delta to
//! hundredths, and use a modal snapshot of stock. This fixture accepts whole packets in
//! `0..=u32::MAX` and uses submission-time stock. Fridge is display-only in both paths.
//! The local FIFO layer for an increase is not a receipt or transfer and makes no claim
//! about legacy valuation.

use std::cmp::Ordering;

use super::{Lot, Product};
use crate::confirmation::ConfirmationFixture;
use crate::{BillDate, ProductId, Role};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StockCountReason {
    CountCorrection,
    DamageOrLeakage,
    Expired,
    TheftOrUnknown,
    OwnUseOrSample,
    Other,
}

/// Owner form submission. `counted` and any supplied `fridge` must be unsigned decimal
/// packet counts after trimming. Omit `fridge` to leave the display count unchanged.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PhysicalCountInput<'a> {
    pub product: ProductId,
    pub counted: &'a str,
    pub fridge: Option<&'a str>,
    pub date: BillDate,
    pub reason: Option<StockCountReason>,
    pub note: Option<&'a str>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StockCountMovement {
    pub date: BillDate,
    pub from: u32,
    pub to: u32,
    pub delta: i64,
    pub reason: Option<StockCountReason>,
    pub note: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PhysicalCount<'a> {
    pub on_hand: u32,
    pub fridge: u32,
    pub movements: &'a [StockCountMovement],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PhysicalCountError {
    AccessDenied,
    ProductNotFound,
    InvalidCount,
    InvalidFridgeCount,
    Overflow,
}

#[derive(Clone, Default)]
#[cfg_attr(test, derive(Debug, Eq, PartialEq))]
pub(super) struct PhysicalCountState {
    fridge: u32,
    movements: Vec<StockCountMovement>,
}

const fn require_owner(role: Role) -> Result<(), PhysicalCountError> {
    match role {
        Role::Owner => Ok(()),
        Role::Manager => Err(PhysicalCountError::AccessDenied),
    }
}

impl ConfirmationFixture {
    /// Reads the current on-hand and fridge counts and the product's correction history.
    ///
    /// # Errors
    /// Denies Manager before lookup; returns `ProductNotFound` for an unknown product.
    pub fn open_physical_count(
        &self,
        role: Role,
        id: ProductId,
    ) -> Result<PhysicalCount<'_>, PhysicalCountError> {
        require_owner(role)?;
        let product = self
            .catalog
            .products
            .get(&id.value())
            .ok_or(PhysicalCountError::ProductNotFound)?;
        Ok(PhysicalCount {
            on_hand: product.stock,
            fridge: product.physical_count.fridge,
            movements: &product.physical_count.movements,
        })
    }

    /// Saves a physical count against current stock. An equal count can update the fridge
    /// display independently and does not record a movement or invalidate cancel/undo.
    ///
    /// # Errors
    /// Denies Manager before validation and lookup. Rejects invalid quantities, unknown
    /// products, or movement epoch overflow without applying any changes.
    pub fn submit_physical_count(
        &mut self,
        role: Role,
        input: PhysicalCountInput<'_>,
    ) -> Result<(), PhysicalCountError> {
        require_owner(role)?;
        let target = input
            .counted
            .trim()
            .parse::<u32>()
            .map_err(|_| PhysicalCountError::InvalidCount)?;
        let fridge = input
            .fridge
            .map(|value| value.trim().parse::<u32>())
            .transpose()
            .map_err(|_| PhysicalCountError::InvalidFridgeCount)?;
        let product = self
            .catalog
            .products
            .get_mut(&input.product.value())
            .ok_or(PhysicalCountError::ProductNotFound)?;
        let from = product.stock;
        if target != from {
            let epoch = self
                .movement_epoch
                .checked_add(1)
                .ok_or(PhysicalCountError::Overflow)?;
            let movement = StockCountMovement {
                date: input.date,
                from,
                to: target,
                delta: i64::from(target) - i64::from(from),
                reason: input.reason,
                note: input
                    .note
                    .map(str::trim)
                    .filter(|note| !note.is_empty())
                    .map(str::to_owned),
            };
            product.correct_on_hand(target, input.date);
            product.physical_count.movements.push(movement);
            self.movement_epoch = epoch;
        }
        if let Some(fridge) = fridge {
            product.physical_count.fridge = fridge;
        }
        Ok(())
    }
}

impl Product {
    fn correct_on_hand(&mut self, target: u32, date: BillDate) {
        match target.cmp(&self.stock) {
            Ordering::Greater => self.lots.push(Lot {
                remaining: target - self.stock,
                unit_cost: None,
                date: Some(date),
            }),
            Ordering::Less => {
                let mut remaining = self.stock - target;
                // Keep exhausted slots in place: past sales refer to lot indices.
                for index in self.fifo_lot_indices() {
                    let lot = &mut self.lots[index];
                    let taken = remaining.min(lot.remaining);
                    lot.remaining -= taken;
                    remaining -= taken;
                    if remaining == 0 {
                        break;
                    }
                }
            }
            Ordering::Equal => return,
        }
        self.stock = target;
    }
}
