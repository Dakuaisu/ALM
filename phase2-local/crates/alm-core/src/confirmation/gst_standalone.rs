//! Standalone synthetic GST bills; independent of ordinary sales and numbering.
//! Ports `nextGstNo`, `newGstDraft`, `fillGstItem`, `saveGstBill` and the list actions of the
//! frozen HTML. Nothing here reads or writes stock, lots, dues, the cash book or reports.
mod print;
mod record;
mod store;

use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};

use super::gst_standalone_totals::totals;
use super::{ConfirmationFixture, GstJurisdiction};
use crate::synthetic_read_model::{date, money};
use crate::{BillDate, Money, ProductId, Role};

pub use print::{GstCustomerContact, GstHsnSummary, GstInvoiceLine, GstInvoiceParty};
pub use store::{GstBillLedger, GstBillStore, GstBillStoreError};

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct GstBillId(pub u64);

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct GstBillNumber(pub u64);

/// The HTML `unit` select: `Case` or `Piece`; anything else is saved as `Case`.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum GstUnit {
    #[default]
    Case,
    Piece,
}

#[derive(Clone, Copy, Debug)]
pub struct GstBillLineInput<'a> {
    pub item: &'a str,
    /// Inert product reference (`pid`); no stock, FIFO, crate, report or ledger code reads it.
    pub product: Option<ProductId>,
    pub hsn: &'a str,
    pub unit: GstUnit,
    pub quantity: u32,
    pub inclusive_rate: Money,
    pub gst_basis_points: u32,
    /// Pieces per case; 0 is saved as 1.
    pub packet_qty: u32,
}

impl GstBillLineInput<'static> {
    /// `blankGstLine()`.
    pub const BLANK: Self = Self {
        item: "",
        product: None,
        hsn: "",
        unit: GstUnit::Case,
        quantity: 1,
        inclusive_rate: Money::from_minor_units(0),
        gst_basis_points: 0,
        packet_qty: 1,
    };
}

/// What `fillGstItem` copies into a line once an item is picked.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GstItemFill {
    pub product: Option<ProductId>,
    /// The picked product's pack size (1 when unset); `None` keeps the line's value.
    pub packet_qty: Option<u32>,
    /// Used only when the line rate is 0, as in the HTML.
    pub sell_price: Option<Money>,
    pub hsn: String,
    pub gst_basis_points: u32,
}

impl GstItemFill {
    pub fn apply<'a>(&'a self, item: &'a str, line: GstBillLineInput<'a>) -> GstBillLineInput<'a> {
        let inclusive_rate = match self.sell_price {
            Some(price) if line.inclusive_rate.minor_units() == 0 => price,
            Some(_) | None => line.inclusive_rate,
        };
        GstBillLineInput {
            item,
            product: self.product,
            hsn: &self.hsn,
            packet_qty: self.packet_qty.unwrap_or(line.packet_qty),
            inclusive_rate,
            gst_basis_points: self.gst_basis_points,
            ..line
        }
    }
}

/// Form state. Blank text fields take the `saveGstBill` defaults when saved.
#[derive(Clone, Copy, Debug, Default)]
pub struct GstBillDraft<'a> {
    pub id: GstBillId,
    pub number: GstBillNumber,
    pub customer_id: &'a str,
    pub customer: &'a str,
    pub date: Option<BillDate>,
    pub gstin: Option<&'a str>,
    pub cust_addr: &'a str,
    pub cust_phone: &'a str,
    pub cust_email: &'a str,
    pub cust_state: &'a str,
    pub cust_state_code: &'a str,
    pub pos_state: &'a str,
    pub pos_code: &'a str,
    pub pay_mode: &'a str,
    pub pay_terms: &'a str,
    pub reverse_charge: &'a str,
    pub transporter: &'a str,
    pub vehicle_no: &'a str,
    pub lr_no: &'a str,
    pub eway_number: Option<&'a str>,
    pub terms_delivery: &'a str,
    pub dispatch: &'a str,
    /// `created` of the bill being edited; 0 for a bill that was never saved.
    pub created: u64,
    /// The caller's clock (`Date.now()`), used as `created` on the first save.
    pub now: u64,
    pub lines: &'a [GstBillLineInput<'a>],
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(from = "record::ArchivedLine")]
pub struct GstBillLine {
    pub item: String,
    pub product: Option<ProductId>,
    pub hsn: String,
    pub unit: GstUnit,
    pub quantity: NonZeroU32,
    #[serde(with = "money")]
    pub inclusive_rate: Money,
    pub gst_basis_points: u32,
    pub packet_qty: NonZeroU32,
    #[serde(with = "money")]
    pub amount: Money,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GstBillTotals {
    pub taxable: Money,
    pub tax: Money,
    pub cgst: Money,
    pub sgst: Money,
    pub igst: Money,
    pub total: Money,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(from = "record::ArchivedBill")]
pub struct GstBill {
    pub id: GstBillId,
    pub number: GstBillNumber,
    pub customer_id: Option<String>,
    pub customer: String,
    #[serde(with = "date")]
    pub date: BillDate,
    pub gstin: Option<String>,
    pub cust_addr: String,
    pub cust_phone: String,
    pub cust_email: String,
    pub cust_state: String,
    pub cust_state_code: String,
    pub pos_state: String,
    pub pos_code: String,
    pub pay_mode: String,
    pub pay_terms: String,
    pub reverse_charge: String,
    pub transporter: String,
    pub vehicle_no: String,
    pub lr_no: String,
    pub eway_number: Option<String>,
    pub terms_delivery: String,
    pub dispatch: String,
    pub cancelled: bool,
    pub created: u64,
    pub lines: Vec<GstBillLine>,
}

/// A saved bill as `gstInvoiceHTML` prints it; the printed values are derived by methods.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GstBillInvoice {
    pub record: GstBill,
    pub totals: GstBillTotals,
    /// Each line's HSN, with `gstInfo(item).hsn` resolved for lines saved without one.
    pub line_hsn: Vec<String>,
}

