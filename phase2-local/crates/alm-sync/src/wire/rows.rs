//! The per-screen projection rows of `pull_projection`, both ways.
use alm_core::confirmation::manual_cashbook::CashBookAmounts;
use alm_core::projection::{
    OwnerValues, ProjectedCashDay, ProjectedCrate, ProjectedCustomer, ProjectedGodownRow,
    ProjectedGstBill, ProjectedProduct, ProjectedProductTerms, ProjectedPurchaseLine,
    ProjectedPurchaseOrder, ProjectedReceipt, ProjectedSale, ProjectedStock, ProjectedSupplier,
    ProjectionBatch, ProjectionSnapshot,
};
use alm_core::{BillDate, Money, Role};
use serde_json::{Value, json};

use super::{number, role_claim};
use crate::AuthorityError;

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

fn m(value: Money) -> i64 {
    value.minor_units()
}

/// The rows of one snapshot in the wire format.
#[allow(clippy::too_many_lines)]
fn encode_rows(s: &ProjectionSnapshot) -> Value {
    let mut rows = json!({
        "sales_unsupported": s.sales_unsupported,
        "catalog": s.catalog.iter().map(|r| json!({"id": r.id, "name": r.name, "rate": m(r.rate)}))
            .collect::<Vec<_>>(),
        "products": s.products.iter().map(|r| json!({"id": r.id, "item": r.item,
            "brand": r.brand, "sell": m(r.sell), "reorder_level": r.reorder_level}))
            .collect::<Vec<_>>(),
        "stock": s.stock.iter().map(|r| json!({"product": r.product, "item": r.item,
            "brand": r.brand, "on_hand": r.on_hand, "reorder_level": r.reorder_level}))
            .collect::<Vec<_>>(),
        "customers": s.customers.iter().map(|r| json!({"id": r.id, "name": r.name,
            "balance": m(r.balance)})).collect::<Vec<_>>(),
        "sales": s.sales.iter().map(|r| json!({"draft_id": r.draft_id, "invoice": r.invoice,
            "date": r.date.to_string(), "customer": r.customer, "items": r.items,
            "lines": r.lines, "packets": r.packets, "total": m(r.total), "paid": m(r.paid),
            "online": r.online, "cancelled": r.cancelled, "balance": m(r.balance)}))
            .collect::<Vec<_>>(),
        "receipts": s.receipts.iter().map(|r| json!({"slot": r.slot,
            "date": r.date.to_string(), "customer": r.customer, "amount": m(r.amount),
            "online": r.online})).collect::<Vec<_>>(),
        "cash_days": s.cash_days.iter().map(|r| {
            let a = r.amounts;
            json!({"id": r.id, "date": r.date.to_string(), "cash": m(a.cash),
                "paytm": m(a.paytm), "phonepe": m(a.phonepe), "goods": m(a.goods),
                "misc": m(a.misc), "salary": m(a.salary), "debt": m(a.debt)})
        }).collect::<Vec<_>>(),
        "suppliers": s.suppliers.iter().map(|r| json!({"id": r.id, "name": r.name}))
            .collect::<Vec<_>>(),
        "crates": s.crates.iter().map(|r| json!({"id": r.id, "name": r.name,
            "price": m(r.price), "components": r.components})).collect::<Vec<_>>(),
        "godown": s.godown.iter().map(|r| json!({"item": r.item, "cases": r.cases,
            "note": r.note})).collect::<Vec<_>>(),
        "gst_bills": s.gst_bills.iter().map(|r| json!({"id": r.id, "number": r.number,
            "date": r.date.to_string(), "customer": r.customer, "cancelled": r.cancelled}))
            .collect::<Vec<_>>(),
    });
    if let Some(o) = &s.owner {
        rows["owner"] = json!({
            "sale_costs": o.sale_costs.iter().map(|(d, r, c)| json!([d, m(*r), c.map(m)]))
                .collect::<Vec<_>>(),
            "stock_value": o.stock_value.map(m),
            "buy_rates": o.buy_rates.iter().map(|(p, b)| json!([p, m(*b)])).collect::<Vec<_>>(),
            "supplier_prices": o.supplier_prices.iter().map(|(p, s, r)| json!([p, s, m(*r)]))
                .collect::<Vec<_>>(),
            "purchase_orders": o.purchase_orders.iter().map(|po| json!({"id": po.id,
                "supplier": po.supplier, "date": po.date.to_string(), "status": po.status,
                "lines": po.lines.iter().map(|l| json!({"item": l.item, "product": l.product,
                    "ordered": l.ordered, "received": l.received, "remaining": l.remaining,
                    "rate": m(l.rate)})).collect::<Vec<_>>()})).collect::<Vec<_>>(),
        });
        if !o.state.is_empty() {
            rows["owner"]["dev_state"] = json!(o.state);
        }
    }
    rows
}

