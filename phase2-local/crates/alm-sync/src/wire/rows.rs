//! The per-screen projection rows of `pull_projection`, both ways.
use alm_core::confirmation::manual_cashbook::CashBookAmounts;
use alm_core::projection::{
    ProjectedCashDay, ProjectedCrate, ProjectedCustomer, ProjectedGodownRow, ProjectedGstBill,
    ProjectedProduct, ProjectedProductTerms, ProjectedReceipt, ProjectedSale, ProjectedStock,
    ProjectedSupplier, ProjectionBatch, ProjectionSnapshot,
};
use alm_core::{BillDate, Money, Role};
use serde_json::Value;

use super::{number, role_claim};

use crate::AuthorityError;

mod encode;
mod owner;
pub use encode::encode_batch;
use owner::decode_owner;

fn list(value: &Value) -> &[Value] {
    value.as_array().map_or(&[], Vec::as_slice)
}

fn money(value: &Value) -> Money {
    Money::from_minor_units(value.as_i64().unwrap_or(0))
}

fn small(value: &Value) -> Result<u32, AuthorityError> {
    u32::try_from(number(value)).map_err(|_| AuthorityError::Invalid)
}

fn text(value: &Value) -> String {
    value.as_str().unwrap_or_default().to_owned()
}

fn date(value: &Value) -> Result<BillDate, AuthorityError> {
    value
        .as_str()
        .and_then(|t| t.parse().ok())
        .ok_or(AuthorityError::Invalid)
}

#[allow(clippy::too_many_lines)]
/// One snapshot's rows in the wire format, for `role`.
///
/// # Errors
/// `Invalid` for rows no authority sends.
pub fn decode_rows(d: &Value, role: Role) -> Result<ProjectionSnapshot, AuthorityError> {
    let mut s = ProjectionSnapshot {
        sales_unsupported: match &d["sales_unsupported"] {
            Value::Null => None,
            id => Some(alm_core::DraftId::from_json(id).ok_or(AuthorityError::Invalid)?),
        },
        catalog: list(&d["catalog"])
            .iter()
            .map(|r| ProjectedProduct {
                id: number(&r["id"]),
                name: text(&r["name"]),
                rate: money(&r["rate"]),
            })
            .collect(),
        ..ProjectionSnapshot::default()
    };
    if role != Role::Owner {
        // A Manager file must never hold more than the catalog, whatever was sent.
        return Ok(s);
    }
    s.products = list(&d["products"])
        .iter()
        .map(|r| {
            Ok(ProjectedProductTerms {
                id: number(&r["id"]),
                item: text(&r["item"]),
                brand: text(&r["brand"]),
                sell: money(&r["sell"]),
                reorder_level: small(&r["reorder_level"])?,
            })
        })
        .collect::<Result<_, AuthorityError>>()?;
    s.stock = list(&d["stock"])
        .iter()
        .map(|r| {
            Ok(ProjectedStock {
                product: number(&r["product"]),
                item: text(&r["item"]),
                brand: text(&r["brand"]),
                on_hand: small(&r["on_hand"])?,
                reorder_level: small(&r["reorder_level"])?,
            })
        })
        .collect::<Result<_, AuthorityError>>()?;
    s.customers = list(&d["customers"])
        .iter()
        .map(|r| ProjectedCustomer {
            id: text(&r["id"]),
            name: text(&r["name"]),
            balance: money(&r["balance"]),
            mobile: text(&r["mobile"]),
            addr1: text(&r["addr1"]),
            addr2: text(&r["addr2"]),
        })
        .collect();
    s.sales = list(&d["sales"])
        .iter()
        .map(|r| {
            Ok(ProjectedSale {
                draft_id: alm_core::DraftId::from_json(&r["draft_id"])
                    .ok_or(AuthorityError::Invalid)?,
                invoice: number(&r["invoice"]),
                date: date(&r["date"])?,
                customer: text(&r["customer"]),
                items: list(&r["items"]).iter().map(text).collect(),
                lines: small(&r["lines"])?,
                packets: small(&r["packets"])?,
                total: money(&r["total"]),
                paid: money(&r["paid"]),
                online: r["online"].as_bool().unwrap_or(false),
                cancelled: r["cancelled"].as_bool().unwrap_or(false),
                balance: money(&r["balance"]),
            })
        })
        .collect::<Result<_, AuthorityError>>()?;
    s.receipts = list(&d["receipts"])
        .iter()
        .map(|r| {
            Ok(ProjectedReceipt {
                slot: number(&r["slot"]),
                date: date(&r["date"])?,
                customer: text(&r["customer"]),
                amount: money(&r["amount"]),
                online: r["online"].as_bool().unwrap_or(false),
            })
        })
        .collect::<Result<_, AuthorityError>>()?;
    s.cash_days = list(&d["cash_days"])
        .iter()
        .map(|r| {
            Ok(ProjectedCashDay {
                id: number(&r["id"]),
                date: date(&r["date"])?,
                amounts: CashBookAmounts {
                    cash: money(&r["cash"]),
                    paytm: money(&r["paytm"]),
                    phonepe: money(&r["phonepe"]),
                    goods: money(&r["goods"]),
                    misc: money(&r["misc"]),
                    salary: money(&r["salary"]),
                    debt: money(&r["debt"]),
                },
            })
        })
        .collect::<Result<_, AuthorityError>>()?;
    s.suppliers = list(&d["suppliers"])
        .iter()
        .map(|r| ProjectedSupplier {
            id: number(&r["id"]),
            name: text(&r["name"]),
        })
        .collect();
    s.crates = list(&d["crates"])
        .iter()
        .map(|r| {
            Ok(ProjectedCrate {
                id: number(&r["id"]),
                name: text(&r["name"]),
                price: money(&r["price"]),
                components: small(&r["components"])?,
            })
        })
        .collect::<Result<_, AuthorityError>>()?;
    s.godown = list(&d["godown"])
        .iter()
        .map(|r| {
            Ok(ProjectedGodownRow {
                item: text(&r["item"]),
                cases: small(&r["cases"])?,
                note: text(&r["note"]),
            })
        })
        .collect::<Result<_, AuthorityError>>()?;
    s.gst_bills = list(&d["gst_bills"])
        .iter()
        .map(|r| {
            Ok(ProjectedGstBill {
                id: number(&r["id"]),
                number: number(&r["number"]),
                date: date(&r["date"])?,
                customer: text(&r["customer"]),
                cancelled: r["cancelled"].as_bool().unwrap_or(false),
            })
        })
        .collect::<Result<_, AuthorityError>>()?;
    if d["owner"].is_object() {
        s.owner = Some(decode_owner(&d["owner"])?);
    }
    Ok(s)
}

