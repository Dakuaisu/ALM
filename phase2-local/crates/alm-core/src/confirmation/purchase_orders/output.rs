pub(super) mod supplier_html;
mod whatsapp;

use crate::confirmation::PopupDocument;
use crate::{BillDate, Money, ProductId, Role};

use super::{ConfirmationFixture, PurchaseError, PurchaseOrder, PurchaseOrderId, owner};
pub(crate) use supplier_html::{SupplierDocument, SupplierRow, render_supplier_print};
pub use whatsapp::{PurchaseOrderHandoff, PurchaseSupplierContact, PurchaseWhatsAppRequest};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PurchaseOrderSelection {
    pub role: Role,
    pub id: PurchaseOrderId,
    pub number: u64,
}

impl PurchaseOrderSelection {
    fn number(self) -> String {
        supplier_html::purchase_order_number(self.number)
    }
}

/// Explicit display metadata from the synthetic fixture; it contains no purchase cost.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PurchaseProductDisplay<'a> {
    pub id: ProductId,
    pub item: &'a str,
    pub pack_size: &'a str,
    pub uom: &'a str,
    pub mrp: Option<Money>,
}

impl PurchaseProductDisplay<'_> {
    fn pack(self) -> Option<String> {
        let joined = format!("{} {}", self.pack_size, self.uom);
        let pack = joined.trim();
        (!pack.is_empty()).then(|| pack.to_owned())
    }

    fn displayed_mrp(self) -> Option<Money> {
        self.mrp.filter(|amount| amount.minor_units() != 0)
    }
}

/// Synthetic business details for supplier output, not persisted by the fixture.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PurchaseBusiness<'a> {
    pub name: &'a str,
    pub address: &'a str,
    pub mobile: &'a str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PurchaseOutputRequest<'a> {
    pub selection: PurchaseOrderSelection,
    pub business: PurchaseBusiness<'a>,
    pub products: &'a [PurchaseProductDisplay<'a>],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PurchaseOrderStatus {
    Pending,
    PartiallyReceived,
    Completed,
    Cancelled,
}

impl PurchaseOrderStatus {
    /// Whether the owner can open Edit for an order with this status.
    pub const fn can_edit(self) -> bool {
        match self {
            Self::Pending => true,
            Self::PartiallyReceived | Self::Completed | Self::Cancelled => false,
        }
    }

    /// Whether the legacy View would offer Receive Goods. Does not receive anything.
    pub const fn can_receive(self) -> bool {
        match self {
            Self::Pending | Self::PartiallyReceived => true,
            Self::Completed | Self::Cancelled => false,
        }
    }
}

impl PurchaseOrder {
    pub fn status(&self) -> PurchaseOrderStatus {
        if self.cancelled {
            PurchaseOrderStatus::Cancelled
        } else if self.items.iter().all(|line| line.remaining() == 0) {
            PurchaseOrderStatus::Completed
        } else if self
            .items
            .iter()
            .any(|line| line.received > 0 || line.cancelled > 0)
        {
            PurchaseOrderStatus::PartiallyReceived
        } else {
            PurchaseOrderStatus::Pending
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PurchaseOrderView {
    pub number: String,
    pub supplier: String,
    pub date: BillDate,
    pub status: PurchaseOrderStatus,
    pub lines: Vec<PurchaseProgressLine>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PurchaseProgressLine {
    pub item: String,
    pub pack: Option<String>,
    pub mrp: Option<Money>,
    pub ordered: u32,
    pub received: u32,
    pub cancelled: u32,
    pub remaining: u32,
}

impl ConfirmationFixture {
    /// Projects the owner's internal progress view without changing the order.
    ///
    /// # Errors
    /// Denies Managers before looking up the selected order.
    pub fn view_purchase_order(
        &self,
        selection: PurchaseOrderSelection,
        products: &[PurchaseProductDisplay<'_>],
    ) -> Result<PurchaseOrderView, PurchaseError> {
        let order = self.selected_purchase_order(selection)?;
        let lines: Vec<_> = order
            .items
            .iter()
            .map(|saved| {
                let item = &saved.line.item;
                let product = products
                    .iter()
                    .find(|product| product.item == item)
                    .copied();
                let ordered = saved.line.ordered.get();
                PurchaseProgressLine {
                    item: item.clone(),
                    pack: product.and_then(PurchaseProductDisplay::pack),
                    mrp: product.and_then(PurchaseProductDisplay::displayed_mrp),
                    ordered,
                    received: saved.received,
                    cancelled: saved.cancelled,
                    remaining: saved.remaining(),
                }
            })
            .collect();
        Ok(PurchaseOrderView {
            number: selection.number(),
            supplier: order.supplier_name.clone(),
            date: order.date,
            status: order.status(),
            lines,
        })
    }

    /// Captures the legacy Save PDF / Print document and intent without opening a popup.
    ///
    /// # Errors
    /// Denies Managers and rejects a missing order before invoking the sink.
    pub fn print_purchase_order(
        &self,
        request: PurchaseOutputRequest<'_>,
        sink: &mut impl FnMut(PopupDocument),
    ) -> Result<(), PurchaseError> {
        let order = self.selected_purchase_order(request.selection)?;
        sink(render_supplier_print(&supplier_document(order, request)));
        Ok(())
    }

    fn selected_purchase_order(
        &self,
        selection: PurchaseOrderSelection,
    ) -> Result<&PurchaseOrder, PurchaseError> {
        owner(selection.role)?;
        self.purchases
            .orders
            .get(selection.id.0)
            .ok_or(PurchaseError::OrderNotFound)
    }
}

fn supplier_document<'a>(
    order: &'a PurchaseOrder,
    request: PurchaseOutputRequest<'a>,
) -> SupplierDocument<'a> {
    let rows = order
        .items
        .iter()
        .map(|saved| {
            let line = &saved.line;
            let products = request.products;
            let product = line
                .product_id
                .and_then(|id| products.iter().find(|product| product.id == id))
                .or_else(|| products.iter().find(|product| product.item == line.item))
                .copied();
            SupplierRow {
                item: &line.item,
                pack: product.and_then(PurchaseProductDisplay::pack),
                mrp: product.and_then(PurchaseProductDisplay::displayed_mrp),
                ordered: line.ordered,
            }
        })
        .collect();
    SupplierDocument {
        number: request.selection.number,
        supplier_name: &order.supplier_name,
        date: order.date,
        note: &order.note,
        business: request.business,
        rows,
    }
}
