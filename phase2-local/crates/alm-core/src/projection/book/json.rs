//! The book rows in the authority's wire format (`owner.book` in `pull_projection`), both
//! ways; the device stores the same JSON per top-level section.
use super::{
    ActivityRow, BookAllocation, BookCashDay, BookCheckout, BookCrate, BookCrateRow, BookCrates,
    BookCustomer, BookCustomerRef, BookGodownRow, BookLot, BookOrder, BookOrderLine,
    BookOrderReceipt, BookPayment, BookProduct, BookProductDetails, BookPurchasing, BookReceipt,
    BookSale, BookSaleLine, BookSettings, OwnerBook,
};
use crate::confirmation::{GstBill, StockCountMovement, StockCountReason};
use crate::{BillDate, Money};
use serde_json::{Value, json};

/// Malformed book rows.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BookFormatError;

type R<T> = Result<T, BookFormatError>;

const REASONS: [(StockCountReason, &str); 6] = [
    (StockCountReason::CountCorrection, "count"),
    (StockCountReason::DamageOrLeakage, "damage"),
    (StockCountReason::Expired, "expired"),
    (StockCountReason::TheftOrUnknown, "theft"),
    (StockCountReason::OwnUseOrSample, "own-use"),
    (StockCountReason::Other, "other"),
];

/// The wire code of a stock count reason.
pub fn reason_code(reason: StockCountReason) -> &'static str {
    REASONS
        .iter()
        .find(|(r, _)| *r == reason)
        .map_or("other", |(_, code)| code)
}

/// Reads [`reason_code`] back.
pub fn reason_from_code(code: &str) -> Option<StockCountReason> {
    REASONS.iter().find(|(_, c)| *c == code).map(|(r, _)| *r)
}

fn number(value: &Value) -> u64 {
    value
        .as_u64()
        .or_else(|| value.as_str().and_then(|t| t.parse().ok()))
        .unwrap_or(0)
}

fn list(value: &Value) -> &[Value] {
    value.as_array().map_or(&[], Vec::as_slice)
}

fn money(value: &Value) -> Money {
    Money::from_minor_units(value.as_i64().unwrap_or(0))
}

fn small(value: &Value) -> R<u32> {
    u32::try_from(number(value)).map_err(|_| BookFormatError)
}

fn text(value: &Value) -> String {
    value.as_str().unwrap_or_default().to_owned()
}

fn date(value: &Value) -> R<BillDate> {
    value
        .as_str()
        .and_then(|t| t.parse().ok())
        .ok_or(BookFormatError)
}

fn m(value: Money) -> i64 {
    value.minor_units()
}

fn om(value: Option<Money>) -> Value {
    value.map_or(Value::Null, |v| json!(v.minor_units()))
}

fn od(value: Option<BillDate>) -> Value {
    value.map_or(Value::Null, |d| json!(d.to_string()))
}

fn opt_money(value: &Value) -> Option<Money> {
    value.as_i64().map(Money::from_minor_units)
}

fn opt_date(value: &Value) -> R<Option<BillDate>> {
    if value.is_null() {
        Ok(None)
    } else {
        date(value).map(Some)
    }
}

fn opt_text(value: &Value) -> Option<String> {
    value.as_str().map(str::to_owned)
}

fn opt_small(value: &Value) -> R<Option<u32>> {
    if value.is_null() {
        Ok(None)
    } else {
        small(value).map(Some)
    }
}

fn flag(value: &Value) -> bool {
    value.as_bool().unwrap_or(false)
}

fn rows<T>(value: &Value, f: impl Fn(&Value) -> R<T>) -> R<Vec<T>> {
    list(value).iter().map(f).collect()
}

