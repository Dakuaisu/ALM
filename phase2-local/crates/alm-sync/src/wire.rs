//! The hosted authority's wire format: the SQL function and JSON terms for each intent, the
//! outcome and error each call returns, and the per-screen projection rows of
//! `pull_projection`. Every authority transport (dev Postgres, REST) and the local simulation
//! read and write projections through this one format, so the device code is the same for all.
use alm_core::{DraftCustomer, Money, Role};
use serde_json::{Value, json};

use crate::{AuthorityError, Intent, Outcome, SaleReceipt};

mod rows;
pub use rows::{decode_batch, encode_batch};

fn day(date: alm_core::BillDate) -> Value {
    json!(date.to_string())
}

fn number(value: &Value) -> u64 {
    value
        .as_u64()
        .or_else(|| value.as_str().and_then(|t| t.parse().ok()))
        .unwrap_or(0)
}

/// The SQL function and its JSON terms for an intent.
pub fn command(intent: &Intent) -> (&'static str, Value) {
    match intent {
        Intent::PostSale(sale) => {
            let lines: Vec<Value> = sale
                .draft
                .lines()
                .iter()
                .zip(&sale.prices)
                .map(|(line, price)| {
                    json!({"product_id": line.product_id().value(), "quantity": line.quantity(),
                        "unit_price_minor": price.minor_units()})
                })
                .collect();
            let mut terms =
                json!({"draft_id": sale.draft.id().value().to_string(), "lines": lines});
            let load = sale.draft.loading_rate().minor_units();
            if load != 0 {
                terms["loading_minor"] = json!(load);
            }
            match sale.draft.customer() {
                DraftCustomer::Selected { id, name } => {
                    terms["customer_id"] = json!(id.value());
                    terms["customer"] = json!(name);
                }
                DraftCustomer::Typed(name) => terms["customer"] = json!(name),
                DraftCustomer::Unassigned => {}
            }
            terms["date"] = day(sale.date);
            terms["paid_minor"] = json!(sale.paid.minor_units());
            ("post_sale", terms)
        }
        Intent::SellCrate(c) => (
            "sell_crate",
            json!({"draft_id": c.draft.to_string(), "template": c.template,
                "quantity": c.quantity, "customer_id": c.customer.0, "customer": c.customer.1,
                "date": day(c.date), "paid_minor": c.paid.minor_units(),
                "rate_minor": c.rate.minor_units(),
                "components": c.components.iter().map(|(p, q, piece)|
                    json!({"product_id": p, "quantity": q, "piece": piece}))
                    .collect::<Vec<_>>()}),
        ),
        Intent::CancelSale { draft } => (
            "set_sale_cancelled",
            json!({"draft_id": draft.to_string(), "cancel": true}),
        ),
        Intent::RestoreSale { draft } => (
            "set_sale_cancelled",
            json!({"draft_id": draft.to_string(), "cancel": false}),
        ),
        Intent::EditSale { draft, lines } => (
            "edit_sale",
            json!({"draft_id": draft.to_string(), "lines": lines.iter()
                .map(|(q, r)| json!({"quantity": q, "rate_minor": r})).collect::<Vec<_>>()}),
        ),
        Intent::RecordReceipt {
            customer,
            date,
            amount,
            online,
        } => (
            "record_receipt",
            json!({"customer_id": customer, "date": day(*date),
                "amount_minor": amount.minor_units(), "online": online}),
        ),
        Intent::SaveCashDay { id, date, amounts } => (
            "save_cash_day",
            json!({"id": id, "date": day(*date), "amounts": {
                "cash": amounts.cash.minor_units(), "paytm": amounts.paytm.minor_units(),
                "phonepe": amounts.phonepe.minor_units(), "goods": amounts.goods.minor_units(),
                "misc": amounts.misc.minor_units(), "salary": amounts.salary.minor_units(),
                "debt": amounts.debt.minor_units()}}),
        ),
        Intent::DeleteCashDay { id } => ("save_cash_day", json!({"id": id, "delete": true})),
        Intent::SaveProduct(p) => (
            "save_product",
            json!({"id": p.id, "new": p.new, "item": p.item, "brand": p.brand,
                "buy_minor": p.buy.minor_units(), "sell_minor": p.sell.minor_units(),
                "reorder": p.reorder, "reviewed": p.reviewed.map(|(b, s)|
                    json!({"buy_minor": b.minor_units(), "sell_minor": s.minor_units()}))}),
        ),
        Intent::DeleteProduct { id } => ("delete_product", json!({"id": id})),
        Intent::AddSupplier { name } => ("add_supplier", json!({"name": name})),
        Intent::SetSupplierPrices { product, rates } => (
            "set_supplier_prices",
            json!({"product_id": product, "rates": rates.iter().map(|(s, r)|
                json!({"supplier_id": s, "rate_minor": r.map(Money::minor_units)}))
                .collect::<Vec<_>>()}),
        ),
        Intent::SavePurchaseOrder(po) => (
            "save_purchase_order",
            json!({"supplier": po.supplier.as_ref().map(|(id, name)| json!({"id": id, "name": name})),
                "date": po.date.map(day), "note": po.note,
                "lines": po.lines.iter().map(|l| json!({"item": l.item, "product_id": l.product,
                    "ordered": l.ordered, "rate_minor": l.rate.minor_units()}))
                    .collect::<Vec<_>>()}),
        ),
        Intent::ReceivePurchaseOrder {
            po,
            date,
            today,
            lines,
            remaining,
        } => (
            "receive_purchase_order",
            json!({"po": po, "date": date.map(day), "today": day(*today),
                "lines": lines.iter().map(|(q, r)| json!({"quantity": q,
                    "rate_minor": r.minor_units()})).collect::<Vec<_>>(),
                "remaining": remaining}),
        ),
        Intent::ClosePurchaseOrder { po, remaining } => (
            "close_purchase_order",
            json!({"po": po, "remaining": remaining}),
        ),
        Intent::SaveCrate(c) => (
            "save_crate",
            json!({"id": c.id, "name": c.name, "price_minor": c.price.minor_units(),
                "components": c.components.iter().map(|(p, q, piece)|
                    json!({"product_id": p, "quantity": q, "piece": piece}))
                    .collect::<Vec<_>>()}),
        ),
        Intent::DeleteCrate { id } => ("delete_crate", json!({"id": id})),
        Intent::SetReorderDefault { level } => ("set_reorder_default", json!({"level": level})),
        Intent::SaveOpening {
            customer,
            amount,
            date,
        } => (
            "save_opening",
            json!({"customer_id": customer, "amount_minor": amount.map(Money::minor_units),
                "date": date.map(day)}),
        ),
        Intent::SaveGodown { item, cases, note } => (
            "save_godown",
            json!({"item": item, "cases": cases, "note": note}),
        ),
        Intent::CountStock {
            product,
            counted,
            fridge,
            date,
            reason,
            note,
            on_hand,
        } => (
            "count_stock",
            json!({"product_id": product, "counted": counted, "fridge": fridge,
                "date": day(*date), "reason": reason, "note": note, "on_hand": on_hand}),
        ),
        Intent::SaveGstBill { bill } => (
            "save_gst_bill",
            json!({"bill": serde_json::from_str::<Value>(bill).unwrap_or(Value::Null)}),
        ),
        Intent::CancelGstBill { id } => ("set_gst_bill_state", json!({"id": id, "delete": false})),
        Intent::DeleteGstBill { id } => ("set_gst_bill_state", json!({"id": id, "delete": true})),
        Intent::ImportGstBill { bill, deleted } => (
            "import_gst_bill",
            json!({"bill": bill.as_deref().map(|b| serde_json::from_str::<Value>(b)
                .unwrap_or(Value::Null)), "deleted": deleted}),
        ),
    }
}

