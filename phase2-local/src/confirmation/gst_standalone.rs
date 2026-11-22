//! Standalone synthetic GST bills; independent of ordinary sales and numbering.
mod record;

use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};

use super::gst_standalone_totals::totals;
use super::{ConfirmationFixture, GstJurisdiction};
use crate::synthetic_read_model::{date, money};
use crate::{BillDate, Money, Role};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct GstBillId(pub u64);

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GstBillNumber(pub u64);

#[derive(Clone, Copy, Debug)]
pub struct GstBillLineInput<'a> {
    pub item: &'a str,
    pub quantity: u32,
    pub inclusive_rate: Money,
    pub gst_basis_points: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct GstBillDraft<'a> {
    pub id: GstBillId,
    pub number: GstBillNumber,
    pub customer: &'a str,
    pub date: Option<BillDate>,
    pub gstin: Option<&'a str>,
    pub eway_number: Option<&'a str>,
    pub jurisdiction: GstJurisdiction,
    pub lines: &'a [GstBillLineInput<'a>],
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GstBillLine {
    pub item: String,
    pub quantity: NonZeroU32,
    #[serde(with = "money")]
    pub inclusive_rate: Money,
    pub gst_basis_points: u32,
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
#[serde(deny_unknown_fields)]
pub struct GstBill {
    pub id: GstBillId,
    pub number: GstBillNumber,
    pub customer: String,
    #[serde(with = "date")]
    pub date: BillDate,
    pub gstin: Option<String>,
    pub eway_number: Option<String>,
    pub jurisdiction: GstJurisdiction,
    pub cancelled: bool,
    pub lines: Vec<GstBillLine>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GstBillInvoice {
    pub record: GstBill,
    pub totals: GstBillTotals,
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
            | Self::Cancelled => None,
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
        owner(role)?;
        if !draft
            .lines
            .iter()
            .any(|line| !line.item.is_empty() && line.quantity != 0)
        {
            return Err(GstBillError::MissingItems);
        }
        if draft.customer.is_empty() {
            return Err(GstBillError::MissingCustomer);
        }
        let date = draft.date.ok_or(GstBillError::MissingDate)?;
        validate_references(draft.gstin, draft.eway_number)?;
        if self
            .gst_bills
            .get(&draft.id)
            .is_some_and(|bill| bill.cancelled)
        {
            return Err(GstBillError::Cancelled);
        }
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
                    quantity,
                    inclusive_rate: line.inclusive_rate,
                    gst_basis_points: line.gst_basis_points,
                    amount: Money::from_minor_units(value),
                }))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let bill = GstBill {
            id: draft.id,
            number: draft.number,
            customer: draft.customer.to_owned(),
            date,
            gstin: draft.gstin.map(str::to_owned),
            eway_number: draft.eway_number.map(str::to_owned),
            jurisdiction: draft.jurisdiction,
            cancelled: false,
            lines,
        };
        let projection = bill.invoice()?;
        self.gst_bills.insert(draft.id, bill);
        sink(&projection);
        Ok(())
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
        sink(&bill.invoice()?);
        Ok(())
    }

    /// Marks the standalone record cancelled without removing it.
    pub fn cancel_gst_bill(&mut self, role: Role, id: GstBillId) -> Result<(), GstBillError> {
        owner(role)?;
        self.gst_bills
            .get_mut(&id)
            .ok_or(GstBillError::NotFound)?
            .cancelled = true;
        Ok(())
    }
}
