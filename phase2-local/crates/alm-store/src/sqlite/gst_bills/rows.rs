//! Reads stored GST bill and line rows.
use std::num::NonZeroU32;

use rusqlite::Row;

use alm_core::confirmation::{GstBill, GstBillId, GstBillLine, GstUnit};
use alm_core::{BillDate, Money, ProductId};

use crate::DraftStorageError;

use super::super::{integer, text};

pub(super) fn unit_text(unit: GstUnit) -> &'static str {
    match unit {
        GstUnit::Case => "Case",
        GstUnit::Piece => "Piece",
    }
}

fn required(row: &Row<'_>, index: usize) -> Result<String, DraftStorageError> {
    text(row, index)?.ok_or(DraftStorageError::InvalidOutput)
}

pub(super) fn decimal(row: &Row<'_>, index: usize) -> Result<u64, DraftStorageError> {
    required(row, index)?
        .parse()
        .map_err(|_| DraftStorageError::InvalidOutput)
}

fn small(row: &Row<'_>, index: usize) -> Result<u32, DraftStorageError> {
    integer(row, index)?
        .and_then(|value| u32::try_from(value).ok())
        .ok_or(DraftStorageError::InvalidOutput)
}

fn money_at(row: &Row<'_>, index: usize) -> Result<Money, DraftStorageError> {
    integer(row, index)?
        .map(Money::from_minor_units)
        .ok_or(DraftStorageError::InvalidOutput)
}

pub(super) fn read_bill(row: &Row<'_>) -> Result<GstBill, DraftStorageError> {
    let date: BillDate = required(row, 2)?
        .parse()
        .map_err(|_| DraftStorageError::InvalidOutput)?;
    Ok(GstBill {
        id: GstBillId(decimal(row, 0)?),
        number: alm_core::confirmation::GstBillNumber(decimal(row, 1)?),
        date,
        customer_id: text(row, 3)?,
        customer: required(row, 4)?,
        gstin: text(row, 5)?,
        cust_addr: required(row, 6)?,
        cust_phone: required(row, 7)?,
        cust_email: required(row, 8)?,
        cust_state: required(row, 9)?,
        cust_state_code: required(row, 10)?,
        pos_state: required(row, 11)?,
        pos_code: required(row, 12)?,
        pay_mode: required(row, 13)?,
        pay_terms: required(row, 14)?,
        reverse_charge: required(row, 15)?,
        transporter: required(row, 16)?,
        vehicle_no: required(row, 17)?,
        lr_no: required(row, 18)?,
        eway_number: text(row, 19)?,
        terms_delivery: required(row, 20)?,
        dispatch: required(row, 21)?,
        cancelled: match integer(row, 22)? {
            Some(0) => false,
            Some(1) => true,
            _ => return Err(DraftStorageError::InvalidOutput),
        },
        created: decimal(row, 23)?,
        lines: Vec::new(),
    })
}

pub(super) fn read_line(row: &Row<'_>) -> Result<(u64, i64, GstBillLine), DraftStorageError> {
    let nonzero =
        |index| NonZeroU32::new(small(row, index)?).ok_or(DraftStorageError::InvalidOutput);
    let product = text(row, 3)?
        .map(|id| id.parse().map(ProductId::new))
        .transpose()
        .map_err(|_| DraftStorageError::InvalidOutput)?;
    let unit = match required(row, 5)?.as_str() {
        "Case" => GstUnit::Case,
        "Piece" => GstUnit::Piece,
        _ => return Err(DraftStorageError::InvalidOutput),
    };
    Ok((
        decimal(row, 0)?,
        integer(row, 1)?.ok_or(DraftStorageError::InvalidOutput)?,
        GstBillLine {
            item: required(row, 2)?,
            product,
            hsn: required(row, 4)?,
            unit,
            quantity: nonzero(6)?,
            inclusive_rate: money_at(row, 7)?,
            gst_basis_points: small(row, 8)?,
            packet_qty: nonzero(9)?,
            amount: money_at(row, 10)?,
        },
    ))
}
