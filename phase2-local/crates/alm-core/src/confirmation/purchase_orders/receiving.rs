use super::{
    BillDate, ConfirmationFixture, Money, ProductId, PurchaseError, PurchaseOrderId,
    PurchaseReceipt, Role, owner,
};
use std::num::NonZeroU32;

mod close_remaining;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReceiveLine {
    pub quantity: u32,
    pub rate: Money,
}

#[derive(Clone, Copy, Debug)]
pub struct ReceiveRequest<'a> {
    pub role: Role,
    pub id: PurchaseOrderId,
    pub date: Option<BillDate>,
    pub today: BillDate,
    pub lines: &'a [ReceiveLine],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReceiveResult {
    Received,
    NothingToReceive,
}

impl ReceiveResult {
    pub const fn warning(self) -> Option<&'static str> {
        match self {
            Self::Received => None,
            Self::NothingToReceive => Some("Enter at least one quantity to receive"),
        }
    }
}

#[derive(Clone)]
#[cfg_attr(any(test, feature = "test-support"), derive(Debug, Eq, PartialEq))]
pub(super) struct PurchaseCost {
    product: ProductId,
    average: Money,
}

impl ConfirmationFixture {
    /// Synthetic staged receiving; historical allocations are not recalculated.
    /// Zero-rate FIFO fallback is captured on receipt, not repriced after a later line-rate change.
    pub fn receive_purchase_order(
        &mut self,
        request: ReceiveRequest<'_>,
    ) -> Result<ReceiveResult, PurchaseError> {
        owner(request.role)?;
        let mut purchases = self.purchases.clone();
        let mut catalog = self.catalog.clone();
        let order = purchases
            .orders
            .get_mut(request.id.0)
            .ok_or(PurchaseError::OrderNotFound)?;
        if request.lines.len() != order.items.len() || order.cancelled {
            return Err(PurchaseError::InvalidReceipt);
        }
        let mut any = false;
        for (line, input) in order.items.iter_mut().zip(request.lines) {
            if input.rate.minor_units() < 0 {
                return Err(PurchaseError::InvalidReceipt);
            }
            let quantity = input.quantity.min(line.remaining());
            let Some(quantity) = NonZeroU32::new(quantity) else {
                continue;
            };
            let rate = input.rate;
            if rate.minor_units() > 0 {
                line.line.rate = rate;
            }
            let product_index = line
                .line
                .product_id
                .and_then(|id| purchases.products.iter().position(|p| p.id == id))
                .or_else(|| {
                    purchases
                        .products
                        .iter()
                        .position(|p| p.item == line.line.item)
                });
            if let Some(product) = product_index.map(|index| &mut purchases.products[index]) {
                let before = catalog
                    .stock(product.id)
                    .ok_or(PurchaseError::InvalidReceipt)?;
                let cost = match purchases
                    .costs
                    .iter_mut()
                    .find(|cost| cost.product == product.id)
                {
                    Some(cost) => cost,
                    None => {
                        purchases.costs.push(PurchaseCost {
                            product: product.id,
                            average: product.buy_rate,
                        });
                        purchases
                            .costs
                            .last_mut()
                            .ok_or(PurchaseError::InvalidReceipt)?
                    }
                };
                let previous = cost.average.minor_units();
                let incoming = if rate.minor_units() > 0 {
                    rate.minor_units()
                } else {
                    previous
                };
                let total = i128::from(before) * i128::from(previous)
                    + i128::from(quantity.get()) * i128::from(incoming);
                let count = i128::from(before) + i128::from(quantity.get());
                cost.average = Money::from_minor_units(
                    i64::try_from((total + count / 2) / count)
                        .map_err(|_| PurchaseError::ReceiptOverflow)?,
                );
                if rate.minor_units() > 0 {
                    catalog
                        .record_receipt_buy(product.id, rate)
                        .map_err(|_| PurchaseError::ReceiptOverflow)?;
                    product.buy_rate = rate;
                    purchases
                        .quotes
                        .entry(product.id.value())
                        .or_default()
                        .record_receipt(&order.supplier_name, rate);
                }
                let fifo_rate = if rate.minor_units() > 0 {
                    rate
                } else if line.line.rate.minor_units() > 0 {
                    line.line.rate
                } else {
                    cost.average
                };
                catalog
                    .receive_lot(
                        product.id,
                        request.date.unwrap_or(request.today),
                        quantity.get(),
                        fifo_rate,
                    )
                    .map_err(|_| PurchaseError::ReceiptOverflow)?;
            }
            line.received = line
                .received
                .checked_add(quantity.get())
                .ok_or(PurchaseError::ReceiptOverflow)?;
            line.receipts.push(PurchaseReceipt {
                date: request.date.unwrap_or(request.today),
                quantity,
                rate,
            });
            any = true;
        }
        if !any {
            return Ok(ReceiveResult::NothingToReceive);
        }
        let movement = self
            .movement_epoch
            .checked_add(1)
            .ok_or(PurchaseError::ReceiptOverflow)?;
        self.purchases = purchases;
        self.catalog = catalog;
        self.movement_epoch = movement;
        Ok(ReceiveResult::Received)
    }

    pub fn purchase_cost(
        &self,
        role: Role,
        id: ProductId,
        supplier: &str,
    ) -> Result<Option<(Money, Money, Option<Money>)>, PurchaseError> {
        owner(role)?;
        Ok(self
            .purchases
            .products
            .iter()
            .find(|p| p.id == id)
            .map(|p| {
                let cost = self.purchases.costs.iter().find(|cost| cost.product == id);
                let average = cost.map_or(p.buy_rate, |cost| cost.average);
                let quote = self
                    .purchases
                    .quotes
                    .get(&id.value())
                    .and_then(|quotes| quotes.receipt_quote(supplier));
                (average, p.buy_rate, quote)
            }))
    }

    pub fn purchase_movements(
        &self,
        role: Role,
        id: PurchaseOrderId,
    ) -> Result<Vec<(BillDate, u32)>, PurchaseError> {
        owner(role)?;
        let order = self
            .purchases
            .orders
            .get(id.0)
            .ok_or(PurchaseError::OrderNotFound)?;
        Ok(order
            .items
            .iter()
            .flat_map(|line| {
                line.receipts
                    .iter()
                    .map(|receipt| (receipt.date, receipt.quantity.get()))
            })
            .collect())
    }

    pub fn purchase_received_on(
        &self,
        role: Role,
        id: PurchaseOrderId,
        date: BillDate,
    ) -> Result<u32, PurchaseError> {
        owner(role)?;
        let order = self
            .purchases
            .orders
            .get(id.0)
            .ok_or(PurchaseError::OrderNotFound)?;
        order
            .items
            .iter()
            .flat_map(|line| &line.receipts)
            .filter(|receipt| receipt.date == date)
            .try_fold(0_u32, |total, receipt| {
                total
                    .checked_add(receipt.quantity.get())
                    .ok_or(PurchaseError::ReceiptOverflow)
            })
    }
}
