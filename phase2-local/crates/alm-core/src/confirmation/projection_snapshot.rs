//! Builds the role-filtered device projection from the authority's current state.
use super::manual_cashbook::ManualCashBook;
use super::owner_billing::{BillingDirectory, CustomerFilter};
use super::{ConfirmationError, ConfirmationFixture, SalesPeriod};
use crate::projection::{
    OwnerValues, ProjectedCashDay, ProjectedCustomer, ProjectedProduct, ProjectedReceipt,
    ProjectedSale, ProjectedStock, ProjectionSnapshot, customer_name, online,
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
        Ok(ProjectionSnapshot {
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
            }),
        })
    }
}
