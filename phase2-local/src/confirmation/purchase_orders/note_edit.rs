use super::{ConfirmationFixture, PurchaseError, PurchaseOrder, PurchaseOrderId, Role, owner};

/// A detached note copy tied to the purchase order as it was when opened.
#[derive(Debug)]
pub struct PurchaseOrderNoteEdit {
    pub note: String,
    original: PurchaseOrder,
}

impl ConfirmationFixture {
    /// Copies the note of an existing pending purchase order without changing it.
    ///
    /// # Errors
    /// Denies Managers, missing orders, and orders that are not pending.
    pub fn open_purchase_order_note(
        &self,
        role: Role,
        id: PurchaseOrderId,
    ) -> Result<PurchaseOrderNoteEdit, PurchaseError> {
        let order = self
            .purchase_orders(role)?
            .get(id.0)
            .ok_or(PurchaseError::OrderNotFound)?;
        ensure_pending(order)?;
        Ok(PurchaseOrderNoteEdit {
            note: order.note.clone(),
            original: order.clone(),
        })
    }

    /// Updates only the note on the same order if it is still pending and unchanged.
    ///
    /// # Errors
    /// Denies Managers, missing orders, non-pending orders, and stale edits.
    pub fn save_purchase_order_note(
        &mut self,
        role: Role,
        edit: &PurchaseOrderNoteEdit,
    ) -> Result<PurchaseOrderId, PurchaseError> {
        owner(role)?;
        let id = edit.original.id;
        let order = self
            .purchases
            .orders
            .get_mut(id.0)
            .ok_or(PurchaseError::OrderNotFound)?;
        ensure_pending(order)?;
        if *order != edit.original {
            return Err(PurchaseError::StaleOrderNote);
        }
        order.note.clone_from(&edit.note);
        Ok(id)
    }
}

fn ensure_pending(order: &PurchaseOrder) -> Result<(), PurchaseError> {
    if order.status().can_edit() {
        Ok(())
    } else {
        Err(PurchaseError::OrderNotPending)
    }
}
