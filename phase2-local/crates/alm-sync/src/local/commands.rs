//! The simulation's catalog, purchasing, stock, opening, godown and GST commands: each runs
//! the same core rule the screens used to call directly, on the state copy `commit` rolls back.
use alm_core::Role;
use alm_core::confirmation::{GstBill, PurchaseOrderId, PurchaseSupplierId};

use super::{State, rejected};
use crate::{AuthorityError, Intent, Outcome};

mod books;
mod purchasing;

const ROLE: Role = Role::Owner;

fn said(text: impl Into<String>) -> AuthorityError {
    AuthorityError::Rejected(text.into())
}

fn order_id(state: &State, index: u64) -> Result<PurchaseOrderId, AuthorityError> {
    state
        .fixture
        .purchase_orders(ROLE)
        .map_err(rejected)?
        .iter()
        .map(|order| order.id)
        .find(|id| u64::try_from(id.index()).is_ok_and(|i| i == index))
        .ok_or_else(|| said("Purchase order not found"))
}

fn supplier_id(state: &State, index: u64) -> Result<PurchaseSupplierId, AuthorityError> {
    state
        .fixture
        .purchase_suppliers(ROLE)
        .map_err(rejected)?
        .iter()
        .map(|supplier| supplier.id)
        .find(|id| u64::try_from(id.index()).is_ok_and(|i| i == index))
        .ok_or_else(|| said("Unknown supplier"))
}

/// Rejects as stale when a PO's remaining quantities differ from the reviewed ones.
fn check_remaining(
    state: &State,
    id: PurchaseOrderId,
    remaining: &[u32],
) -> Result<(), AuthorityError> {
    let order = state
        .fixture
        .purchase_orders(ROLE)
        .map_err(rejected)?
        .iter()
        .find(|order| order.id == id)
        .ok_or_else(|| said("Purchase order not found"))?;
    let current: Vec<u32> = order
        .items
        .iter()
        .map(alm_core::confirmation::SavedPurchaseLine::remaining)
        .collect();
    if current == remaining {
        Ok(())
    } else {
        Err(AuthorityError::StaleReview)
    }
}

fn index(value: usize) -> Result<u64, AuthorityError> {
    u64::try_from(value).map_err(|_| AuthorityError::Invalid)
}

fn bill(text: &str) -> Result<GstBill, AuthorityError> {
    GstBill::from_record_text(text).map_err(|_| AuthorityError::Invalid)
}

fn gst_error(error: alm_core::confirmation::GstBillError) -> AuthorityError {
    error.warning().map_or_else(|| rejected(error), said)
}

pub(super) fn apply(state: &mut State, intent: &Intent) -> Result<Outcome, AuthorityError> {
    match intent {
        Intent::SaveProduct(_)
        | Intent::DeleteProduct { .. }
        | Intent::AddSupplier { .. }
        | Intent::SetSupplierPrices { .. }
        | Intent::SavePurchaseOrder(_)
        | Intent::ReceivePurchaseOrder { .. }
        | Intent::ClosePurchaseOrder { .. } => purchasing::apply(state, intent),
        Intent::SaveCrate(_)
        | Intent::DeleteCrate { .. }
        | Intent::SaveOpening { .. }
        | Intent::SaveGodown { .. }
        | Intent::CountStock { .. }
        | Intent::SaveGstBill { .. }
        | Intent::CancelGstBill { .. }
        | Intent::DeleteGstBill { .. }
        | Intent::ImportGstBill { .. } => books::apply(state, intent),
        Intent::PostSale(_)
        | Intent::SellCrate(_)
        | Intent::CancelSale { .. }
        | Intent::RestoreSale { .. }
        | Intent::EditSale { .. }
        | Intent::RecordReceipt { .. }
        | Intent::SaveCashDay { .. }
        | Intent::DeleteCashDay { .. } => Err(AuthorityError::Invalid),
    }
}
