use super::{Error, EstimateDocument, EstimateRow, LineKind, SaleId, Store};
use crate::bill_math::round_balance;
use crate::confirmation::BalanceMode;
use crate::confirmation::estimate::render_estimate;
use crate::{BillDate, Money, Role};

impl Store {
    pub fn estimate_html(&self, role: Role, id: &SaleId, today: BillDate) -> Result<String, Error> {
        let document = self.estimate_document(role, id, today)?;
        Ok(render_estimate(
            &document,
            Some(self.business_profile(role)?),
        ))
    }

    pub fn estimate_document(
        &self,
        role: Role,
        id: &SaleId,
        today: BillDate,
    ) -> Result<EstimateDocument<'_>, Error> {
        let records = self.records(role)?;
        let scope = self.scope.as_ref().ok_or(Error::InvalidArchive)?;
        let links = scope.validate(records)?;
        let account = links.sales.get(id.value()).ok_or(Error::InvalidArchive)?;
        let mut matches = records.sales.iter().filter(|sale| sale.id == id.value());
        let sale = matches.next().ok_or(Error::InvalidArchive)?;
        if matches.next().is_some() || !sale.cancelled || sale.lines.is_empty() {
            return Err(Error::InvalidArchive);
        }
        let mut total = 0_i64;
        let mut quantity = 0_u32;
        let mut rows = Vec::with_capacity(sale.lines.len());
        for line in &sale.lines {
            match &line.kind {
                LineKind::Item => {}
                LineKind::Crate { .. } => return Err(Error::InvalidArchive),
            }
            if line.item.trim().is_empty()
                || line.quantity == 0
                || line.rate_minor.checked_mul(i64::from(line.quantity)) != Some(line.amount_minor)
            {
                return Err(Error::InvalidArchive);
            }
            total = total
                .checked_add(line.amount_minor)
                .ok_or(Error::InvalidArchive)?;
            quantity = quantity
                .checked_add(line.quantity)
                .ok_or(Error::InvalidArchive)?;
            rows.push(EstimateRow {
                label: &line.item,
                quantity: line.quantity,
                price: Money::from_minor_units(line.rate_minor),
                amount: Money::from_minor_units(line.amount_minor),
            });
        }
        if total != sale.total_minor {
            return Err(Error::InvalidArchive);
        }
        let mut balance = 0_i64;
        for row in &records.sales {
            if links.sales.get(row.id.as_str()) != Some(account) {
                continue;
            }
            if !row.cancelled {
                balance = balance
                    .checked_add(
                        row.total_minor
                            .checked_sub(row.paid_minor)
                            .ok_or(Error::InvalidArchive)?,
                    )
                    .ok_or(Error::InvalidArchive)?;
            }
        }
        for row in &records.receipts {
            if links.receipts.get(row.id.as_str()) != Some(account) {
                continue;
            }
            balance = balance
                .checked_sub(row.amount_minor)
                .ok_or(Error::InvalidArchive)?;
        }
        let balance = round_balance(balance).ok_or(Error::InvalidArchive)?;
        Ok(EstimateDocument {
            invoice: sale.number,
            date: sale.date,
            customer: &sale.customer,
            lines: rows,
            items_quantity: quantity,
            items_total: Money::from_minor_units(total),
            loading: Money::from_minor_units(0),
            total: Money::from_minor_units(sale.total_minor),
            paid: Money::from_minor_units(sale.paid_minor),
            cancelled: true,
            balance_mode: BalanceMode::Latest,
            balance: Money::from_minor_units(balance),
            previous_balance: None,
            today,
        })
    }
}
