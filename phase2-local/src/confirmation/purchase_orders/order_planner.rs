use std::num::NonZeroU32;

use crate::{BillDate, Money, ProductId, Role};

use super::{
    ConfirmationFixture, PurchaseDraft, PurchaseError, PurchaseLine, PurchaseOrderId,
    PurchaseSupplier, owner,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OrderPlannerError {
    AccessDenied,
    EmptyCart,
    QuantityOutOfRange,
    AmountOverflow,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderPlanRow {
    pub product: ProductId,
    pub item: String,
    pub on_hand: u32,
    pub reorder_level: u32,
    pub suggested_quantity: i64,
    pub unit_rate: Money,
    pub best_supplier: Option<PurchaseSupplier>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderCartLine {
    pub product: ProductId,
    pub item: String,
    pub quantity: i64,
    pub unit_rate: Money,
    pub best_supplier: Option<PurchaseSupplier>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderPlanner {
    rows: Vec<OrderPlanRow>,
    cart: Vec<OrderCartLine>,
}

impl OrderPlanner {
    pub fn rows(&self, low_only: bool) -> impl Iterator<Item = &OrderPlanRow> {
        self.rows
            .iter()
            .filter(move |row| !low_only || row.on_hand <= row.reorder_level)
    }

    pub const fn cart(&self) -> &[OrderCartLine] {
        self.cart.as_slice()
    }

    /// Captures the displayed rate and supplier. Blank or zero before Add uses the suggestion.
    pub fn add_item(&mut self, product: ProductId, quantity: Option<i64>) -> bool {
        let Some(row) = self.rows.iter().find(|row| row.product == product) else {
            return false;
        };
        if self.cart.iter().any(|line| line.item == row.item) {
            return false;
        }
        self.cart.push(OrderCartLine {
            product,
            item: row.item.clone(),
            quantity: quantity
                .filter(|&qty| qty != 0)
                .unwrap_or(row.suggested_quantity),
            unit_rate: row.unit_rate,
            best_supplier: row.best_supplier.clone(),
        });
        true
    }

    /// Adds supplied visible group members in order without changing existing cart lines.
    pub fn add_all(
        &mut self,
        role: Role,
        members: &[(ProductId, Option<i64>)],
    ) -> Result<(), OrderPlannerError> {
        owner(role).map_err(|_| OrderPlannerError::AccessDenied)?;
        for &(product, quantity) in members {
            self.add_item(product, quantity);
        }
        Ok(())
    }

    /// Editing a cart line to zero keeps it in the cart, including its best supplier.
    pub fn set_quantity(&mut self, product: ProductId, quantity: i64) -> bool {
        let Some(line) = self.cart.iter_mut().find(|line| line.product == product) else {
            return false;
        };
        line.quantity = quantity;
        true
    }

    pub fn remove_item(&mut self, product: ProductId) -> bool {
        let len = self.cart.len();
        self.cart.retain(|line| line.product != product);
        self.cart.len() != len
    }

    pub fn cart_total(&self) -> Result<Money, OrderPlannerError> {
        self.cart
            .iter()
            .try_fold(0_i64, |total, line| {
                total.checked_add(line.unit_rate.minor_units().checked_mul(line.quantity)?)
            })
            .map(Money::from_minor_units)
            .ok_or(OrderPlannerError::AmountOverflow)
    }

    /// Opens a detached PO draft. Supplier inference sees every cart line, even omitted quantities.
    pub fn checkout(&self, role: Role, date: BillDate) -> Result<PurchaseDraft, OrderPlannerError> {
        owner(role).map_err(|_| OrderPlannerError::AccessDenied)?;
        let items = self
            .cart
            .iter()
            .filter(|line| line.quantity > 0)
            .map(|line| {
                let ordered = u32::try_from(line.quantity)
                    .and_then(NonZeroU32::try_from)
                    .map_err(|_| OrderPlannerError::QuantityOutOfRange)?;
                Ok(PurchaseLine {
                    item: line.item.clone(),
                    product_id: Some(line.product),
                    ordered,
                    rate: line.unit_rate,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        if items.is_empty() {
            return Err(OrderPlannerError::EmptyCart);
        }
        let mut candidates = self
            .cart
            .iter()
            .filter_map(|line| line.best_supplier.as_ref());
        let supplier = candidates
            .next()
            .filter(|first| candidates.all(|candidate| candidate.name == first.name))
            .cloned();
        Ok(PurchaseDraft {
            supplier,
            date: Some(date),
            note: String::new(),
            items,
        })
    }
}

impl ConfirmationFixture {
    pub fn open_order_planner(&self, role: Role) -> Result<OrderPlanner, OrderPlannerError> {
        owner(role).map_err(|_| OrderPlannerError::AccessDenied)?;
        let columns = self.purchases.price_suppliers();
        let rows = self
            .catalog
            .order_planner_stock(self.product_reorder_default)
            .filter_map(|(product, on_hand, reorder_level)| {
                let (item, buy_rate) = self.supplier_price_product(product)?;
                let quote = self.purchases.reorder_quote(product, &columns);
                Some(OrderPlanRow {
                    product,
                    item: item.to_owned(),
                    on_hand,
                    reorder_level,
                    suggested_quantity: (i64::from(reorder_level) * 3 - i64::from(on_hand)).max(1),
                    unit_rate: quote.map_or(buy_rate, |(_, rate)| rate),
                    best_supplier: quote.map(|(name, _)| PurchaseSupplier {
                        name: name.to_owned(),
                        id: columns
                            .iter()
                            .find(|supplier| supplier.name == name)
                            .map(|s| s.id),
                    }),
                })
            })
            .collect();
        Ok(OrderPlanner {
            rows,
            cart: Vec::new(),
        })
    }

    /// Saves through the ordinary PO path, removing only the saved item names after success.
    pub fn save_order_plan(
        &mut self,
        role: Role,
        plan: &mut OrderPlanner,
        draft: &PurchaseDraft,
    ) -> Result<PurchaseOrderId, PurchaseError> {
        let id = self.save_purchase_order(role, draft)?;
        let saved = &self.purchases.orders[id.0];
        plan.cart
            .retain(|entry| !saved.items.iter().any(|line| line.line.item == entry.item));
        Ok(id)
    }
}
