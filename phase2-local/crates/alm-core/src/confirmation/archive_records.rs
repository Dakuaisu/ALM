//! Projects the live fixture into the archive `Records` the backup, restore and structured
//! exports work on (legacy `DB` products / customers / sales / payments). Values are copied, not
//! recalculated: totals, paid and balances are the fixture's.
use super::owner_billing::BillingDirectory;
use super::{ConfirmationError, ConfirmationFixture, ReceiptMode};
use crate::synthetic_read_model::{Customer, Line, LineKind, Product, Receipt, Records, Sale};
use crate::{DraftCustomer, DraftCustomerId, Money, Role};

fn mode(mode: ReceiptMode) -> &'static str {
    match mode {
        ReceiptMode::Cash => "Cash",
        ReceiptMode::Online => "Online",
    }
}

impl ConfirmationFixture {
    /// The archive records for this device's synthetic shop. Crate and checkout bills are not
    /// representable in the archive line model yet and are rejected.
    ///
    /// # Errors
    /// Denies Managers; rejects crate/checkout bills and inconsistent records.
    pub fn archive_records(
        &self,
        role: Role,
        directory: &BillingDirectory,
    ) -> Result<Records, ConfirmationError> {
        if role != Role::Owner {
            return Err(ConfirmationError::AccessDenied);
        }
        let levels: std::collections::BTreeMap<_, _> = self
            .catalog
            .order_planner_stock(self.product_reorder_default)
            .map(|(id, _, reorder)| (id.value(), reorder))
            .collect();
        let rows = self
            .product_list(role, "", None)
            .map_err(|_| ConfirmationError::AccessDenied)?
            .rows;
        let products = rows
            .into_iter()
            .map(|row| {
                let detail = directory.products.iter().find(|p| p.id == row.product);
                Product {
                    reorder: levels
                        .get(&row.product.value())
                        .copied()
                        .unwrap_or(self.product_reorder_default),
                    item: row.item,
                    company: row.brand,
                    pack: detail.map_or_else(String::new, |d| d.pack_size.to_owned()),
                    uom: detail.map_or_else(String::new, |d| d.uom.to_owned()),
                    packet_quantity: detail.map_or(1, |d| d.packet_quantity.max(1)),
                    mrp_minor: detail.and_then(|d| d.mrp).map_or(0, Money::minor_units),
                    buy_minor: row.buy.minor_units(),
                    sell_minor: row.sell.minor_units(),
                    purchased: 0,
                    sold: 0,
                    on_hand: i64::from(row.on_hand),
                }
            })
            .collect();
        let mut customers = Vec::new();
        for customer in directory.customers(self) {
            let ledger = self.customer_ledger(role, &DraftCustomerId::new(customer.id.into()))?;
            customers.push(Customer {
                name: customer.name.into(),
                mobile: customer.mobile.into(),
                address: customer.address(),
                balance_minor: ledger.closing.minor_units(),
            });
        }
        let mut sales = Vec::new();
        for (id, (review, receipt, _)) in &self.current_sales {
            if (review.checkout.is_some() || review.draft.crate_row().is_some())
                && !review.is_named_crate_sale()
            {
                return Err(ConfirmationError::UnsupportedSalesRecord(*id));
            }
            let customer = match review.draft.customer() {
                DraftCustomer::Selected { name, .. } | DraftCustomer::Typed(name) => name.clone(),
                DraftCustomer::Unassigned => String::new(),
            };
            let date = review
                .draft
                .bill_date()
                .ok_or(ConfirmationError::InvalidDraft)?;
            let paid = receipt.received.ok_or(ConfirmationError::InvalidDraft)?;
            let credit = receipt.due.is_some_and(|due| due.minor_units() > 0);
            let mut lines = Vec::new();
            for (index, line) in review.draft.lines().iter().enumerate() {
                let (quantity, price) = review.line_values(index)?;
                let detail = directory
                    .products
                    .iter()
                    .find(|p| p.id == line.product_id());
                let amount = price
                    .minor_units()
                    .checked_mul(i64::from(quantity))
                    .ok_or(ConfirmationError::Overflow)?;
                lines.push(Line {
                    item: self
                        .catalog
                        .item_name(line.product_id())
                        .unwrap_or_default()
                        .to_owned(),
                    pack: detail.map_or_else(String::new, |d| d.pack_size.to_owned()),
                    uom: detail.map_or_else(String::new, |d| d.uom.to_owned()),
                    packet_quantity: detail.map_or(1, |d| d.packet_quantity.max(1)),
                    kind: LineKind::Item,
                    rate_minor: price.minor_units(),
                    quantity,
                    amount_minor: amount,
                });
            }
            if let (Some(row), [price]) = (review.draft.crate_row(), review.prices.as_slice()) {
                let quantity = row.quantity().get();
                lines.push(Line {
                    item: row.name().unwrap_or_default().to_owned(),
                    pack: String::new(),
                    uom: String::new(),
                    packet_quantity: 1,
                    kind: LineKind::Crate {
                        components: row
                            .components()
                            .iter()
                            .map(|component| crate::synthetic_read_model::Component {
                                item: self
                                    .catalog
                                    .item_name(component.product)
                                    .unwrap_or_default()
                                    .to_owned(),
                                quantity: component.quantity.get(),
                            })
                            .collect(),
                    },
                    rate_minor: price.minor_units(),
                    quantity,
                    amount_minor: price
                        .minor_units()
                        .checked_mul(i64::from(quantity))
                        .ok_or(ConfirmationError::Overflow)?,
                });
            }
            sales.push(Sale {
                id: format!("sale-{}", id.value()),
                date,
                number: receipt.invoice,
                customer,
                mode: if credit {
                    "Credit"
                } else {
                    mode(review.payment_mode)
                }
                .into(),
                lines,
                total_minor: receipt.total.minor_units(),
                paid_minor: paid.minor_units(),
                cancelled: self
                    .lifecycle
                    .get(id)
                    .is_some_and(|(cancelled, _)| *cancelled),
            });
        }
        sales.sort_by_key(|sale| sale.number);
        let mut receipts = Vec::new();
        for (slot, payment) in self.preview_payments.iter().enumerate() {
            let customer = match &payment.metadata {
                Some(metadata) => metadata.selected_name.clone(),
                None => directory
                    .customer(self, &payment.customer_id)
                    .map(|c| c.name.to_owned())
                    .unwrap_or_else(|_| payment.customer_id.clone()),
            };
            receipts.push(Receipt {
                id: format!("receipt-{slot}"),
                date: payment.date,
                mode: payment.mode.map_or("Cash", mode).into(),
                customer,
                amount_minor: payment.amount.minor_units(),
            });
        }
        Ok(Records {
            reorder_default: self.product_reorder_default,
            products,
            customers,
            sales,
            receipts,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::Role;
    use crate::confirmation::demo_fixture::{directory, fixture};
    use crate::confirmation::owner_billing::{ReceiptTerms, save_receipt};
    use crate::confirmation::{ConfirmationError, ReceiptMode};

    #[test]
    fn records_copy_the_fixture_products_customers_and_receipts() {
        let mut f = fixture().unwrap();
        save_receipt(
            &mut f,
            &directory(),
            Role::Owner,
            "batch1-market",
            ReceiptTerms {
                date: "2025-05-01",
                amount: "30.50",
                mode: ReceiptMode::Online,
            },
        )
        .unwrap();
        let records = f.archive_records(Role::Owner, &directory()).unwrap();
        let products: Vec<_> = records
            .products
            .iter()
            .map(|p| (p.item.as_str(), p.sell_minor, p.on_hand))
            .collect();
        assert_eq!(
            products,
            [
                ("Synthetic Lime", 12_050, 50),
                ("Synthetic Soda", 6_250, 40)
            ]
        );
        let customers: Vec<_> = records
            .customers
            .iter()
            .map(|c| (c.name.as_str(), c.balance_minor))
            .collect();
        assert_eq!(
            customers,
            [("Synthetic Customer", 0), ("Synthetic Market", -3_050)]
        );
        assert_eq!(records.receipts[0].customer, "Synthetic Market");
        assert_eq!(
            (
                records.receipts[0].mode.as_str(),
                records.receipts[0].amount_minor
            ),
            ("Online", 3_050)
        );
        assert!(records.sales.is_empty());
        assert_eq!(
            f.archive_records(Role::Manager, &directory()),
            Err(ConfirmationError::AccessDenied)
        );
    }
}