fn encode_product(p: &BookProduct) -> Value {
    json!({
        "id": p.id,
        "details": p.details.as_ref().map(|d| json!({"item": d.item, "brand": d.brand,
            "reorder_level": d.reorder_level, "gst_rate": d.gst_rate, "cess": d.cess,
            "hsn": d.hsn})),
        "buy": m(p.buy), "margin": m(p.margin), "sell": m(p.sell), "stock": p.stock,
        "seeded_opening": p.seeded_opening, "fridge": p.fridge,
        "lots": p.lots.iter().map(|l| json!([od(l.date), l.remaining, om(l.unit_cost)]))
            .collect::<Vec<_>>(),
        "counts": p.counts.iter().map(|c| json!({"date": c.date.to_string(), "from": c.from,
            "to": c.to, "delta": c.delta, "reason": c.reason.map(reason_code),
            "note": c.note})).collect::<Vec<_>>(),
    })
}

fn decode_product(r: &Value) -> R<BookProduct> {
    let d = &r["details"];
    Ok(BookProduct {
        id: number(&r["id"]),
        details: if d.is_object() {
            Some(BookProductDetails {
                item: text(&d["item"]),
                brand: text(&d["brand"]),
                reorder_level: small(&d["reorder_level"])?,
                gst_rate: opt_small(&d["gst_rate"])?,
                cess: opt_small(&d["cess"])?,
                hsn: opt_text(&d["hsn"]),
            })
        } else {
            None
        },
        buy: money(&r["buy"]),
        margin: money(&r["margin"]),
        sell: money(&r["sell"]),
        stock: small(&r["stock"])?,
        seeded_opening: small(&r["seeded_opening"])?,
        fridge: small(&r["fridge"])?,
        lots: rows(&r["lots"], |l| {
            Ok(BookLot {
                date: opt_date(&l[0])?,
                remaining: small(&l[1])?,
                unit_cost: opt_money(&l[2]),
            })
        })?,
        counts: rows(&r["counts"], |c| {
            Ok(StockCountMovement {
                date: date(&c["date"])?,
                from: small(&c["from"])?,
                to: small(&c["to"])?,
                delta: c["delta"].as_i64().ok_or(BookFormatError)?,
                reason: match c["reason"].as_str() {
                    None => None,
                    Some(code) => Some(reason_from_code(code).ok_or(BookFormatError)?),
                },
                note: opt_text(&c["note"]),
            })
        })?,
    })
}

fn encode_sale(s: &BookSale) -> Value {
    let customer = match &s.customer {
        BookCustomerRef::Unassigned => json!({"kind": "unassigned"}),
        BookCustomerRef::Typed(name) => json!({"kind": "typed", "name": name}),
        BookCustomerRef::Selected { id, name } => {
            json!({"kind": "selected", "id": id, "name": name})
        }
    };
    let payment = match s.payment {
        BookPayment::Legacy => json!({"kind": "legacy"}),
        BookPayment::Named(a) => json!({"kind": "named", "amount": m(a)}),
        BookPayment::WalkInCash(a) => json!({"kind": "walk_in", "amount": m(a)}),
    };
    json!({
        "draft": s.draft, "invoice": s.invoice, "customer": customer, "date": od(s.date),
        "loading": m(s.loading_rate),
        "lines": s.lines.iter().map(|l| json!([l.product, l.quantity])).collect::<Vec<_>>(),
        "prices": s.prices.iter().map(|p| m(*p)).collect::<Vec<_>>(),
        "crate_rows": s.crate_rows.iter().map(|c| json!({"quantity": c.quantity,
            "components": c.components.iter().map(|(p, q, piece)| json!([p, q, piece]))
                .collect::<Vec<_>>(),
            "name": c.price.as_ref().map(|p| &p.0), "rate": c.price.as_ref().map(|p| m(p.1))}))
            .collect::<Vec<_>>(),
        "checkout": s.checkout.as_ref().map(|c| json!({"loading": m(c.loading_rate),
            "lines": c.lines.iter().map(|(piece, pack, pp, cp)| json!([piece, pack, m(*pp), m(*cp)]))
                .collect::<Vec<_>>()})),
        "crate_packs": s.crate_packs,
        "payment": payment, "online": s.online, "quantity": s.quantity, "total": m(s.total),
        "received": om(s.received), "due": om(s.due),
        "customer_balance": om(s.customer_balance), "edits": s.edits,
        "cancelled": s.cancelled, "epoch": s.epoch, "creation_key": s.creation_key,
        "items": s.items.iter().map(|(p, i)| json!([p, i])).collect::<Vec<_>>(),
        "cost": om(s.cost),
        "line_units": s.line_units.iter().map(|(l, u)| json!([l, om(*u)])).collect::<Vec<_>>(),
        "allocations": s.allocations.iter().map(|a| json!([a.line, a.product, a.lot,
            a.quantity, om(a.cost)])).collect::<Vec<_>>(),
    })
}