/// Reads a command's JSON outcome.
///
/// # Errors
/// `Invalid` for a shape no command returns.
pub fn outcome(value: &Value) -> Result<Outcome, AuthorityError> {
    if value.get("invoice").is_some() {
        return Ok(Outcome::Sale(SaleReceipt {
            draft: number(&value["draft_id"]),
            invoice: number(&value["invoice"]),
            quantity: u32::try_from(number(&value["quantity"]))
                .map_err(|_| AuthorityError::Invalid)?,
            total: Money::from_minor_units(value["total_minor"].as_i64().unwrap_or(0)),
        }));
    }
    if let Some(id) = value.get("created") {
        return Ok(Outcome::Created(number(id)));
    }
    if let Some(n) = value.get("gst_number") {
        return Ok(Outcome::GstNumber(number(n)));
    }
    if value.get("done").is_some() {
        return Ok(Outcome::Done);
    }
    Err(AuthorityError::Invalid)
}

/// Reads `request_status`: null for no committed outcome.
///
/// # Errors
/// As [`outcome`].
pub fn status(value: &Value) -> Result<Option<Outcome>, AuthorityError> {
    if value.is_null() {
        return Ok(None);
    }
    outcome(&value["outcome"]).map(Some)
}

/// Reads `request_lookup`: `{"state": "committed" | "absent" | "conflict", "outcome"}`.
///
/// # Errors
/// `Invalid` for another shape.
pub fn lookup(value: &Value) -> Result<crate::Lookup, AuthorityError> {
    match value["state"].as_str() {
        Some("committed") => outcome(&value["outcome"]).map(crate::Lookup::Committed),
        Some("absent") => Ok(crate::Lookup::NotCommitted),
        Some("conflict") => Ok(crate::Lookup::OtherIntent),
        _ => Err(AuthorityError::Invalid),
    }
}

/// The typed error for a SQLSTATE raised by a command (`ALM01` … `ALM08`, `42501`).
pub fn refusal(code: &str, message: &str) -> AuthorityError {
    match code {
        "ALM01" => AuthorityError::Conflict,
        "ALM02" => AuthorityError::AlreadyPosted,
        "ALM03" => AuthorityError::StaleReview,
        "ALM06" | "42501" => AuthorityError::Denied,
        _ => AuthorityError::Rejected(format!("{code} {message}")),
    }
}

/// The role claim the hosted functions read from the JWT `app_metadata`.
pub const fn role_claim(role: Role) -> &'static str {
    match role {
        Role::Owner => "owner",
        Role::Manager => "manager",
    }
}
