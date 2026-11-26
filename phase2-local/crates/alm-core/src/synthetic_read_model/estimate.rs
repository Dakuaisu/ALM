use super::{Error, EstimateDocument, EstimateRow, LineKind, ReadModel, SaleId};
use crate::bill_math::round_balance;
use crate::confirmation::estimate::render_estimate;
use crate::confirmation::{BalanceMode, PreviousBalance};
use crate::{BillDate, Money, Role};

impl ReadModel {
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
                label: std::borrow::Cow::Borrowed(&line.item),
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

    /// The Latest-balance estimate of any archived bill, as the live app shows it
    /// (`BalanceMode::Latest`): the customer's archived closing balance rounded once to whole
    /// rupees, no previous-balance row.
    ///
    /// # Errors
    /// As [`Self::archived_estimate`].
    pub fn latest_estimate(
        &self,
        role: Role,
        id: &SaleId,
        today: BillDate,
    ) -> Result<EstimateDocument<'_>, Error> {
        self.archived_estimate(role, id, today, BalanceMode::Latest)
    }

    /// The estimate of any archived bill (item and crate lines) in `mode`, as the live app shows
    /// it (`invBalanceMode`): Latest is the customer's archived closing balance rounded once to
    /// whole rupees; "as on bill date" (and Latest for a bill without a customer ledger) is the
    /// archived previous balance plus the bill's own due, with the Prev Balance row and its date.
    ///
    /// # Errors
    /// Denies Managers; rejects unknown or duplicated sales, inconsistent amounts, a Latest
    /// balance without the customer, and "as on bill date" for archives written before it was
    /// recorded.
    pub fn archived_estimate(
        &self,
        role: Role,
        id: &SaleId,
        today: BillDate,
        mode: BalanceMode,
    ) -> Result<EstimateDocument<'_>, Error> {
        let records = self.records(role)?;
        let mut matches = records.sales.iter().filter(|sale| sale.id == id.value());
        let sale = matches.next().ok_or(Error::InvalidArchive)?;
        if matches.next().is_some() || sale.lines.is_empty() {
            return Err(Error::InvalidArchive);
        }
        let mut total = 0_i64;
        let mut quantity = 0_u32;
        let mut rows = Vec::with_capacity(sale.lines.len());
        for line in &sale.lines {
            if line.rate_minor.checked_mul(i64::from(line.quantity)) != Some(line.amount_minor) {
                return Err(Error::InvalidArchive);
            }
            total = total
                .checked_add(line.amount_minor)
                .ok_or(Error::InvalidArchive)?;
            quantity = quantity
                .checked_add(line.quantity)
                .ok_or(Error::InvalidArchive)?;
            rows.push(EstimateRow {
                label: match line.kind {
                    LineKind::Item => std::borrow::Cow::Borrowed(line.item.as_str()),
                    LineKind::Crate { .. } => std::borrow::Cow::Owned(line.display_item()),
                },
                quantity: line.quantity,
                price: Money::from_minor_units(line.rate_minor),
                amount: Money::from_minor_units(line.amount_minor),
            });
        }
        let loading = sale
            .total_minor
            .checked_sub(total)
            .ok_or(Error::InvalidArchive)?;
        let customer = records
            .customers
            .iter()
            .find(|customer| customer.name == sale.customer);
        let (balance_mode, balance, previous_balance) = match (mode, customer) {
            (BalanceMode::Latest, Some(customer)) => (
                BalanceMode::Latest,
                round_balance(customer.balance_minor).ok_or(Error::InvalidArchive)?,
                None,
            ),
            (BalanceMode::Latest, None) if sale.as_on_bill_date.is_none() => {
                return Err(Error::InvalidArchive);
            }
            _ => {
                let snapshot = sale.as_on_bill_date.as_ref().ok_or(Error::InvalidArchive)?;
                let own_due = sale
                    .total_minor
                    .checked_sub(sale.paid_minor)
                    .ok_or(Error::InvalidArchive)?;
                (
                    BalanceMode::AsOnBillDate,
                    snapshot
                        .previous_minor
                        .checked_add(own_due)
                        .ok_or(Error::InvalidArchive)?,
                    (snapshot.previous_minor != 0).then_some(PreviousBalance {
                        amount: Money::from_minor_units(snapshot.previous_minor),
                        date: snapshot.previous_date,
                    }),
                )
            }
        };
        Ok(EstimateDocument {
            invoice: sale.number,
            date: sale.date,
            customer: &sale.customer,
            lines: rows,
            items_quantity: quantity,
            items_total: Money::from_minor_units(total),
            loading: Money::from_minor_units(loading),
            total: Money::from_minor_units(sale.total_minor),
            paid: Money::from_minor_units(sale.paid_minor),
            cancelled: sale.cancelled,
            balance_mode,
            balance: Money::from_minor_units(balance),
            previous_balance,
            today,
        })
    }

    /// The printable estimate of `latest_estimate` with this archive's business profile.
    ///
    /// # Errors
    /// As [`Self::latest_estimate`].
    pub fn latest_estimate_html(
        &self,
        role: Role,
        id: &SaleId,
        today: BillDate,
    ) -> Result<String, Error> {
        self.archived_estimate_html(role, id, today, BalanceMode::Latest)
    }

    /// The printable estimate of `archived_estimate` with this archive's business profile.
    ///
    /// # Errors
    /// As [`Self::archived_estimate`].
    pub fn archived_estimate_html(
        &self,
        role: Role,
        id: &SaleId,
        today: BillDate,
        mode: BalanceMode,
    ) -> Result<String, Error> {
        let document = self.archived_estimate(role, id, today, mode)?;
        Ok(render_estimate(
            &document,
            Some(self.business_profile(role)?),
        ))
    }
}