fn decode_crate_row(c: &Value) -> R<BookCrateRow> {
    Ok(BookCrateRow {
        quantity: small(&c["quantity"])?,
        components: rows(&c["components"], |x| {
            Ok((number(&x[0]), small(&x[1])?, flag(&x[2])))
        })?,
        price: match (c["name"].as_str(), opt_money(&c["rate"])) {
            (Some(name), Some(rate)) => Some((name.to_owned(), rate)),
            _ => None,
        },
    })
}

fn decode_sale(r: &Value) -> R<BookSale> {
    let c = &r["customer"];
    let customer = match c["kind"].as_str() {
        Some("unassigned") => BookCustomerRef::Unassigned,
        Some("typed") => BookCustomerRef::Typed(text(&c["name"])),
        Some("selected") => BookCustomerRef::Selected {
            id: text(&c["id"]),
            name: text(&c["name"]),
        },
        _ => return Err(BookFormatError),
    };
    let p = &r["payment"];
    let payment = match p["kind"].as_str() {
        Some("legacy") => BookPayment::Legacy,
        Some("named") => BookPayment::Named(money(&p["amount"])),
        Some("walk_in") => BookPayment::WalkInCash(money(&p["amount"])),
        _ => return Err(BookFormatError),
    };
    Ok(BookSale {
        draft: number(&r["draft"]),
        invoice: number(&r["invoice"]),
        customer,
        date: opt_date(&r["date"])?,
        loading_rate: money(&r["loading"]),
        lines: rows(&r["lines"], |l| {
            Ok(BookSaleLine {
                product: number(&l[0]),
                quantity: small(&l[1])?,
            })
        })?,
        prices: list(&r["prices"]).iter().map(money).collect(),
        // Books written before mixed bills hold one optional `crate_row`.
        crate_rows: if r["crate_rows"].is_array() {
            rows(&r["crate_rows"], decode_crate_row)?
        } else if r["crate_row"].is_object() {
            vec![decode_crate_row(&r["crate_row"])?]
        } else {
            Vec::new()
        },
        checkout: if r["checkout"].is_object() {
            let c = &r["checkout"];
            Some(BookCheckout {
                lines: rows(&c["lines"], |x| {
                    Ok((flag(&x[0]), small(&x[1])?, money(&x[2]), money(&x[3])))
                })?,
                loading_rate: money(&c["loading"]),
            })
        } else {
            None
        },
        crate_packs: if r["crate_packs"].is_array() {
            Some(rows(&r["crate_packs"], small)?)
        } else {
            None
        },
        payment,
        online: flag(&r["online"]),
        quantity: small(&r["quantity"])?,
        total: money(&r["total"]),
        received: opt_money(&r["received"]),
        due: opt_money(&r["due"]),
        customer_balance: opt_money(&r["customer_balance"]),
        edits: number(&r["edits"]),
        cancelled: flag(&r["cancelled"]),
        epoch: number(&r["epoch"]),
        creation_key: r["creation_key"].as_u64(),
        items: rows(&r["items"], |i| Ok((number(&i[0]), opt_text(&i[1]))))?,
        cost: opt_money(&r["cost"]),
        line_units: rows(&r["line_units"], |u| Ok((number(&u[0]), opt_money(&u[1]))))?,
        allocations: rows(&r["allocations"], |a| {
            Ok(BookAllocation {
                line: number(&a[0]),
                product: number(&a[1]),
                lot: number(&a[2]),
                quantity: small(&a[3])?,
                cost: opt_money(&a[4]),
            })
        })?,
    })
}

