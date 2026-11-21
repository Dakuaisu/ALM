use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};

use super::{Line, LineKind, SaleId, Store};
use crate::confirmation::gst_invoice::{
    GstCalculationHeader, GstResolvedLine, calculate_gst_invoice,
};
use crate::confirmation::{GstError, GstInvoice, GstJurisdiction};
use crate::{Money, ProductId, Role};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HistoricalGst {
    pub sale: SaleId,
    pub jurisdiction: GstJurisdiction,
    pub lines: Vec<HistoricalGstLine>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HistoricalGstLine {
    pub position: usize,
    pub sale_line: Line,
    pub product: ProductId,
    pub hsn: String,
    pub rate: u32,
    pub cess: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GstArchiveError {
    AccessDenied,
    SaleNotFound,
    MissingTax,
    InvalidArchive,
    Calculation(GstError),
}

impl Store {
    pub fn emit_ordinary_gst(
        &self,
        role: Role,
        id: &SaleId,
        sink: &mut impl FnMut(GstInvoice),
    ) -> Result<(), GstArchiveError> {
        let records = self
            .records(role)
            .map_err(|_| GstArchiveError::AccessDenied)?;
        let mut sales = records.sales.iter().filter(|sale| sale.id == id.value());
        let sale = sales.next().ok_or(GstArchiveError::SaleNotFound)?;
        if sales.next().is_some() {
            return Err(GstArchiveError::InvalidArchive);
        }
        let mut taxes = self.gst.iter().flatten().filter(|tax| &tax.sale == id);
        let tax = taxes.next().ok_or(GstArchiveError::MissingTax)?;
        if taxes.next().is_some() || sale.lines.is_empty() || sale.lines.len() != tax.lines.len() {
            return Err(GstArchiveError::InvalidArchive);
        }
        let mut total = 0_i64;
        let mut resolved = Vec::with_capacity(sale.lines.len());
        for (position, (line, historical)) in sale.lines.iter().zip(&tax.lines).enumerate() {
            if historical.position != position
                || line != &historical.sale_line
                || line.item.trim().is_empty()
                || historical.hsn.trim().is_empty()
                || line.rate_minor < 0
                || line.amount_minor < 0
                || !matches!(line.kind, LineKind::Item)
                || line.rate_minor.checked_mul(i64::from(line.quantity)) != Some(line.amount_minor)
            {
                return Err(GstArchiveError::InvalidArchive);
            }
            let quantity = NonZeroU32::new(line.quantity).ok_or(GstArchiveError::InvalidArchive)?;
            total = total
                .checked_add(line.amount_minor)
                .ok_or(GstArchiveError::InvalidArchive)?;
            resolved.push(Ok(GstResolvedLine {
                product: historical.product,
                name: &line.item,
                quantity,
                hsn: &historical.hsn,
                rate: historical.rate,
                cess: historical.cess,
                price: Money::from_minor_units(line.rate_minor),
                amount: Money::from_minor_units(line.amount_minor),
            }));
        }
        if total != sale.total_minor || sale.paid_minor < 0 || sale.paid_minor > total {
            return Err(GstArchiveError::InvalidArchive);
        }
        let document = calculate_gst_invoice(
            GstCalculationHeader {
                invoice: sale.number,
                date: sale.date,
                customer: &sale.customer,
                cancelled: sale.cancelled,
                jurisdiction: tax.jurisdiction,
                line_count: sale.lines.len(),
                items_total: Money::from_minor_units(total),
                loading: Money::from_minor_units(0),
                total: Money::from_minor_units(sale.total_minor),
            },
            resolved.into_iter(),
        )
        .map_err(GstArchiveError::Calculation)?;
        sink(document);
        Ok(())
    }
}
