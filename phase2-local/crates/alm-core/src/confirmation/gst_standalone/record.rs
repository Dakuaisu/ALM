use super::{
    GstBill, GstBillError, GstBillInvoice, GstBillTotals, amount, totals, validate_references,
};

impl GstBill {
    pub(crate) fn validate_saved(&self) -> Result<(), GstBillError> {
        if self.lines.is_empty() || self.lines.iter().any(|line| line.item.is_empty()) {
            return Err(GstBillError::MissingItems);
        }
        if self.customer.is_empty() {
            return Err(GstBillError::MissingCustomer);
        }
        validate_references(self.gstin.as_deref(), self.eway_number.as_deref())?;
        for line in &self.lines {
            if amount(line.inclusive_rate, line.quantity.get())? != line.amount.minor_units() {
                return Err(GstBillError::InvalidAmount);
            }
        }
        self.totals().map(|_| ())
    }

    pub(crate) fn invoice(&self) -> Result<GstBillInvoice, GstBillError> {
        let totals = self.totals()?;
        Ok(GstBillInvoice {
            record: self.clone(),
            totals,
        })
    }

    fn totals(&self) -> Result<GstBillTotals, GstBillError> {
        totals(
            self.lines
                .iter()
                .map(|line| Ok((line.amount.minor_units(), line.gst_basis_points))),
            self.jurisdiction,
        )
    }
}
