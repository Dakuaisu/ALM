use super::{
    BillDate, ConfirmationFixture, Money, NonZeroU32, PurchaseError, PurchaseOrderId, Role, fmt,
};

/// Detached defaults for the Receive Goods form, in saved purchase-order row order.
#[derive(Debug, Eq, PartialEq)]
pub struct ReceiveForm {
    pub id: PurchaseOrderId,
    pub supplier_name: String,
    pub received_on: BillDate,
    pub rows: Vec<ReceiveFormRow>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct ReceiveFormRow {
    pub item: String,
    pub ordered: NonZeroU32,
    pub received: u32,
    pub remaining: u32,
    pub rate: Money,
    pub receive_now: u32,
    pub rate_editable: bool,
    pub quantity_editable: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReceiveFormError {
    AccessDenied,
    OrderNotFound,
}

impl fmt::Display for ReceiveFormError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AccessDenied => fmt::Display::fmt(&PurchaseError::AccessDenied, f),
            Self::OrderNotFound => f.write_str("PO not found"),
        }
    }
}

impl std::error::Error for ReceiveFormError {}

impl ConfirmationFixture {
    /// Opens a detached Receive Goods form without recording any receipts.
    ///
    /// # Errors
    /// Denies Managers before looking up the order and warns if the order is missing.
    pub fn open_purchase_receive_form(
        &self,
        role: Role,
        id: PurchaseOrderId,
        today: BillDate,
    ) -> Result<ReceiveForm, ReceiveFormError> {
        let order = self
            .purchase_orders(role)
            .map_err(|_| ReceiveFormError::AccessDenied)?
            .get(id.0)
            .ok_or(ReceiveFormError::OrderNotFound)?;
        let rows = order
            .items
            .iter()
            .map(|line| {
                let remaining = line.remaining();
                ReceiveFormRow {
                    item: line.line.item.clone(),
                    ordered: line.line.ordered,
                    received: line.received,
                    remaining,
                    rate: line.line.rate,
                    receive_now: remaining,
                    rate_editable: remaining > 0,
                    quantity_editable: remaining > 0,
                }
            })
            .collect();
        Ok(ReceiveForm {
            id: order.id,
            supplier_name: order.supplier_name.clone(),
            received_on: today,
            rows,
        })
    }
}
