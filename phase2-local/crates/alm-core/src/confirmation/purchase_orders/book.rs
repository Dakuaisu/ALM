//! Purchasing as Owner book rows, and back.
use std::num::NonZeroU32;

use super::receiving::PurchaseCost;
use super::supplier_prices::SupplierQuotes;
use super::{
    PurchaseBook, PurchaseLine, PurchaseOrder, PurchaseOrderId, PurchaseProduct, PurchaseReceipt,
    PurchaseSupplierId, SavedPurchaseLine, SavedPurchaseSupplier,
};
use crate::ProductId;
use crate::confirmation::ConfirmationError;
use crate::projection::book::{BookOrder, BookOrderLine, BookOrderReceipt, BookPurchasing};

fn index(value: usize) -> Result<u64, ConfirmationError> {
    u64::try_from(value).map_err(|_| ConfirmationError::Overflow)
}

fn position(value: u64) -> Result<usize, ConfirmationError> {
    usize::try_from(value).map_err(|_| ConfirmationError::Overflow)
}

fn positive(value: u32) -> Result<NonZeroU32, ConfirmationError> {
    NonZeroU32::new(value).ok_or(ConfirmationError::InvalidDraft)
}

impl PurchaseBook {
    pub(in crate::confirmation) fn book(&self) -> Result<BookPurchasing, ConfirmationError> {
        Ok(BookPurchasing {
            products: self
                .products
                .iter()
                .map(|p| (p.id.value(), p.item.clone(), p.buy_rate))
                .collect(),
            suppliers: self
                .suppliers
                .iter()
                .map(|s| Ok((index(s.id.0)?, s.name.clone())))
                .collect::<Result<_, ConfirmationError>>()?,
            price_suppliers: self
                .price_suppliers
                .iter()
                .map(|s| index(s.0))
                .collect::<Result<_, ConfirmationError>>()?,
            costs: self
                .costs
                .iter()
                .map(|c| (c.product.value(), c.average))
                .collect(),
            quotes: self
                .quotes
                .iter()
                .map(|(product, quotes)| (*product, quotes.0.clone()))
                .collect(),
            orders: self
                .orders
                .iter()
                .map(|o| {
                    Ok(BookOrder {
                        id: index(o.id.0)?,
                        supplier: index(o.supplier_id.0)?,
                        supplier_name: o.supplier_name.clone(),
                        date: o.date,
                        note: o.note.clone(),
                        cancelled: o.cancelled,
                        lines: o
                            .items
                            .iter()
                            .map(|l| BookOrderLine {
                                item: l.line.item.clone(),
                                product: l.line.product_id.map(ProductId::value),
                                ordered: l.line.ordered.get(),
                                received: l.received,
                                closed: l.cancelled,
                                rate: l.line.rate,
                                receipts: l
                                    .receipts
                                    .iter()
                                    .map(|r| BookOrderReceipt {
                                        date: r.date,
                                        quantity: r.quantity.get(),
                                        rate: r.rate,
                                    })
                                    .collect(),
                            })
                            .collect(),
                    })
                })
                .collect::<Result<_, ConfirmationError>>()?,
        })
    }

    pub(in crate::confirmation) fn from_book(
        book: &BookPurchasing,
    ) -> Result<Self, ConfirmationError> {
        Ok(Self {
            products: book
                .products
                .iter()
                .map(|(id, item, buy)| PurchaseProduct {
                    id: ProductId::new(*id),
                    item: item.clone(),
                    buy_rate: *buy,
                })
                .collect(),
            suppliers: book
                .suppliers
                .iter()
                .map(|(id, name)| {
                    Ok(SavedPurchaseSupplier {
                        id: PurchaseSupplierId(position(*id)?),
                        name: name.clone(),
                    })
                })
                .collect::<Result<_, ConfirmationError>>()?,
            orders: book
                .orders
                .iter()
                .map(|o| {
                    Ok(PurchaseOrder {
                        id: PurchaseOrderId(position(o.id)?),
                        supplier_id: PurchaseSupplierId(position(o.supplier)?),
                        supplier_name: o.supplier_name.clone(),
                        date: o.date,
                        note: o.note.clone(),
                        cancelled: o.cancelled,
                        items: o
                            .lines
                            .iter()
                            .map(|l| {
                                Ok(SavedPurchaseLine {
                                    line: PurchaseLine {
                                        item: l.item.clone(),
                                        product_id: l.product.map(ProductId::new),
                                        ordered: positive(l.ordered)?,
                                        rate: l.rate,
                                    },
                                    received: l.received,
                                    cancelled: l.closed,
                                    receipts: l
                                        .receipts
                                        .iter()
                                        .map(|r| {
                                            Ok(PurchaseReceipt {
                                                date: r.date,
                                                quantity: positive(r.quantity)?,
                                                rate: r.rate,
                                            })
                                        })
                                        .collect::<Result<_, ConfirmationError>>()?,
                                })
                            })
                            .collect::<Result<_, ConfirmationError>>()?,
                    })
                })
                .collect::<Result<_, ConfirmationError>>()?,
            costs: book
                .costs
                .iter()
                .map(|(product, average)| PurchaseCost {
                    product: ProductId::new(*product),
                    average: *average,
                })
                .collect(),
            quotes: book
                .quotes
                .iter()
                .map(|(product, quotes)| (*product, SupplierQuotes(quotes.clone())))
                .collect(),
            price_suppliers: book
                .price_suppliers
                .iter()
                .map(|s| Ok(PurchaseSupplierId(position(*s)?)))
                .collect::<Result<_, ConfirmationError>>()?,
        })
    }
}
