use std::num::NonZeroU32;

use serde::Deserialize;

use super::{
    GstBill, GstBillDraft, GstBillError, GstBillId, GstBillInvoice, GstBillLine, GstBillNumber,
    GstBillTotals, GstUnit, amount, totals, validate_references,
};
use crate::confirmation::GstJurisdiction;
use crate::synthetic_read_model::{date, money};
use crate::{BillDate, Money, ProductId};

pub(super) const DEFAULT_STATE: &str = "Uttar Pradesh";
pub(super) const DEFAULT_STATE_CODE: &str = "09";
pub(super) const DEFAULT_PAY_MODE: &str = "Cash";
pub(super) const DEFAULT_REVERSE_CHARGE: &str = "No";
/// `gstInvoiceHTML` compares the place of supply with the seller's Uttar Pradesh code.
const SELLER_STATE_CODE: &str = "09";

fn or<'s>(value: &'s str, fallback: &'s str) -> &'s str {
    if value.is_empty() { fallback } else { value }
}

/// The `saveGstBill` header fallbacks, shared by saves and older archives.
struct Header {
    cust_state: String,
    cust_state_code: String,
    pos_state: String,
    pos_code: String,
    pay_mode: String,
    reverse_charge: String,
}

impl Header {
    fn new(
        cust_state: &str,
        cust_state_code: &str,
        pos_state: &str,
        pos_code: &str,
        pay_mode: &str,
        reverse_charge: &str,
    ) -> Self {
        let cust_state = or(cust_state, DEFAULT_STATE);
        let cust_state_code = or(cust_state_code, DEFAULT_STATE_CODE);
        Self {
            cust_state: cust_state.to_owned(),
            cust_state_code: cust_state_code.to_owned(),
            pos_state: or(pos_state, cust_state).to_owned(),
            pos_code: or(pos_code, cust_state_code).to_owned(),
            pay_mode: or(pay_mode, DEFAULT_PAY_MODE).to_owned(),
            reverse_charge: or(reverse_charge, DEFAULT_REVERSE_CHARGE).to_owned(),
        }
    }
}

fn non_empty(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_owned())
}

impl GstBill {
    /// The stored record as one line of JSON, as the authority path carries it.
    ///
    /// # Errors
    /// Fails only if the record does not serialize.
    pub fn to_record_text(&self) -> Result<String, GstBillError> {
        serde_json::to_string(self).map_err(|_| GstBillError::InvalidAmount)
    }

    /// Reads [`Self::to_record_text`] back.
    ///
    /// # Errors
    /// Rejects text that is not a stored record.
    pub fn from_record_text(text: &str) -> Result<Self, GstBillError> {
        serde_json::from_str(text).map_err(|_| GstBillError::InvalidAmount)
    }