fn encode_purchasing(p: &BookPurchasing) -> Value {
    json!({
        "products": p.products.iter().map(|(id, item, buy)| json!([id, item, m(*buy)]))
            .collect::<Vec<_>>(),
        "suppliers": p.suppliers.iter().map(|(id, name)| json!([id, name])).collect::<Vec<_>>(),
        "price_suppliers": p.price_suppliers,
        "costs": p.costs.iter().map(|(id, avg)| json!([id, m(*avg)])).collect::<Vec<_>>(),
        "quotes": p.quotes.iter().map(|(id, q)| json!([id, q.iter()
            .map(|(name, rate)| json!([name, m(*rate)])).collect::<Vec<_>>()]))
            .collect::<Vec<_>>(),
        "orders": p.orders.iter().map(|o| json!({"id": o.id, "supplier": o.supplier,
            "supplier_name": o.supplier_name, "date": o.date.to_string(), "note": o.note,
            "cancelled": o.cancelled, "lines": o.lines.iter().map(|l| json!({"item": l.item,
                "product": l.product, "ordered": l.ordered, "received": l.received,
                "closed": l.closed, "rate": m(l.rate), "receipts": l.receipts.iter()
                    .map(|r| json!([r.date.to_string(), r.quantity, m(r.rate)]))
                    .collect::<Vec<_>>()})).collect::<Vec<_>>()}))
            .collect::<Vec<_>>(),
    })
}

fn decode_purchasing(p: &Value) -> R<BookPurchasing> {
    Ok(BookPurchasing {
        products: rows(&p["products"], |r| {
            Ok((number(&r[0]), text(&r[1]), money(&r[2])))
        })?,
        suppliers: rows(&p["suppliers"], |r| Ok((number(&r[0]), text(&r[1]))))?,
        price_suppliers: list(&p["price_suppliers"]).iter().map(number).collect(),
        costs: rows(&p["costs"], |r| Ok((number(&r[0]), money(&r[1]))))?,
        quotes: rows(&p["quotes"], |r| {
            Ok((
                number(&r[0]),
                rows(&r[1], |q| Ok((text(&q[0]), money(&q[1]))))?,
            ))
        })?,
        orders: rows(&p["orders"], |o| {
            Ok(BookOrder {
                id: number(&o["id"]),
                supplier: number(&o["supplier"]),
                supplier_name: text(&o["supplier_name"]),
                date: date(&o["date"])?,
                note: text(&o["note"]),
                cancelled: flag(&o["cancelled"]),
                lines: rows(&o["lines"], |l| {
                    Ok(BookOrderLine {
                        item: text(&l["item"]),
                        product: l["product"].as_u64(),
                        ordered: small(&l["ordered"])?,
                        received: small(&l["received"])?,
                        closed: small(&l["closed"])?,
                        rate: money(&l["rate"]),
                        receipts: rows(&l["receipts"], |r| {
                            Ok(BookOrderReceipt {
                                date: date(&r[0])?,
                                quantity: small(&r[1])?,
                                rate: money(&r[2]),
                            })
                        })?,
                    })
                })?,
            })
        })?,
    })
}

