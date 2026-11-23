//! Data `gstInvoiceHTML` prints for a saved standalone GST bill; markup stays with the caller.
use super::{GstBillError, GstBillInvoice, GstUnit};
use crate::Money;
use crate::confirmation::GstJurisdiction;
use crate::confirmation::estimate::display_date;
use crate::confirmation::gst_standalone_totals::hsn_summary;

/// One invoice row: `qcell`, `taxRate`, the `per` column and the taxable amount.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GstInvoiceLine {
    pub hsn: String,
    pub unit: GstUnit,
    pub quantity: u32,
    /// `qty * packetQty` for cases; `None` for pieces, which print only `N PCS`.
    pub pieces: Option<u64>,
    pub inclusive_rate: Money,
    pub taxable_rate: Money,
    pub taxable_amount: Money,
}

/// One HSN/SAC summary row, grouped by HSN and total rate in first-seen order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GstHsnSummary {
    pub hsn: String,
    pub gst_basis_points: u32,
    pub taxable: Money,
    /// CGST and SGST each at half the rate; zero on interstate bills.
    pub half_tax: Money,
    /// IGST at the full rate; zero on intrastate bills.
    pub integrated_tax: Money,
    pub total_tax: Money,
}

/// The selected customer, used only to fill a blank address, phone or GSTIN on print.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GstCustomerContact {
    pub id: String,
    pub addr1: String,
    pub addr2: String,
    pub mobile: String,
    pub gstin: String,
}

/// The "Buyer (Bill to)" block.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GstInvoiceParty {
    pub name: String,
    pub address: String,
    pub phone: String,
    pub email: String,
    pub gstin: String,
    pub state: String,
    pub state_code: String,
}

impl GstBillInvoice {
    /// `posCode || custStateCode || "09"` compared with the seller code "09".
    pub fn jurisdiction(&self) -> GstJurisdiction {
        self.record.jurisdiction()
    }

    /// `"ALM/" + fyStr(date) + "/" + no`; the financial year starts in April.
    pub fn invoice_number(&self) -> String {
        let (year, month, _) = self.record.date.parts();
        let start = if month >= 4 { year } else { year - 1 };
        format!(
            "ALM/{:02}-{:02}/{}",
            start % 100,
            (start + 1) % 100,
            self.record.number.0
        )
    }

    /// `dDisp(date)`.
    pub fn dated(&self) -> String {
        display_date(self.record.date)
    }

    /// `metaPairs` after the invoice number and date, in print order; transport rows only when set.
    pub fn meta(&self) -> Vec<(&'static str, String)> {
        let bill = &self.record;
        let mut pairs = vec![
            ("Mode of Payment", bill.pay_mode.clone()),
            ("Reverse Charge", bill.reverse_charge.clone()),
            (
                "Place of Supply",
                format!("{} ({})", bill.pos_state, bill.pos_code),
            ),
        ];
        for (label, value) in [
            ("Transporter", bill.transporter.as_str()),
            ("Vehicle No.", bill.vehicle_no.as_str()),
            ("LR / GR No.", bill.lr_no.as_str()),
            (
                "E-Way Bill No.",
                bill.eway_number.as_deref().unwrap_or_default(),
            ),
        ] {
            if !value.is_empty() {
                pairs.push((label, value.to_owned()));
            }
        }
        pairs
    }

    /// `totQty`: the Grand Total quantity, summed over every line regardless of unit.
    pub fn total_quantity(&self) -> u64 {
        self.record
            .lines
            .iter()
            .map(|line| u64::from(line.quantity.get()))
            .sum()
    }

    pub fn lines(&self) -> Result<Vec<GstInvoiceLine>, GstBillError> {
        self.record
            .lines
            .iter()
            .enumerate()
            .map(|(index, line)| {
                let quantity = line.quantity.get();
                Ok(GstInvoiceLine {
                    hsn: self.hsn_of(index),
                    unit: line.unit,
                    quantity,
                    pieces: match line.unit {
                        GstUnit::Piece => None,
                        GstUnit::Case => {
                            Some(u64::from(quantity) * u64::from(line.packet_qty.get()))
                        }
                    },
                    inclusive_rate: line.inclusive_rate,
                    taxable_rate: line.taxable_rate()?,
                    taxable_amount: line.taxable_amount()?,
                })
            })
            .collect()
    }

    pub fn hsn_summary(&self) -> Result<Vec<GstHsnSummary>, GstBillError> {
        let hsn: Vec<_> = (0..self.record.lines.len())
            .map(|index| self.hsn_of(index))
            .collect();
        hsn_summary(
            hsn.iter().zip(&self.record.lines).map(|(hsn, line)| {
                (
                    hsn.as_str(),
                    line.amount.minor_units(),
                    line.gst_basis_points,
                )
            }),
            self.jurisdiction(),
        )
    }

    fn hsn_of(&self, index: usize) -> String {
        self.line_hsn
            .get(index)
            .cloned()
            .unwrap_or_else(|| self.record.lines[index].hsn.clone())
    }

    /// Buyer block; `contact` is used only when its ID is the bill's `customer_id`.
    pub fn party(&self, contact: Option<&GstCustomerContact>) -> GstInvoiceParty {
        let bill = &self.record;
        let contact = contact.filter(|c| bill.customer_id.as_deref() == Some(c.id.as_str()));
        let pick = |own: &str, other: Option<String>| {
            if own.is_empty() {
                other.unwrap_or_default()
            } else {
                own.to_owned()
            }
        };
        GstInvoiceParty {
            name: bill.customer.clone(),
            address: pick(
                &bill.cust_addr,
                contact.map(|c| {
                    [c.addr1.as_str(), c.addr2.as_str()]
                        .into_iter()
                        .filter(|part| !part.is_empty())
                        .collect::<Vec<_>>()
                        .join(", ")
                }),
            ),
            phone: pick(&bill.cust_phone, contact.map(|c| c.mobile.clone())),
            email: bill.cust_email.clone(),
            gstin: pick(
                bill.gstin.as_deref().unwrap_or_default(),
                contact.map(|c| c.gstin.clone()),
            ),
            state: bill.cust_state.clone(),
            state_code: bill.cust_state_code.clone(),
        }
    }
}
