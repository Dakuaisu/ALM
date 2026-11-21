use super::{
    ArchivedPurchaseBusiness, ArchivedPurchaseLine, ArchivedPurchaseOrder, ArchivedPurchaseReceipt,
    PurchaseDisplay, validate_orders,
};
use crate::Role;
use crate::confirmation::{
    PurchaseBusiness, PurchaseOrder, PurchaseOrderView, PurchaseProductDisplay,
};
use crate::synthetic_read_model::{Error, storage::owner};

pub struct PurchaseCapture<'a> {
    pub order: &'a PurchaseOrder,
    pub number: u64,
    pub view: &'a PurchaseOrderView,
    pub business: PurchaseBusiness<'a>,
    pub products: &'a [PurchaseProductDisplay<'a>],
}

fn display(product: Option<&PurchaseProductDisplay<'_>>) -> PurchaseDisplay {
    let pack = product.and_then(|product| {
        let joined = format!("{} {}", product.pack_size, product.uom);
        let trimmed = joined.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_owned())
    });
    PurchaseDisplay {
        pack,
        mrp: product.and_then(|product| product.mrp.filter(|mrp| mrp.minor_units() != 0)),
    }
}

impl ArchivedPurchaseOrder {
    pub fn capture(role: Role, input: PurchaseCapture<'_>) -> Result<Self, Error> {
        owner(role)?;
        let order = input.order;
        let view = input.view;
        if view.number != format!("PO-{:03}", input.number)
            || view.supplier != order.supplier_name
            || view.date != order.date
            || view.lines.len() != order.items.len()
        {
            return Err(Error::InvalidArchive);
        }
        let mut lines = Vec::with_capacity(order.items.len());
        for (saved, progress) in order.items.iter().zip(&view.lines) {
            let line = &saved.line;
            let internal = input
                .products
                .iter()
                .find(|product| product.item == line.item);
            let supplier = line
                .product_id
                .and_then(|id| input.products.iter().find(|product| product.id == id))
                .or_else(|| {
                    input
                        .products
                        .iter()
                        .find(|product| product.item == line.item)
                });
            let internal = display(internal);
            if progress.item != line.item
                || progress.ordered != line.ordered.get()
                || progress.received != saved.received
                || progress.remaining != saved.remaining()
                || progress.pack != internal.pack
                || progress.mrp != internal.mrp
            {
                return Err(Error::InvalidArchive);
            }
            lines.push(ArchivedPurchaseLine {
                item: line.item.clone(),
                product_id: line.product_id,
                ordered: line.ordered,
                rate: line.rate,
                received: saved.received,
                cancelled: progress.cancelled,
                remaining: progress.remaining,
                receipts: saved
                    .receipts()
                    .iter()
                    .map(|receipt| ArchivedPurchaseReceipt {
                        date: receipt.date,
                        quantity: receipt.quantity,
                        rate: receipt.rate,
                    })
                    .collect(),
                internal,
                supplier: display(supplier),
            });
        }
        let result = Self {
            number: input.number,
            supplier_name: order.supplier_name.clone(),
            date: order.date,
            note: order.note.clone(),
            status: view.status.into(),
            lines,
            business: ArchivedPurchaseBusiness {
                name: input.business.name.to_owned(),
                address: input.business.address.to_owned(),
                mobile: input.business.mobile.to_owned(),
            },
        };
        validate_orders(Some(std::slice::from_ref(&result)))?;
        Ok(result)
    }
}