/// The book in the wire format.
pub fn to_json(b: &OwnerBook) -> Value {
    let s = &b.settings;
    json!({
        "settings": {"default_product": s.default_product, "reorder_default": s.reorder_default,
            "loading": m(s.loading_rate), "crate_rate": om(s.crate_rate),
            "next_invoice": s.next_invoice, "movement": s.movement, "ledger": m(s.ledger),
            "received_ledger": m(s.received_ledger), "next_cash": s.next_cash},
        "products": b.products.iter().map(encode_product).collect::<Vec<_>>(),
        "checkout_terms": b.checkout_terms.iter().map(|(p, pack, piece, case)|
            json!([p, pack, m(*piece), m(*case)])).collect::<Vec<_>>(),
        "sales": b.sales.iter().map(encode_sale).collect::<Vec<_>>(),
        "receipts": b.receipts.iter().map(|r| json!({"customer": r.customer,
            "amount": m(r.amount), "date": r.date.to_string(), "after_invoice": r.after_invoice,
            "online": r.online, "selected_name": r.selected.as_ref().map(|s| &s.0),
            "creation_key": r.selected.as_ref().map(|s| s.1)})).collect::<Vec<_>>(),
        "customers": b.customers.iter().map(|c| json!({"id": c.id, "registered": c.registered,
            "balance": om(c.balance), "has_opening": c.opening.is_some(),
            "opening": c.opening.map(|o| m(o.0)), "opening_date": od(c.opening.and_then(|o| o.1))}))
            .collect::<Vec<_>>(),
        "purchasing": encode_purchasing(&b.purchasing),
        "crates": {"last_id": b.crates.last_id, "templates": b.crates.templates.iter()
            .map(|t| json!({"id": t.id, "name": t.name, "price": m(t.price),
                "components": t.components.iter().map(|(p, q, piece)| json!([p, q, piece]))
                    .collect::<Vec<_>>()})).collect::<Vec<_>>(),
            "piece_mrps": b.crates.piece_mrps.iter().map(|(p, mrp)| json!([p, m(*mrp)]))
                .collect::<Vec<_>>()},
        "godown": b.godown.iter().map(|g| json!({"item": g.item, "cases": g.cases,
            "note": g.note})).collect::<Vec<_>>(),
        "gst_bills": b.gst_bills.iter()
            .map(|bill| bill.to_record_text().ok()
                .and_then(|t| serde_json::from_str::<Value>(&t).ok()).unwrap_or(Value::Null))
            .collect::<Vec<_>>(),
        "gst_deleted": b.gst_deleted,
        "activity": b.activity.iter().map(|a| json!({"at_ms": a.at_ms, "device": a.device,
            "account": a.account, "kind": a.kind, "subject": a.subject,
            "amount": om(a.amount), "detail": a.detail})).collect::<Vec<_>>(),
        "legacy": serde_json::to_value(&b.legacy).unwrap_or(Value::Null),
        "cash_days": b.cash_days.iter().map(|r| {
            let a = r.amounts;
            json!({"id": r.id, "date": r.date.to_string(), "cash": m(a.cash),
                "paytm": m(a.paytm), "phonepe": m(a.phonepe), "goods": m(a.goods),
                "misc": m(a.misc), "salary": m(a.salary), "debt": m(a.debt)})
        }).collect::<Vec<_>>(),
    })
}

