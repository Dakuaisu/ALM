//! Typed synthetic snapshots only; none of these APIs posts or changes a PO.
mod capture;
pub(crate) mod money;

use std::collections::HashSet;
use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};

use super::{AccountScope, Error, HistoricalGst, ReadModel, Records, date, storage::owner};
use crate::confirmation::purchase_orders::output::{
    SupplierDocument, SupplierRow, render_supplier_print,
};
use crate::confirmation::{
    PopupDocument, PurchaseBusiness, PurchaseOrderStatus, PurchaseOrderView, PurchaseProgressLine,
};
use crate::{BillDate, Money, ProductId, Role};

pub use capture::PurchaseCapture;

pub struct ScopedArchiveInput {
    pub records: Records,
    pub scope: AccountScope,
    pub gst: Vec<HistoricalGst>,
    pub purchase_orders: Vec<ArchivedPurchaseOrder>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ArchivedPurchaseBusiness {
    pub name: String,
    pub address: String,
    pub mobile: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PurchaseDisplay {
    pub pack: Option<String>,
    #[serde(with = "money::optional")]
    pub mrp: Option<Money>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ArchivedPurchaseStatus {
    Pending,
    PartiallyReceived,
    Completed,
    Cancelled,
}

impl From<PurchaseOrderStatus> for ArchivedPurchaseStatus {
    fn from(status: PurchaseOrderStatus) -> Self {
        match status {
            PurchaseOrderStatus::Pending => Self::Pending,
            PurchaseOrderStatus::PartiallyReceived => Self::PartiallyReceived,
            PurchaseOrderStatus::Completed => Self::Completed,
            PurchaseOrderStatus::Cancelled => Self::Cancelled,
        }
    }
}
impl From<ArchivedPurchaseStatus> for PurchaseOrderStatus {
    fn from(status: ArchivedPurchaseStatus) -> Self {
        match status {
            ArchivedPurchaseStatus::Pending => Self::Pending,
            ArchivedPurchaseStatus::PartiallyReceived => Self::PartiallyReceived,
            ArchivedPurchaseStatus::Completed => Self::Completed,
            ArchivedPurchaseStatus::Cancelled => Self::Cancelled,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ArchivedPurchaseReceipt {
    #[serde(with = "date")]
    pub date: BillDate,
    pub quantity: NonZeroU32,
    #[serde(with = "money")]
    pub rate: Money,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ArchivedPurchaseLine {
    pub item: String,
    pub product_id: Option<ProductId>,
    pub ordered: NonZeroU32,
    #[serde(with = "money")]
    pub rate: Money,
    pub received: u32,
    pub cancelled: u32,
    pub remaining: u32,
    pub receipts: Vec<ArchivedPurchaseReceipt>,
    pub internal: PurchaseDisplay,
    pub supplier: PurchaseDisplay,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ArchivedPurchaseOrder {
    pub number: u64,
    pub supplier_name: String,
    #[serde(with = "date")]
    pub date: BillDate,
    pub note: String,
    pub business: ArchivedPurchaseBusiness,
    pub status: ArchivedPurchaseStatus,
    pub lines: Vec<ArchivedPurchaseLine>,
}

pub fn validate_orders(orders: Option<&[ArchivedPurchaseOrder]>) -> Result<(), Error> {
    let mut numbers = HashSet::new();
    for order in orders.into_iter().flatten() {
        if !numbers.insert(order.number) || order.lines.is_empty() {
            return Err(Error::InvalidArchive);
        }
        for line in &order.lines {
            let received = line.receipts.iter().try_fold(0_u32, |sum, receipt| {
                sum.checked_add(receipt.quantity.get())
            });
            let remaining = line
                .ordered
                .get()
                .checked_sub(line.received)
                .and_then(|n| n.checked_sub(line.cancelled));
            if received != Some(line.received) || remaining != Some(line.remaining) {
                return Err(Error::InvalidArchive);
            }
        }
        let all_complete = order.lines.iter().all(|line| line.remaining == 0);
        let progressed = order
            .lines
            .iter()
            .any(|line| line.received > 0 || line.cancelled > 0);
        let valid_status = match order.status {
            ArchivedPurchaseStatus::Pending => !all_complete && !progressed,
            ArchivedPurchaseStatus::PartiallyReceived => !all_complete && progressed,
            ArchivedPurchaseStatus::Completed => all_complete,
            ArchivedPurchaseStatus::Cancelled => true,
        };
        if !valid_status {
            return Err(Error::InvalidArchive);
        }
    }
    Ok(())
}

impl ReadModel {
    pub fn purchase_orders(&self, role: Role) -> Result<&[ArchivedPurchaseOrder], Error> {
        owner(role)?;
        Ok(self.purchase_orders.as_deref().unwrap_or_default())
    }

    pub fn purchase_order(
        &self,
        role: Role,
        number: u64,
    ) -> Result<Option<&ArchivedPurchaseOrder>, Error> {
        Ok(self
            .purchase_orders(role)?
            .iter()
            .find(|order| order.number == number))
    }

    pub fn view_purchase_order(
        &self,
        role: Role,
        number: u64,
    ) -> Result<Option<PurchaseOrderView>, Error> {
        Ok(self
            .purchase_order(role, number)?
            .map(|order| PurchaseOrderView {
                number: format!("PO-{:03}", order.number),
                supplier: order.supplier_name.clone(),
                date: order.date,
                status: order.status.into(),
                lines: order
                    .lines
                    .iter()
                    .map(|line| PurchaseProgressLine {
                        item: line.item.clone(),
                        pack: line.internal.pack.clone(),
                        mrp: line.internal.mrp,
                        ordered: line.ordered.get(),
                        received: line.received,
                        cancelled: line.cancelled,
                        remaining: line.remaining,
                    })
                    .collect(),
            }))
    }

    pub fn print_purchase_order(
        &self,
        role: Role,
        number: u64,
    ) -> Result<Option<PopupDocument>, Error> {
        Ok(self.purchase_order(role, number)?.map(|order| {
            render_supplier_print(&SupplierDocument {
                number: order.number,
                supplier_name: &order.supplier_name,
                date: order.date,
                note: &order.note,
                business: PurchaseBusiness {
                    name: &order.business.name,
                    address: &order.business.address,
                    mobile: &order.business.mobile,
                },
                rows: order
                    .lines
                    .iter()
                    .map(|line| SupplierRow {
                        item: &line.item,
                        pack: line.supplier.pack.clone(),
                        mrp: line.supplier.mrp,
                        ordered: line.ordered,
                    })
                    .collect(),
            })
        }))
    }
}
