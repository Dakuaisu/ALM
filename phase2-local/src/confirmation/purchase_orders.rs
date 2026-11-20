mod note_edit;
mod output;
mod paste;
mod receive_form;
mod receiving;
#[cfg(test)]
mod tests;

use std::{fmt, num::NonZeroU32};

use crate::{BillDate, Money, ProductId, Role};

use super::{ConfirmationError, ConfirmationFixture};

pub use note_edit::PurchaseOrderNoteEdit;
pub use output::{
    PurchaseBusiness, PurchaseOrderHandoff, PurchaseOrderSelection, PurchaseOrderStatus,
    PurchaseOrderView, PurchaseOutputRequest, PurchaseProductDisplay, PurchaseProgressLine,
    PurchaseSupplierContact, PurchaseWhatsAppRequest,
};
pub use paste::PurchasePasteWarnings;
pub use receive_form::{ReceiveForm, ReceiveFormError, ReceiveFormRow};
pub use receiving::{ReceiveLine, ReceiveRequest, ReceiveResult};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PurchaseSupplierId(usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PurchaseOrderId(usize);

/// Fixture metadata for a product already present in inventory.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PurchaseProduct {
    pub id: ProductId,
    pub item: String,
    pub buy_rate: Money,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PurchaseSupplier {
    pub id: Option<PurchaseSupplierId>,
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PurchaseLine {
    pub item: String,
    pub product_id: Option<ProductId>,
    pub ordered: NonZeroU32,
    pub rate: Money,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PurchaseDraft {
    pub supplier: Option<PurchaseSupplier>,
    pub date: Option<BillDate>,
    pub note: String,
    pub items: Vec<PurchaseLine>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SavedPurchaseSupplier {
    pub id: PurchaseSupplierId,
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SavedPurchaseLine {
    pub line: PurchaseLine,
    pub received: u32,
    cancelled: u32,
    receipts: Vec<PurchaseReceipt>,
}

impl SavedPurchaseLine {
    pub const fn receipts(&self) -> &[PurchaseReceipt] {
        self.receipts.as_slice()
    }

    pub(crate) const fn remaining(&self) -> u32 {
        self.line
            .ordered
            .get()
            .saturating_sub(self.received)
            .saturating_sub(self.cancelled)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PurchaseReceipt {
    pub date: BillDate,
    pub quantity: NonZeroU32,
    pub rate: Money,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PurchaseOrder {
    pub id: PurchaseOrderId,
    pub supplier_id: PurchaseSupplierId,
    pub supplier_name: String,
    pub date: BillDate,
    pub note: String,
    pub items: Vec<SavedPurchaseLine>,
    cancelled: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PurchaseError {
    AccessDenied,
    OrderNotFound,
    OrderNotPending,
    StaleOrderNote,
    NoUsableRows,
    MissingSupplier,
    MissingItems,
    MissingDate,
    InvalidReceipt,
    ReceiptOverflow,
}

impl fmt::Display for PurchaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::AccessDenied => "Only Owner can access purchase orders",
            Self::OrderNotFound => "Purchase order not found",
            Self::OrderNotPending => "Only pending purchase orders can be edited",
            Self::StaleOrderNote => "Purchase order changed; reopen it to edit the note",
            Self::NoUsableRows => "Nothing usable found — select the rows in the sheet, copy, and paste them here unchanged.",
            Self::MissingSupplier => "Enter supplier",
            Self::MissingItems => "Add at least one item with an order quantity",
            Self::MissingDate => "Pick the order date",
            Self::InvalidReceipt => "Invalid receipt",
            Self::ReceiptOverflow => "Receipt amount or quantity overflow",
        })
    }
}

impl std::error::Error for PurchaseError {}

#[derive(Clone, Default)]
#[cfg_attr(test, derive(Debug, Eq, PartialEq))]
pub(super) struct PurchaseBook {
    products: Vec<PurchaseProduct>,
    suppliers: Vec<SavedPurchaseSupplier>,
    orders: Vec<PurchaseOrder>,
    costs: Vec<receiving::PurchaseCost>,
}

const fn owner(role: Role) -> Result<(), PurchaseError> {
    match role {
        Role::Owner => Ok(()),
        Role::Manager => Err(PurchaseError::AccessDenied),
    }
}

impl ConfirmationFixture {
    /// Seeds item and buy-rate metadata from the trusted fixture without changing inventory.
    pub fn register_purchase_product(
        &mut self,
        product: PurchaseProduct,
    ) -> Result<(), ConfirmationError> {
        if product.buy_rate.minor_units() < 0 {
            return Err(ConfirmationError::InvalidAmount);
        }
        if product.item.trim().is_empty()
            || self.catalog.stock(product.id).is_none()
            || self.purchases.products.iter().any(|p| p.id == product.id)
        {
            return Err(ConfirmationError::InvalidDraft);
        }
        self.purchases.products.push(product);
        Ok(())
    }

    /// Fills the owner's editable draft and returns warnings without saving.
    pub fn parse_purchase_paste(
        &self,
        role: Role,
        draft: &mut PurchaseDraft,
        text: &str,
    ) -> Result<PurchasePasteWarnings, PurchaseError> {
        owner(role)?;
        paste::fill(&self.purchases, draft, text)
    }

    /// Saves one new local order; no receiving or stock mutations are performed.
    pub fn save_purchase_order(
        &mut self,
        role: Role,
        draft: &PurchaseDraft,
    ) -> Result<PurchaseOrderId, PurchaseError> {
        owner(role)?;
        self.purchases.save(draft)
    }

    pub fn purchase_suppliers(
        &self,
        role: Role,
    ) -> Result<&[SavedPurchaseSupplier], PurchaseError> {
        owner(role)?;
        Ok(&self.purchases.suppliers)
    }

    pub fn purchase_orders(&self, role: Role) -> Result<&[PurchaseOrder], PurchaseError> {
        owner(role)?;
        Ok(&self.purchases.orders)
    }
}

impl PurchaseBook {
    fn save(&mut self, draft: &PurchaseDraft) -> Result<PurchaseOrderId, PurchaseError> {
        let name = draft
            .supplier
            .as_ref()
            .map(|supplier| supplier.name.trim())
            .filter(|name| !name.is_empty())
            .ok_or(PurchaseError::MissingSupplier)?;
        let mut lines = draft
            .items
            .iter()
            .filter(|line| !line.item.trim().is_empty())
            .peekable();
        if lines.peek().is_none() {
            return Err(PurchaseError::MissingItems);
        }
        let date = draft.date.ok_or(PurchaseError::MissingDate)?;
        let items = lines
            .cloned()
            .map(|mut line| {
                if line.product_id.is_none() {
                    line.product_id = self
                        .products
                        .iter()
                        .find(|p| p.item == line.item)
                        .map(|p| p.id);
                }
                SavedPurchaseLine {
                    line,
                    received: 0,
                    cancelled: 0,
                    receipts: Vec::new(),
                }
            })
            .collect();
        let existing = self.suppliers.iter().find(|supplier| supplier.name == name);
        let supplier_id = existing.map_or(PurchaseSupplierId(self.suppliers.len()), |s| s.id);
        let id = PurchaseOrderId(self.orders.len());
        let order = PurchaseOrder {
            id,
            supplier_id,
            supplier_name: name.to_owned(),
            date,
            note: draft.note.clone(),
            items,
            cancelled: false,
        };

        if existing.is_none() {
            self.suppliers.push(SavedPurchaseSupplier {
                id: supplier_id,
                name: name.to_owned(),
            });
        }
        self.orders.push(order);
        Ok(id)
    }
}
