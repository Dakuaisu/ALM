//! DEV-ONLY Postgres implementation of `SalesAuthority` over the migrated local schema, for the
//! conformance suite. Each call runs as the `authenticated` role with the caller's role in the
//! JWT claims (`auth.jwt()`), inside one long transaction that is rolled back when the
//! authority is dropped; a savepoint per call stands in for the function's own transaction.
//! Not a device transport.
#![allow(dead_code)]

use std::cell::RefCell;

use alm_core::projection::{
    OwnerValues, ProjectedCashDay, ProjectedCrate, ProjectedCustomer, ProjectedGodownRow,
    ProjectedGstBill, ProjectedProduct, ProjectedProductTerms, ProjectedPurchaseLine,
    ProjectedPurchaseOrder, ProjectedReceipt, ProjectedSale, ProjectedStock, ProjectedSupplier,
    ProjectionBatch, ProjectionSnapshot,
};
use alm_core::{BillDate, DraftCustomer, Money, Role};
use alm_sync::{AuthorityError, Intent, Outcome, RequestId, SaleReceipt, SalesAuthority};
use postgres::types::ToSql;
use postgres::{Client, NoTls};
use serde_json::{Value, json};

pub struct PgAuthority(RefCell<Client>);

fn claims(role: Role) -> String {
    let role = match role {
        Role::Owner => "owner",
        Role::Manager => "manager",
    };
    json!({"app_metadata": {"role": role}}).to_string()
}

fn money(value: &Value) -> Money {
    Money::from_minor_units(value.as_i64().unwrap_or(0))
}

fn date(value: &Value) -> Option<BillDate> {
    value.as_str().and_then(|text| text.parse().ok())
}

fn day(date: BillDate) -> Value {
    json!(date.to_string())
}

fn number(value: &Value) -> u64 {
    value
        .as_u64()
        .or_else(|| value.as_str().and_then(|t| t.parse().ok()))
        .unwrap_or(0)
}

fn rows(value: &Value) -> &[Value] {
    value.as_array().map_or(&[], Vec::as_slice)
}

