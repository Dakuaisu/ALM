//! Builds the role-filtered device projection from the authority's current state.
use super::manual_cashbook::ManualCashBook;
use super::owner_billing::{BillingDirectory, CustomerFilter};
use super::{ConfirmationError, ConfirmationFixture, SalesPeriod};
use crate::projection::{
    OwnerValues, ProjectedCashDay, ProjectedCrate, ProjectedCustomer, ProjectedGodownRow,
    ProjectedGstBill, ProjectedProduct, ProjectedProductTerms, ProjectedPurchaseLine,
    ProjectedPurchaseOrder, ProjectedReceipt, ProjectedSale, ProjectedStock, ProjectedSupplier,
    ProjectionSnapshot, customer_name, online,
};
use crate::{BillDate, Money, Role};

impl ConfirmationFixture {
    pub(crate) fn projection_snapshot(
        &self,
        role: Role,
        directory: &BillingDirectory,
        cashbook: &ManualCashBook,
    ) -> Result<ProjectionSnapshot, ConfirmationError> {
        let catalog = directory
            .products
            .iter()
            .filter(|product| self.catalog.item_name(product.id).is_some())
            .map(|product| ProjectedProduct {
                id: product.id.value(),
                name: product.name.to_owned(),
                rate: product.provisional_rate,
            })
            .collect();
        if role != Role::Owner {
            return Ok(ProjectionSnapshot {
                catalog,
                ..ProjectionSnapshot::default()
            });
        }
        // The sales list's own period filter needs a date; `All` ignores it.
        let any_day = BillDate::new(2000, 1, 1).map_err(|_| ConfirmationError::InvalidDraft)?;
        let (history, sales_unsupported) =
            match self.sales_history(role, any_day, SalesPeriod::All, "") {
                Ok(history) => (history.rows, None),
                Err(ConfirmationError::UnsupportedSalesRecord(id)) => {
                    (Vec::new(), Some(id.value()))
                }
                Err(error) => return Err(error),
            };
        let zero = Money::from_minor_units(0);
        let mut sales = Vec::new();
        let mut sale_costs = Vec::new();
        for row in &history {
            let id = row.bill.draft_id;
            let (review, _, _) = self
                .current_sales
                .get(&id)
                .ok_or(ConfirmationError::InvalidDraft)?;
            let items = self
                .posted_sale_items
                .get(&id)
                .map(|items| items.iter().filter_map(|i| i.item.clone()).collect())
                .unwrap_or_default();
            sales.push(ProjectedSale {
                draft_id: id.value(),
                invoice: row.bill.invoice,
                date: row.date,
                customer: customer_name(review.draft.customer()),
                items,
                lines: u32::try_from(review.draft.lines().len())
                    .map_err(|_| ConfirmationError::Overflow)?,
                packets: row.bill.quantity,
                total: row.bill.total,
                paid: row.bill.received.unwrap_or(zero),
                online: online(review.payment_mode),
                cancelled: row.cancelled,
                balance: row.balance,
            });
            if !row.cancelled {
                let cost = self.catalog.frozen_cost(id)?.map(Money::from_minor_units);
                sale_costs.push((id.value(), review.amounts()?.subtotal, cost));
            }
        }
        let receipts = self
            .preview_payments
            .iter()
            .enumerate()
            .map(|(slot, payment)| {
                Ok(ProjectedReceipt {
                    slot: u64::try_from(slot).map_err(|_| ConfirmationError::Overflow)?,
                    date: payment.date,
                    customer: payment.metadata.as_ref().map_or_else(
                        || {
                            directory
                                .customer(self, &payment.customer_id)
                                .map_or_else(|_| payment.customer_id.clone(), |c| c.name.into())
                        },
                        |metadata| metadata.selected_name.clone(),
                    ),
                    amount: payment.amount,
                    online: payment.mode.is_some_and(online),
                })
            })
            .collect::<Result<_, ConfirmationError>>()?;
        let customers = directory
            .customer_book(self, role, &CustomerFilter::All)?
            .rows
            .into_iter()
            .map(|row| ProjectedCustomer {
                id: row.customer.id.to_owned(),
                name: row.customer.name.to_owned(),
                balance: row.balance,
                mobile: row.customer.mobile.to_owned(),
                addr1: row.customer.addr1.to_owned(),
                addr2: row.customer.addr2.to_owned(),
            })
            .collect();
        let list = self
            .stock_list(role, "", false)
            .map_err(|_| ConfirmationError::InvalidDraft)?;
        let stock = list
            .rows
            .iter()
            .map(|row| ProjectedStock {
                product: row.product.value(),
                item: row.item.clone(),
                brand: row.brand.clone(),
                on_hand: row.on_hand,
                reorder_level: row.reorder_level,
            })
            .collect();
        let cash_days = cashbook
            .rows(role)
            .map_err(|_| ConfirmationError::AccessDenied)?
            .into_iter()
            .map(|row| ProjectedCashDay {
                id: row.id.value(),
                date: row.date,
                amounts: row.amounts,
            })
            .collect();
        let mut snapshot = ProjectionSnapshot {
            sales_unsupported,
            sales,
            receipts,
            customers,
            stock,
            catalog,
            cash_days,
            owner: Some(OwnerValues {
                sale_costs,
                stock_value: list.value,
                ..OwnerValues::default()
            }),
            ..ProjectionSnapshot::default()
        };
        self.project_books(&mut snapshot)?;
        if let Some(owner) = snapshot.owner.as_mut() {
            owner.book = self.owner_book(role, cashbook)?;
        }
        Ok(snapshot)
    }