/// Reads a `pull_projection` result asked for as `requested`. The authority answers with the
/// role it grants now: an Owner asking may get a Manager batch (a downgrade), never the reverse.
///
/// # Errors
/// `Invalid` for a role above the one asked for or malformed rows.
pub fn decode_batch(value: &Value, requested: Role) -> Result<ProjectionBatch, AuthorityError> {
    let role = match value["role"].as_str() {
        Some(text) if text == role_claim(requested) => requested,
        Some(text) if text == role_claim(Role::Manager) => Role::Manager,
        _ => return Err(AuthorityError::Invalid),
    };
    // Changes since the device's checkpoint: it rebuilds the snapshot from what it holds.
    if value["delta"].is_object() {
        return Ok(ProjectionBatch {
            role,
            checkpoint: number(&value["checkpoint"]),
            as_of_ms: number(&value["as_of_ms"]),
            snapshot: None,
            rows: Some(alm_core::projection::PulledRows::Changes(
                value["delta"].clone(),
            )),
        });
    }
    let (snapshot, rows) = match &value["rows"] {
        Value::Null => (None, None),
        rows => (
            Some(decode_rows(rows, role)?),
            Some(alm_core::projection::PulledRows::Full {
                rows: rows.clone(),
                hashes: value["row_hashes"].clone(),
            }),
        ),
    };
    Ok(ProjectionBatch {
        role,
        checkpoint: number(&value["checkpoint"]),
        as_of_ms: number(&value["as_of_ms"]),
        snapshot,
        rows,
    })
}