    /// Builds the saved record; `product_named` resolves `prodByItem` for an unset `pid`.
    pub(super) fn from_draft(
        draft: &GstBillDraft<'_>,
        date: BillDate,
        product_named: impl Fn(&str) -> Option<ProductId>,
    ) -> Result<Self, GstBillError> {
        let lines = draft
            .lines
            .iter()
            .filter_map(|line| {
                let quantity = NonZeroU32::new(line.quantity)?;
                if line.item.is_empty() {
                    return None;
                }
                let value = amount(line.inclusive_rate, quantity.get());
                Some(value.map(|value| GstBillLine {
                    item: line.item.to_owned(),
                    product: line.product.or_else(|| product_named(line.item)),
                    hsn: line.hsn.to_owned(),
                    unit: line.unit,
                    quantity,
                    inclusive_rate: line.inclusive_rate,
                    gst_basis_points: line.gst_basis_points,
                    packet_qty: NonZeroU32::new(line.packet_qty).unwrap_or(NonZeroU32::MIN),
                    amount: Money::from_minor_units(value),
                }))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let header = Header::new(
            draft.cust_state,
            draft.cust_state_code,
            draft.pos_state,
            draft.pos_code,
            draft.pay_mode,
            draft.reverse_charge,
        );
        Ok(Self {
            id: draft.id,
            number: draft.number,
            customer_id: non_empty(draft.customer_id),
            customer: draft.customer.to_owned(),
            date,
            gstin: draft.gstin.map(str::to_owned),
            cust_addr: draft.cust_addr.to_owned(),
            cust_phone: draft.cust_phone.to_owned(),
            cust_email: draft.cust_email.to_owned(),
            cust_state: header.cust_state,
            cust_state_code: header.cust_state_code,
            pos_state: header.pos_state,
            pos_code: header.pos_code,
            pay_mode: header.pay_mode,
            pay_terms: draft.pay_terms.to_owned(),
            reverse_charge: header.reverse_charge,
            transporter: draft.transporter.to_owned(),
            vehicle_no: draft.vehicle_no.to_owned(),
            lr_no: draft.lr_no.to_owned(),
            eway_number: draft.eway_number.map(str::to_owned),
            terms_delivery: draft.terms_delivery.to_owned(),
            dispatch: draft.dispatch.to_owned(),
            cancelled: false,
            created: if draft.created == 0 {
                draft.now
            } else {
                draft.created
            },
            lines,
        })
    }

    /// `posCode || custStateCode || "09"` compared with the seller code "09".
    pub fn jurisdiction(&self) -> GstJurisdiction {
        let supply = or(
            or(&self.pos_code, &self.cust_state_code),
            DEFAULT_STATE_CODE,
        );
        if supply == SELLER_STATE_CODE {
            GstJurisdiction::Intra
        } else {
            GstJurisdiction::Inter
        }
    }

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

    /// Invoice data with the keyword `gstInfo` HSN guess for blank line HSNs.
    pub(crate) fn invoice(&self) -> Result<GstBillInvoice, GstBillError> {
        self.invoice_with(|item| {
            crate::confirmation::gst_defaults::defaults(item)
                .1
                .to_owned()
        })
    }

    /// Invoice data; `fallback_hsn` is `gstInfo(item).hsn` for lines saved without an HSN.
    pub(crate) fn invoice_with(
        &self,
        fallback_hsn: impl Fn(&str) -> String,
    ) -> Result<GstBillInvoice, GstBillError> {
        let invoice = GstBillInvoice {
            record: self.clone(),
            totals: self.totals()?,
            line_hsn: self
                .lines
                .iter()
                .map(|line| {
                    if line.hsn.is_empty() {
                        fallback_hsn(&line.item)
                    } else {
                        line.hsn.clone()
                    }
                })
                .collect(),
        };
        invoice.hsn_summary()?;
        Ok(invoice)
    }

    fn totals(&self) -> Result<GstBillTotals, GstBillError> {
        totals(
            self.lines
                .iter()
                .map(|line| Ok((line.amount.minor_units(), line.gst_basis_points))),
            self.jurisdiction(),
        )
    }
}

/// Archive form; fields added after the first archive version take the `saveGstBill` defaults.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ArchivedBill {
    id: GstBillId,
    number: GstBillNumber,
    #[serde(default)]
    customer_id: Option<String>,
    customer: String,
    #[serde(with = "date")]
    date: BillDate,
    gstin: Option<String>,
    #[serde(default)]
    cust_addr: String,
    #[serde(default)]
    cust_phone: String,
    #[serde(default)]
    cust_email: String,
    #[serde(default)]
    cust_state: String,
    #[serde(default)]
    cust_state_code: String,
    #[serde(default)]
    pos_state: String,
    #[serde(default)]
    pos_code: String,
    #[serde(default)]
    pay_mode: String,
    #[serde(default)]
    pay_terms: String,
    #[serde(default)]
    reverse_charge: String,
    #[serde(default)]
    transporter: String,
    #[serde(default)]
    vehicle_no: String,
    #[serde(default)]
    lr_no: String,
    eway_number: Option<String>,
    #[serde(default)]
    terms_delivery: String,
    #[serde(default)]
    dispatch: String,
    /// First-version archives stored an explicit flag; the place of supply now decides.
    #[serde(default, rename = "jurisdiction")]
    _legacy_jurisdiction: Option<GstJurisdiction>,
    cancelled: bool,
    #[serde(default)]
    created: u64,
    lines: Vec<GstBillLine>,
}

impl From<ArchivedBill> for GstBill {
    fn from(bill: ArchivedBill) -> Self {
        let header = Header::new(
            &bill.cust_state,
            &bill.cust_state_code,
            &bill.pos_state,
            &bill.pos_code,
            &bill.pay_mode,
            &bill.reverse_charge,
        );
        Self {
            id: bill.id,
            number: bill.number,
            customer_id: bill.customer_id.filter(|id| !id.is_empty()),
            customer: bill.customer,
            date: bill.date,
            gstin: bill.gstin,
            cust_addr: bill.cust_addr,
            cust_phone: bill.cust_phone,
            cust_email: bill.cust_email,
            cust_state: header.cust_state,
            cust_state_code: header.cust_state_code,
            pos_state: header.pos_state,
            pos_code: header.pos_code,
            pay_mode: header.pay_mode,
            pay_terms: bill.pay_terms,
            reverse_charge: header.reverse_charge,
            transporter: bill.transporter,
            vehicle_no: bill.vehicle_no,
            lr_no: bill.lr_no,
            eway_number: bill.eway_number,
            terms_delivery: bill.terms_delivery,
            dispatch: bill.dispatch,
            cancelled: bill.cancelled,
            created: bill.created,
            lines: bill.lines,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ArchivedLine {
    item: String,
    #[serde(default)]
    product: Option<ProductId>,
    #[serde(default)]
    hsn: String,
    #[serde(default)]
    unit: GstUnit,
    quantity: NonZeroU32,
    #[serde(with = "money")]
    inclusive_rate: Money,
    gst_basis_points: u32,
    #[serde(default = "one")]
    packet_qty: NonZeroU32,
    #[serde(with = "money")]
    amount: Money,
}

const fn one() -> NonZeroU32 {
    NonZeroU32::MIN
}

impl From<ArchivedLine> for GstBillLine {
    fn from(line: ArchivedLine) -> Self {
        Self {
            item: line.item,
            product: line.product,
            hsn: line.hsn,
            unit: line.unit,
            quantity: line.quantity,
            inclusive_rate: line.inclusive_rate,
            gst_basis_points: line.gst_basis_points,
            packet_qty: line.packet_qty,
            amount: line.amount,
        }
    }
}