/// A batch as `pull_projection` returns it (always a full replacement when rows changed).
pub fn encode_batch(batch: &ProjectionBatch) -> Value {
    json!({
        "role": role_claim(batch.role),
        "checkpoint": batch.checkpoint,
        "as_of_ms": batch.as_of_ms,
        "full": batch.snapshot.is_some(),
        "rows": batch.snapshot.as_ref().map(encode_rows),
        "removed": [],
    })
}

#[allow(clippy::too_many_lines)]
fn decode_rows(d: &Value, role: Role) -> Result<ProjectionSnapshot, AuthorityError> {
    let mut s = ProjectionSnapshot {
        sales_unsupported: d["sales_unsupported"].as_u64(),
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
        })
        .collect();
    s.sales = list(&d["sales"])
        .iter()
        .map(|r| {
            Ok(ProjectedSale {
                draft_id: number(&r["draft_id"]),
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

fn decode_owner(o: &Value) -> Result<OwnerValues, AuthorityError> {
    Ok(OwnerValues {
        sale_costs: list(&o["sale_costs"])
            .iter()
            .map(|r| {
                (
                    number(&r[0]),
                    money(&r[1]),
                    r[2].as_i64().map(Money::from_minor_units),
                )
            })
            .collect(),
        stock_value: o["stock_value"].as_i64().map(Money::from_minor_units),
        buy_rates: list(&o["buy_rates"])
            .iter()
            .map(|r| (number(&r[0]), money(&r[1])))
            .collect(),
        supplier_prices: list(&o["supplier_prices"])
            .iter()
            .map(|r| (number(&r[0]), number(&r[1]), money(&r[2])))
            .collect(),
        purchase_orders: list(&o["purchase_orders"])
            .iter()
            .map(|po| {
                Ok(ProjectedPurchaseOrder {
                    id: number(&po["id"]),
                    supplier: text(&po["supplier"]),
                    date: date(&po["date"])?,
                    status: text(&po["status"]),
                    lines: list(&po["lines"])
                        .iter()
                        .map(|l| {
                            Ok(ProjectedPurchaseLine {
                                item: text(&l["item"]),
                                product: l["product"].as_u64(),
                                ordered: small(&l["ordered"])?,
                                received: small(&l["received"])?,
                                remaining: small(&l["remaining"])?,
                                rate: money(&l["rate"]),
                            })
                        })
                        .collect::<Result<_, AuthorityError>>()?,
                })
            })
            .collect::<Result<_, AuthorityError>>()?,
        state: list(&o["dev_state"])
            .iter()
            .map(|r| (text(&r[0]), text(&r[1])))
            .collect(),
    })
}

/// Reads a `pull_projection` result for `role`.
///
/// # Errors
/// `Invalid` for a batch of another role or malformed rows.
pub fn decode_batch(value: &Value, role: Role) -> Result<ProjectionBatch, AuthorityError> {
    if value["role"].as_str() != Some(role_claim(role)) {
        return Err(AuthorityError::Invalid);
    }
    let snapshot = match &value["rows"] {
        Value::Null => None,
        rows => Some(decode_rows(rows, role)?),
    };
    Ok(ProjectionBatch {
        role,
        checkpoint: number(&value["checkpoint"]),
        as_of_ms: number(&value["as_of_ms"]),
        snapshot,
    })
}