/// The function and JSON terms for an intent.
fn call(intent: &Intent) -> (&'static str, Value) {
    match intent {
        Intent::PostSale(sale) => {
            let mut terms: Value = serde_json::from_str(&intent.supabase_terms().unwrap()).unwrap();
            if let DraftCustomer::Selected { id, name } = sale.draft.customer() {
                terms["customer_id"] = json!(id.value());
                terms["customer"] = json!(name);
            }
            terms["date"] = day(sale.date);
            terms["paid_minor"] = json!(sale.paid.minor_units());
            ("post_sale", terms)
        }
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

fn outcome(value: &Value) -> Result<Outcome, AuthorityError> {
    if value.get("invoice").is_some() {
        return Ok(Outcome::Sale(SaleReceipt {
            draft: number(&value["draft_id"]),
            invoice: number(&value["invoice"]),
            quantity: u32::try_from(number(&value["quantity"]))
                .map_err(|_| AuthorityError::Invalid)?,
            total: money(&value["total_minor"]),
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

fn refusal(error: &postgres::Error) -> AuthorityError {
    match error.code().map(postgres::error::SqlState::code) {
        Some("ALM01") => AuthorityError::Conflict,
        Some("ALM02") => AuthorityError::AlreadyPosted,
        Some("ALM03") => AuthorityError::StaleReview,
        Some("ALM06" | "42501") => AuthorityError::Denied,
        Some(code) => AuthorityError::Rejected(
            error
                .as_db_error()
                .map_or_else(|| code.to_owned(), |db| format!("{code} {}", db.message())),
        ),
        None => AuthorityError::Unavailable,
    }
}

impl PgAuthority {
    /// Connects to `ALM_PG_URL` (None when unset), opens the test transaction and seeds it with
    /// `seed`'s Owner projection so both authorities start from the same shop.
    pub fn connect(seed: &ProjectionSnapshot) -> Option<Self> {
        let url = match std::env::var("ALM_PG_URL") {
            Ok(url) => url,
            Err(std::env::VarError::NotPresent) => return None,
            Err(error) => panic!("invalid ALM_PG_URL: {error}"),
        };
        let mut client = Client::connect(&url, NoTls).expect("connect to test database");
        client.batch_execute("BEGIN").expect("begin");
        let owner = seed.owner.as_ref().expect("an owner seed");
        for p in &seed.products {
            let stock = seed
                .stock
                .iter()
                .find(|s| s.product == p.id)
                .map_or(0, |s| s.on_hand);
            let buy = owner
                .buy_rates
                .iter()
                .find(|(id, _)| *id == p.id)
                .map_or(0, |(_, b)| b.minor_units());
            client
                .execute(
                    "INSERT INTO public.products (id, item, brand, sell_minor, buy_minor, stock, reorder_level)
                     VALUES ($1, $2, $3, $4, $5, $6, $7)",
                    &[
                        &i64::try_from(p.id).unwrap(),
                        &p.item,
                        &p.brand,
                        &p.sell.minor_units(),
                        &buy,
                        &i32::try_from(stock).unwrap(),
                        &i32::try_from(p.reorder_level).unwrap(),
                    ],
                )
                .expect("seed product");
            client
                .execute(
                    "INSERT INTO public.product_lots (product_id, remaining) VALUES ($1, $2)",
                    &[
                        &i64::try_from(p.id).unwrap(),
                        &i32::try_from(stock).unwrap(),
                    ],
                )
                .expect("seed lot");
        }
        for c in &seed.customers {
            client
                .execute(
                    "INSERT INTO public.customers (id, name) VALUES ($1, $2)",
                    &[&c.id, &c.name],
                )
                .expect("seed customer");
        }
        for s in &seed.suppliers {
            client
                .execute(
                    "INSERT INTO public.suppliers (id, name) VALUES ($1, $2)",
                    &[&i64::try_from(s.id).unwrap(), &s.name],
                )
                .expect("seed supplier");
        }
        for (product, supplier, rate) in &owner.supplier_prices {
            client
                .execute(
                    "INSERT INTO public.supplier_prices (product_id, supplier_id, rate_minor) VALUES ($1, $2, $3)",
                    &[
                        &i64::try_from(*product).unwrap(),
                        &i64::try_from(*supplier).unwrap(),
                        &rate.minor_units(),
                    ],
                )
                .expect("seed quote");
        }
        Some(Self(RefCell::new(client)))
    }

    /// Runs `sql` as `role` through the `authenticated` client role.
    fn as_role(
        &self,
        role: Role,
        sql: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Value, AuthorityError> {
        let mut client = self.0.borrow_mut();
        client
            .batch_execute("SAVEPOINT call")
            .map_err(|_| AuthorityError::Unavailable)?;
        client
            .execute(
                "SELECT set_config('request.jwt.claims', $1, true)",
                &[&claims(role)],
            )
            .map_err(|_| AuthorityError::Unavailable)?;
        client
            .batch_execute("SET LOCAL ROLE authenticated")
            .map_err(|_| AuthorityError::Unavailable)?;
        let result = client.query_one(sql, params);
        match result {
            Ok(row) => {
                let value: Option<Value> = row.get(0);
                client
                    .batch_execute("RESET ROLE; RELEASE SAVEPOINT call")
                    .map_err(|_| AuthorityError::Unavailable)?;
                Ok(value.unwrap_or(Value::Null))
            }
            Err(error) => {
                client
                    .batch_execute("ROLLBACK TO SAVEPOINT call; RESET ROLE; RELEASE SAVEPOINT call")
                    .map_err(|_| AuthorityError::Unavailable)?;
                Err(refusal(&error))
            }
        }
    }
}

fn snapshot(data: &Value, role: Role) -> ProjectionSnapshot {
    let products = rows(&data["products"]);
    let mut s = ProjectionSnapshot {
        catalog: rows(&data["catalog"])
            .iter()
            .map(|r| ProjectedProduct {
                id: number(&r["id"]),
                name: r["name"].as_str().unwrap_or_default().to_owned(),
                rate: money(&r["rate"]),
            })
            .collect(),
        ..ProjectionSnapshot::default()
    };
    if role != Role::Owner {
        return s;
    }
    let text = |v: &Value| v.as_str().unwrap_or_default().to_owned();
    s.products = products
        .iter()
        .map(|r| ProjectedProductTerms {
            id: number(&r["id"]),
            item: text(&r["item"]),
            brand: text(&r["brand"]),
            sell: money(&r["sell"]),
            reorder_level: u32::try_from(number(&r["reorder_level"])).unwrap_or(0),
        })
        .collect();
    s.stock = products
        .iter()
        .map(|r| ProjectedStock {
            product: number(&r["id"]),
            item: text(&r["item"]),
            brand: text(&r["brand"]),
            on_hand: u32::try_from(number(&r["on_hand"])).unwrap_or(0),
            reorder_level: u32::try_from(number(&r["reorder_level"])).unwrap_or(0),
        })
        .collect();
    s.customers = rows(&data["customers"])
        .iter()
        .map(|r| ProjectedCustomer {
            id: text(&r["id"]),
            name: text(&r["name"]),
            balance: money(&r["balance"]),
        })
        .collect();
    s.sales = rows(&data["sales"])
        .iter()
        .map(|r| ProjectedSale {
            draft_id: number(&r["draft_id"]),
            invoice: number(&r["invoice"]),
            date: date(&r["date"]).unwrap_or(BillDate::new(2000, 1, 1).unwrap()),
            customer: text(&r["customer"]),
            items: Vec::new(),
            lines: u32::try_from(number(&r["lines"])).unwrap_or(0),
            packets: u32::try_from(number(&r["packets"])).unwrap_or(0),
            total: money(&r["total"]),
            paid: money(&r["paid"]),
            online: false,
            cancelled: r["cancelled"].as_bool().unwrap_or(false),
            balance: Money::from_minor_units(0),
        })
        .collect();
    s.receipts = rows(&data["receipts"])
        .iter()
        .map(|r| ProjectedReceipt {
            slot: number(&r["slot"]),
            date: date(&r["date"]).unwrap_or(BillDate::new(2000, 1, 1).unwrap()),
            customer: text(&r["customer"]),
            amount: money(&r["amount"]),
            online: r["online"].as_bool().unwrap_or(false),
        })
        .collect();
    s.cash_days = rows(&data["cash_days"])
        .iter()
        .map(|r| ProjectedCashDay {
            id: number(&r["id"]),
            date: date(&r["date"]).unwrap_or(BillDate::new(2000, 1, 1).unwrap()),
            amounts: alm_core::confirmation::manual_cashbook::CashBookAmounts {
                cash: money(&r["cash"]),
                paytm: money(&r["paytm"]),
                phonepe: money(&r["phonepe"]),
                goods: money(&r["goods"]),
                misc: money(&r["misc"]),
                salary: money(&r["salary"]),
                debt: money(&r["debt"]),
            },
        })
        .collect();
    s.suppliers = rows(&data["suppliers"])
        .iter()
        .map(|r| ProjectedSupplier {
            id: number(&r["id"]),
            name: text(&r["name"]),
        })
        .collect();
    s.crates = rows(&data["crates"])
        .iter()
        .map(|r| ProjectedCrate {
            id: number(&r["id"]),
            name: text(&r["name"]),
            price: money(&r["price"]),
            components: u32::try_from(number(&r["components"])).unwrap_or(0),
        })
        .collect();
    s.godown = rows(&data["godown"])
        .iter()
        .map(|r| ProjectedGodownRow {
            item: text(&r["item"]),
            cases: u32::try_from(number(&r["cases"])).unwrap_or(0),
            note: text(&r["note"]),
        })
        .collect();
    s.gst_bills = rows(&data["gst_bills"])
        .iter()
        .map(|r| ProjectedGstBill {
            id: number(&r["id"]),
            number: number(&r["number"]),
            date: date(&r["date"]).unwrap_or(BillDate::new(2000, 1, 1).unwrap()),
            customer: text(&r["customer"]),
            cancelled: r["cancelled"].as_bool().unwrap_or(false),
        })
        .collect();
    let owner = &data["owner"];
    s.owner = Some(OwnerValues {
        sale_costs: rows(&owner["sale_costs"])
            .iter()
            .map(|r| {
                (
                    number(&r[0]),
                    money(&r[1]),
                    r[2].as_i64().map(Money::from_minor_units),
                )
            })
            .collect(),
        stock_value: owner["stock_value"].as_i64().map(Money::from_minor_units),
        buy_rates: rows(&owner["buy_rates"])
            .iter()
            .map(|r| (number(&r[0]), money(&r[1])))
            .collect(),
        supplier_prices: rows(&owner["supplier_prices"])
            .iter()
            .map(|r| (number(&r[0]), number(&r[1]), money(&r[2])))
            .collect(),
        purchase_orders: rows(&owner["purchase_orders"])
            .iter()
            .map(|o| {
                let lines: Vec<ProjectedPurchaseLine> = rows(&o["lines"])
                    .iter()
                    .map(|l| ProjectedPurchaseLine {
                        item: text(&l["item"]),
                        product: l["product"].as_u64(),
                        ordered: u32::try_from(number(&l["ordered"])).unwrap_or(0),
                        received: u32::try_from(number(&l["received"])).unwrap_or(0),
                        remaining: u32::try_from(number(&l["remaining"])).unwrap_or(0),
                        rate: money(&l["rate"]),
                    })
                    .collect();
                let status = if lines.iter().all(|l| l.remaining == 0) {
                    "Completed"
                } else if lines.iter().any(|l| l.received > 0) {
                    "PartiallyReceived"
                } else {
                    "Pending"
                };
                ProjectedPurchaseOrder {
                    id: number(&o["id"]),
                    supplier: text(&o["supplier"]),
                    date: date(&o["date"]).unwrap_or(BillDate::new(2000, 1, 1).unwrap()),
                    status: status.into(),
                    lines,
                }
            })
            .collect(),
        state: Vec::new(),
    });
    s
}

impl SalesAuthority for PgAuthority {
    fn submit(
        &mut self,
        role: Role,
        request: &RequestId,
        intent: &Intent,
    ) -> Result<Outcome, AuthorityError> {
        let (function, terms) = call(intent);
        let value = self.as_role(
            role,
            &format!("SELECT public.{function}($1, $2)"),
            &[&request.as_str(), &terms],
        )?;
        outcome(&value)
    }

    fn status(&self, role: Role, request: &RequestId) -> Result<Option<Outcome>, AuthorityError> {
        let value = self.as_role(
            role,
            "SELECT public.request_status($1)",
            &[&request.as_str()],
        )?;
        if value.is_null() {
            return Ok(None);
        }
        outcome(&value["outcome"]).map(Some)
    }

    fn allocate_gst_number(&mut self, role: Role, bill_id: &str) -> Result<u64, AuthorityError> {
        let value = self.as_role(
            role,
            "SELECT to_jsonb(public.allocate_gst_bill_number($1))",
            &[&bill_id],
        )?;
        value.as_u64().ok_or(AuthorityError::Invalid)
    }

    fn pull(&self, role: Role, checkpoint: Option<u64>) -> Result<ProjectionBatch, AuthorityError> {
        let since = checkpoint.map(|c| i64::try_from(c).unwrap_or(i64::MAX));
        let mut value = self.as_role(role, "SELECT public.pull_projection($1)", &[&since])?;
        if !value["rows"].is_null() && value["full"] != json!(true) {
            // Something changed: this adapter applies full replacements, so read everything.
            value = self.as_role(role, "SELECT public.pull_projection($1)", &[&None::<i64>])?;
        }
        let snapshot = (!value["rows"].is_null()).then(|| snapshot(&value["rows"], role));
        Ok(ProjectionBatch {
            role,
            checkpoint: number(&value["checkpoint"]),
            as_of_ms: 0,
            snapshot,
        })
    }
}
