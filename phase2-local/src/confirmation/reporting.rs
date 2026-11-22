use super::{ConfirmationError, ConfirmationFixture, DraftId, Money, Role};
use crate::DraftCustomer;

mod monthly;
pub use monthly::{CustomerRank, MonthlyProfit, ReportsError};

impl ConfirmationFixture {
    /// Gross profit excludes expenses and tax; active unknown cost is `None`, cancelled is zero.
    pub fn gross_profit(
        &self,
        role: Role,
        id: DraftId,
    ) -> Result<Option<Money>, ConfirmationError> {
        if role != Role::Owner {
            return Err(ConfirmationError::AccessDenied);
        }
        let (review, receipt, _) = self
            .current_sales
            .get(&id)
            .ok_or(ConfirmationError::DraftNotFound)?;
        super::cancellation::require_received_regular(review, receipt)?;
        let &(cancelled, _) = self
            .lifecycle
            .get(&id)
            .ok_or(ConfirmationError::DraftNotFound)?;
        if cancelled {
            return Ok(Some(Money::from_minor_units(0)));
        }
        self.catalog
            .frozen_cost(id)?
            .map(|cost| {
                receipt
                    .total
                    .minor_units()
                    .checked_sub(cost)
                    .map(Money::from_minor_units)
                    .ok_or(ConfirmationError::Overflow)
            })
            .transpose()
    }

    /// A scoped synthetic CSV, with amounts expressed in minor units and no cost fields.
    pub fn sales_csv(&self, role: Role, id: DraftId) -> Result<String, ConfirmationError> {
        if role != Role::Owner {
            return Err(ConfirmationError::AccessDenied);
        }
        let (review, receipt, _) = self
            .current_sales
            .get(&id)
            .ok_or(ConfirmationError::DraftNotFound)?;
        super::cancellation::require_received_regular(review, receipt)?;
        let &(cancelled, _) = self
            .lifecycle
            .get(&id)
            .ok_or(ConfirmationError::DraftNotFound)?;
        let DraftCustomer::Selected { name, .. } = review.draft.customer() else {
            return Err(ConfirmationError::InvalidDraft);
        };
        let date = review
            .draft
            .bill_date()
            .ok_or(ConfirmationError::InvalidDraft)?;
        let paid = receipt
            .received
            .ok_or(ConfirmationError::InvalidDraft)?
            .minor_units();
        let mut rows =
            vec!["Date,Bill No,Customer,Product,Price,Quantity,Total,Paid,Status".to_owned()];
        for (index, line) in review.draft.lines().iter().enumerate() {
            let (_, price) = review.line_values(index)?;
            let escaped = if name.contains([',', '"', '\n', '\r']) {
                format!("\"{}\"", name.replace('"', "\"\""))
            } else {
                name.clone()
            };
            let total = if index == 0 {
                receipt.total.minor_units().to_string()
            } else {
                String::new()
            };
            let paid = if index == 0 {
                paid.to_string()
            } else {
                String::new()
            };
            let status = if cancelled { "CANCELLED" } else { "" };
            rows.push(format!(
                "{date},{},{escaped},{},{},{},{total},{paid},{status}",
                receipt.invoice,
                line.product_id().value(),
                price.minor_units(),
                line.quantity().get()
            ));
        }
        Ok(rows.join("\n"))
    }
}