/// Reads `owner.book`.
///
/// # Errors
/// [`BookFormatError`] for malformed rows.
pub fn from_json(b: &Value) -> R<OwnerBook> {
    let s = &b["settings"];
    let c = &b["crates"];
    Ok(OwnerBook {
        settings: BookSettings {
            default_product: number(&s["default_product"]),
            reorder_default: small(&s["reorder_default"])?,
            loading_rate: money(&s["loading"]),
            crate_rate: opt_money(&s["crate_rate"]),
            next_invoice: number(&s["next_invoice"]),
            movement: number(&s["movement"]),
            ledger: money(&s["ledger"]),
            received_ledger: money(&s["received_ledger"]),
            next_cash: number(&s["next_cash"]),
        },
        products: rows(&b["products"], decode_product)?,
        checkout_terms: rows(&b["checkout_terms"], |r| {
            Ok((number(&r[0]), small(&r[1])?, money(&r[2]), money(&r[3])))
        })?,
        sales: rows(&b["sales"], decode_sale)?,
        receipts: rows(&b["receipts"], |r| {
            Ok(BookReceipt {
                customer: text(&r["customer"]),
                amount: money(&r["amount"]),
                date: date(&r["date"])?,
                after_invoice: number(&r["after_invoice"]),
                online: r["online"].as_bool(),
                selected: match (r["selected_name"].as_str(), r["creation_key"].as_u64()) {
                    (Some(name), Some(key)) => Some((name.to_owned(), key)),
                    _ => None,
                },
            })
        })?,
        customers: rows(&b["customers"], |r| {
            Ok(BookCustomer {
                id: text(&r["id"]),
                registered: flag(&r["registered"]),
                balance: opt_money(&r["balance"]),
                opening: if flag(&r["has_opening"]) {
                    Some((money(&r["opening"]), opt_date(&r["opening_date"])?))
                } else {
                    None
                },
            })
        })?,
        purchasing: decode_purchasing(&b["purchasing"])?,
        crates: BookCrates {
            last_id: number(&c["last_id"]),
            templates: rows(&c["templates"], |t| {
                Ok(BookCrate {
                    id: number(&t["id"]),
                    name: text(&t["name"]),
                    price: money(&t["price"]),
                    components: rows(&t["components"], |r| {
                        Ok((number(&r[0]), small(&r[1])?, flag(&r[2])))
                    })?,
                })
            })?,
            piece_mrps: rows(&c["piece_mrps"], |r| Ok((number(&r[0]), money(&r[1]))))?,
        },
        godown: rows(&b["godown"], |g| {
            Ok(BookGodownRow {
                item: text(&g["item"]),
                cases: small(&g["cases"])?,
                note: opt_text(&g["note"]),
            })
        })?,
        gst_bills: rows(&b["gst_bills"], |g| {
            GstBill::from_record_text(&g.to_string()).map_err(|_| BookFormatError)
        })?,
        gst_deleted: list(&b["gst_deleted"]).iter().map(number).collect(),
        activity: rows(&b["activity"], |a| {
            Ok(ActivityRow {
                at_ms: number(&a["at_ms"]),
                device: text(&a["device"]),
                account: text(&a["account"]),
                kind: text(&a["kind"]),
                subject: text(&a["subject"]),
                amount: opt_money(&a["amount"]),
                detail: text(&a["detail"]),
            })
        })?,
        legacy: if b["legacy"].is_null() {
            crate::legacy::import::LegacyImports::default()
        } else {
            serde_json::from_value(b["legacy"].clone()).map_err(|_| BookFormatError)?
        },
        cash_days: rows(&b["cash_days"], |r| {
            Ok(BookCashDay {
                id: number(&r["id"]),
                date: date(&r["date"])?,
                amounts: crate::confirmation::manual_cashbook::CashBookAmounts {
                    cash: money(&r["cash"]),
                    paytm: money(&r["paytm"]),
                    phonepe: money(&r["phonepe"]),
                    goods: money(&r["goods"]),
                    misc: money(&r["misc"]),
                    salary: money(&r["salary"]),
                    debt: money(&r["debt"]),
                },
            })
        })?,
    })
}

/// The book as `(section, JSON body)` rows, as the device stores it.
pub fn to_sections(book: &OwnerBook) -> Vec<(String, String)> {
    match to_json(book) {
        Value::Object(map) => map.into_iter().map(|(k, v)| (k, v.to_string())).collect(),
        _ => Vec::new(),
    }
}

/// Reads [`to_sections`] back; no sections is the empty book.
///
/// # Errors
/// [`BookFormatError`] for malformed sections.
pub fn from_sections(sections: &[(String, String)]) -> R<OwnerBook> {
    if sections.is_empty() {
        return Ok(OwnerBook::default());
    }
    let mut map = serde_json::Map::new();
    for (name, body) in sections {
        map.insert(
            name.clone(),
            serde_json::from_str(body).map_err(|_| BookFormatError)?,
        );
    }
    from_json(&Value::Object(map))
}
