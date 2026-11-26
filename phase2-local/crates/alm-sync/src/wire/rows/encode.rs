//! Projection rows in the wire format, as `pull_projection` sends them.
use alm_core::Money;
use alm_core::projection::{ProjectionBatch, ProjectionSnapshot};
use serde_json::{Value, json};

use super::super::role_claim;

fn m(value: Money) -> i64 {
    value.minor_units()
}

/// The rows of one snapshot in the wire format.
#[allow(clippy::too_many_lines)]
fn encode_rows(s: &ProjectionSnapshot) -> Value {
    let mut rows = json!({
        "sales_unsupported": s.sales_unsupported.map(alm_core::DraftId::to_json),
        "catalog": s.catalog.iter().map(|r| json!({"id": r.id, "name": r.name, "rate": m(r.rate)}))
            .collect::<Vec<_>>(),
        "products": s.products.iter().map(|r| json!({"id": r.id, "item": r.item,
            "brand": r.brand, "sell": m(r.sell), "reorder_level": r.reorder_level}))
            .collect::<Vec<_>>(),
        "stock": s.stock.iter().map(|r| json!({"product": r.product, "item": r.item,
            "brand": r.brand, "on_hand": r.on_hand, "reorder_level": r.reorder_level}))
            .collect::<Vec<_>>(),
        "customers": s.customers.iter().map(|r| json!({"id": r.id, "name": r.name,
            "balance": m(r.balance), "mobile": r.mobile, "addr1": r.addr1, "addr2": r.addr2}))
            .collect::<Vec<_>>(),
        "sales": s.sales.iter().map(|r| json!({"draft_id": r.draft_id.to_json(), "invoice": r.invoice,
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
            "sale_costs": o.sale_costs.iter().map(|(d, r, c)| json!([d.to_json(), m(*r), c.map(m)]))
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
        rows["owner"]["book"] = alm_core::projection::book::to_json(&o.book);
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
