use crate::Role;

use super::super::{ConfirmationFixture, PurchaseError, PurchaseOrderId, owner};

impl ConfirmationFixture {
    /// Closes each unreceived case only when an Owner confirms the prompt.
    ///
    /// # Errors
    /// Denies Managers before looking up the selected order.
    pub fn close_remaining(
        &mut self,
        role: Role,
        id: PurchaseOrderId,
        confirmed: bool,
    ) -> Result<(), PurchaseError> {
        owner(role)?;
        if !confirmed {
            return Ok(());
        }
        let order = self
            .purchases
            .orders
            .get_mut(id.0)
            .ok_or(PurchaseError::OrderNotFound)?;
        for line in &mut order.items {
            line.cancelled = line.cancelled.saturating_add(line.remaining());
        }
        Ok(())
    }
}