impl GstBillInvoice {
    pub fn business_header_html(
        &self,
        profile: &crate::synthetic_read_model::BusinessProfile,
    ) -> String {
        use super::estimate::escape_text;
        let fssai = if profile.fssai.is_empty() {
            String::new()
        } else {
            format!("FSSAI License Number: {}<br>", escape_text(&profile.fssai))
        };
        format!(
            "<div>{fssai}GSTIN/UIN: {}<br>State Name : Uttar Pradesh, Code : 09</div>",
            escape_text(&profile.gstin)
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GstBillError {
    AccessDenied,
    MissingItems,
    MissingCustomer,
    MissingDate,
    InvalidGstin,
    InvalidEway,
    InvalidAmount,
    UnsupportedRounding,
    Overflow,
    NotFound,
    Cancelled,
    /// The ID has a deletion marker; a deleted bill cannot be saved again.
    Deleted,
}

impl GstBillError {
    pub const fn warning(self) -> Option<&'static str> {
        match self {
            Self::MissingItems => Some("Add at least one item with quantity"),
            Self::MissingCustomer => Some("Enter the customer name"),
            Self::MissingDate => Some("Pick the invoice date"),
            Self::InvalidGstin => Some("GSTIN must be exactly 15 characters"),
            Self::InvalidEway => Some("E-Way Bill number must be 12 digits"),
            Self::AccessDenied
            | Self::InvalidAmount
            | Self::UnsupportedRounding
            | Self::Overflow
            | Self::NotFound
            | Self::Cancelled
            | Self::Deleted => None,
        }
    }
}

fn owner(role: Role) -> Result<(), GstBillError> {
    match role {
        Role::Owner => Ok(()),
        Role::Manager => Err(GstBillError::AccessDenied),
    }
}

fn amount(rate: Money, quantity: u32) -> Result<i64, GstBillError> {
    let rate = rate.minor_units();
    if rate < 0 {
        return Err(GstBillError::InvalidAmount);
    }
    rate.checked_mul(i64::from(quantity))
        .ok_or(GstBillError::Overflow)
}

fn validate_references(gstin: Option<&str>, eway_number: Option<&str>) -> Result<(), GstBillError> {
    if gstin.is_some_and(|value| !value.is_empty() && value.trim().encode_utf16().count() != 15) {
        return Err(GstBillError::InvalidGstin);
    }
    if eway_number.is_some_and(|value| {
        !value.is_empty() && value.bytes().filter(u8::is_ascii_digit).count() != 12
    }) {
        return Err(GstBillError::InvalidEway);
    }
    Ok(())
}

impl ConfirmationFixture {
    /// `nextGstNo`: highest stored GST number, cancelled bills included, plus one.
    pub fn next_gst_bill_number(&self, role: Role) -> Result<GstBillNumber, GstBillError> {
        owner(role)?;
        self.gst_bills
            .values()
            .map(|bill| bill.number.0)
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .map(GstBillNumber)
            .ok_or(GstBillError::Overflow)
    }

    /// `newGstDraft`: numbered on creation, dated `today`, with one blank line.
    pub fn new_gst_bill_draft(
        &self,
        role: Role,
        id: GstBillId,
        today: BillDate,
    ) -> Result<GstBillDraft<'static>, GstBillError> {
        Ok(GstBillDraft {
            id,
            number: self.next_gst_bill_number(role)?,
            date: Some(today),
            cust_state: record::DEFAULT_STATE,
            cust_state_code: record::DEFAULT_STATE_CODE,
            pos_state: record::DEFAULT_STATE,
            pos_code: record::DEFAULT_STATE_CODE,
            pay_mode: record::DEFAULT_PAY_MODE,
            reverse_charge: record::DEFAULT_REVERSE_CHARGE,
            lines: &[GstBillLineInput::BLANK],
            ..GstBillDraft::default()
        })
    }

    /// `fillGstItem`: reads product names, pack sizes, prices and tax settings only.
    pub fn fill_gst_item(
        &self,
        role: Role,
        item: &str,
        product: Option<ProductId>,
    ) -> Result<GstItemFill, GstBillError> {
        owner(role)?;
        let product = product
            .filter(|id| self.catalog.has_product(*id))
            .or_else(|| self.catalog.product_id_by_exact_name(item));
        let (gst_basis_points, hsn) = self.catalog.gst_info(item);
        Ok(GstItemFill {
            product,
            packet_qty: product.map(|id| {
                self.checkout_products
                    .get(&id.value())
                    .map_or(1, |terms| terms.pack.get())
            }),
            sell_price: product.map(|id| self.catalog.sell_price(id)),
            hsn,
            gst_basis_points,
        })
    }

    /// Calculates the form's always-intrastate tax display without saving.
    pub fn gst_bill_form(
        &self,
        role: Role,
        draft: &GstBillDraft<'_>,
    ) -> Result<GstBillTotals, GstBillError> {
        owner(role)?;
        totals(
            draft.lines.iter().map(|line| {
                amount(line.inclusive_rate, line.quantity)
                    .map(|value| (value, line.gst_basis_points))
            }),
            GstJurisdiction::Intra,
        )
    }

    /// Saves or replaces a standalone synthetic record and emits its saved invoice.
    pub fn save_gst_bill(
        &mut self,
        role: Role,
        draft: &GstBillDraft<'_>,
        sink: &mut impl FnMut(&GstBillInvoice),
    ) -> Result<(), GstBillError> {
        self.save_gst_bill_to(role, draft, &mut store::Unpersisted, sink)
            .map_err(GstBillStoreError::into_bill)
    }

    /// Emits an existing synthetic record, including cancelled records.
    pub fn emit_gst_bill(
        &self,
        role: Role,
        id: GstBillId,
        sink: &mut impl FnMut(&GstBillInvoice),
    ) -> Result<(), GstBillError> {
        owner(role)?;
        let bill = self.gst_bills.get(&id).ok_or(GstBillError::NotFound)?;
        sink(&self.gst_invoice(bill)?);
        Ok(())
    }

    /// Marks the standalone record cancelled without removing it.
    pub fn cancel_gst_bill(&mut self, role: Role, id: GstBillId) -> Result<(), GstBillError> {
        self.cancel_gst_bill_to(role, id, &mut store::Unpersisted)
            .map_err(GstBillStoreError::into_bill)
    }

    /// `removeRec("gstBills", id)`: removes an active or cancelled bill and keeps a marker.
    pub fn delete_gst_bill(&mut self, role: Role, id: GstBillId) -> Result<(), GstBillError> {
        self.delete_gst_bill_to(role, id, &mut store::Unpersisted)
            .map_err(GstBillStoreError::into_bill)
    }

    /// Stored bills in ID order plus deletion markers, for backup and restart checks.
    pub fn gst_bill_ledger(&self, role: Role) -> Result<GstBillLedger, GstBillError> {
        owner(role)?;
        Ok(GstBillLedger {
            bills: self.gst_bills.values().cloned().collect(),
            deleted: self.gst_deleted.iter().copied().collect(),
        })
    }

    /// A copy with no GST bills or markers, for "no ordinary effects" checks in other crates.
    #[cfg(any(test, feature = "test-support"))]
    #[doc(hidden)]
    pub fn without_gst_bills(&self) -> Self {
        let mut ordinary = self.clone();
        ordinary.gst_bills.clear();
        ordinary.gst_deleted.clear();
        ordinary
    }

    fn gst_invoice(&self, bill: &GstBill) -> Result<GstBillInvoice, GstBillError> {
        bill.invoice_with(|item| self.catalog.gst_info(item).1)
    }
}