    /// Products, suppliers, purchase orders, crates, godown and GST bills (Owner only).
    fn project_books(&self, snapshot: &mut ProjectionSnapshot) -> Result<(), ConfirmationError> {
        let role = Role::Owner;
        let index = |value: usize| u64::try_from(value).map_err(|_| ConfirmationError::Overflow);
        let levels: std::collections::BTreeMap<_, _> = self
            .catalog
            .order_planner_stock(self.product_reorder_default)
            .map(|(id, _, reorder)| (id.value(), reorder))
            .collect();
        let owner = snapshot.owner.get_or_insert_with(OwnerValues::default);
        for row in self
            .product_list(role, "", None)
            .map_err(|_| ConfirmationError::AccessDenied)?
            .rows
        {
            let id = row.product.value();
            owner.buy_rates.push((id, row.buy));
            snapshot.products.push(ProjectedProductTerms {
                id,
                item: row.item,
                brand: row.brand,
                sell: row.sell,
                reorder_level: levels
                    .get(&id)
                    .copied()
                    .unwrap_or(self.product_reorder_default),
            });
        }
        for supplier in self
            .purchase_suppliers(role)
            .map_err(|_| ConfirmationError::AccessDenied)?
        {
            snapshot.suppliers.push(ProjectedSupplier {
                id: index(supplier.id.index())?,
                name: supplier.name.clone(),
            });
        }
        let grid = self
            .compare_supplier_prices(role, None)
            .map_err(|_| ConfirmationError::AccessDenied)?;
        for row in &grid.rows {
            for (supplier, rate) in grid.suppliers.iter().zip(&row.prices) {
                if let Some(rate) = rate {
                    owner.supplier_prices.push((
                        row.product.value(),
                        index(supplier.id.index())?,
                        *rate,
                    ));
                }
            }
        }
        for order in self
            .purchase_orders(role)
            .map_err(|_| ConfirmationError::AccessDenied)?
        {
            owner.purchase_orders.push(ProjectedPurchaseOrder {
                id: index(order.id.index())?,
                supplier: order.supplier_name.clone(),
                date: order.date,
                status: format!("{:?}", order.status()),
                lines: order
                    .items
                    .iter()
                    .map(|line| ProjectedPurchaseLine {
                        item: line.line.item.clone(),
                        product: line.line.product_id.map(crate::ProductId::value),
                        ordered: line.line.ordered.get(),
                        received: line.received,
                        remaining: line.remaining(),
                        rate: line.line.rate,
                    })
                    .collect(),
            });
        }
        for row in self
            .crate_template_list(role)
            .map_err(|_| ConfirmationError::AccessDenied)?
        {
            snapshot.crates.push(ProjectedCrate {
                id: row.id.value(),
                name: row.template.name.clone(),
                price: row.template.price,
                components: u32::try_from(row.template.components.len())
                    .map_err(|_| ConfirmationError::Overflow)?,
            });
        }
        for row in self
            .open_manual_godown(role)
            .map_err(|_| ConfirmationError::AccessDenied)?
        {
            snapshot.godown.push(ProjectedGodownRow {
                item: row.item.clone(),
                cases: row.cases.get(),
                note: row.note.clone().unwrap_or_default(),
            });
        }
        for bill in self
            .gst_bill_ledger(role)
            .map_err(|_| ConfirmationError::AccessDenied)?
            .bills
        {
            snapshot.gst_bills.push(ProjectedGstBill {
                id: bill.id.0,
                number: bill.number.0,
                date: bill.date,
                customer: bill.customer.clone(),
                cancelled: bill.cancelled,
            });
        }
        Ok(())
    }
}
