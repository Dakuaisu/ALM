//! DEV-ONLY Postgres implementation of `SalesAuthority` over the migrated local schema, for the
//! conformance and differential suites. Each call runs as the `authenticated` role with the
//! caller's role in the JWT claims (`auth.jwt()`), inside one long transaction that is rolled
//! back when the authority is dropped; a savepoint per call stands in for the function's own
//! transaction. The JSON mapping is `alm_sync::wire`, shared with the REST transport.
#![allow(dead_code)]

use std::cell::RefCell;

use alm_core::Role;
use alm_core::projection::ProjectionBatch;
use alm_sync::{
    AuthorityError, Intent, LocalAuthority, Lookup, Outcome, RequestId, SalesAuthority, wire,
};
use postgres::types::ToSql;
use postgres::{Client, NoTls};
use serde_json::{Value, json};

pub struct PgAuthority(RefCell<Client>);

/// One active FIFO layer as `(received on, remaining, unit cost)`.
pub type Lot = (Option<String>, u32, Option<i64>);

fn refusal(error: &postgres::Error) -> AuthorityError {
    match error.code().map(postgres::error::SqlState::code) {
        Some(code) => wire::refusal(
            code,
            error
                .as_db_error()
                .map_or("", postgres::error::DbError::message),
        ),
        None => AuthorityError::Unavailable,
    }
}

fn sql_id(value: u64) -> i64 {
    i64::try_from(value).unwrap()
}

/// Inserts `seed`'s shop from its Owner book rows: settings, products with every FIFO layer,
/// purchasing registration and quotes, customers, suppliers and crates' piece MRPs.
#[allow(clippy::too_many_lines)]
pub fn seed_shop(client: &mut Client, seed: &LocalAuthority) {
    let snapshot = seed.pull(Role::Owner, None).unwrap().snapshot.unwrap();
    let book = &snapshot.owner.as_ref().expect("an owner seed").book;
    assert!(
        book.sales.is_empty() && book.receipts.is_empty(),
        "seed has no bills"
    );
    let s = &book.settings;
    client
        .execute(
            "UPDATE public.authority_state SET default_product = $1, reorder_default = $2,
                 loading_minor = $3, crate_rate_minor = $4, next_cash = $5,
                 last_crate = $6, movement = $7 WHERE id = 1",
            &[
                &sql_id(s.default_product),
                &i32::try_from(s.reorder_default).unwrap(),
                &s.loading_rate.minor_units(),
                &s.crate_rate.map(alm_core::Money::minor_units),
                &sql_id(s.next_cash),
                &sql_id(book.crates.last_id),
                &sql_id(s.movement),
            ],
        )
        .expect("seed settings");
    for p in &book.products {
        let d = p.details.as_ref().expect("seed products are named");
        let provisional = snapshot
            .catalog
            .iter()
            .find(|c| c.id == p.id)
            .map(|c| c.rate.minor_units());
        let purchase = book.purchasing.products.iter().find(|(id, ..)| *id == p.id);
        let cost = book.purchasing.costs.iter().position(|(id, _)| *id == p.id);
        let average = cost.map(|i| book.purchasing.costs[i].1.minor_units());
        let cost_seq = cost.map(|i| i64::try_from(i).unwrap() - 1_000);
        let small = |v: u32| i32::try_from(v).unwrap();
        client
            .execute(
                "INSERT INTO public.products (id, item, brand, sell_minor, buy_minor, stock,
                     reorder_level, provisional_minor, fridge, seeded_opening, gst_rate, cess, hsn,
                     purchase_item, purchase_buy_minor, purchase_avg_minor, purchase_cost_seq)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)",
                &[
                    &sql_id(p.id),
                    &d.item,
                    &d.brand,
                    &p.sell.minor_units(),
                    &p.buy.minor_units(),
                    &small(p.stock),
                    &small(d.reorder_level),
                    &provisional,
                    &small(p.fridge),
                    &small(p.seeded_opening),
                    &d.gst_rate.map(small),
                    &d.cess.map(small),
                    &d.hsn,
                    &purchase.map(|(_, item, _)| item.clone()),
                    &purchase.map(|(_, _, buy)| buy.minor_units()),
                    &average,
                    &cost_seq,
                ],
            )
            .expect("seed product");
        for lot in &p.lots {
            client
                .execute(
                    "INSERT INTO public.product_lots (product_id, received_on, remaining, unit_cost_minor)
                     VALUES ($1, $2::text::date, $3, $4)",
                    &[
                        &sql_id(p.id),
                        &lot.date.map(|d| d.to_string()),
                        &small(lot.remaining),
                        &lot.unit_cost.map(alm_core::Money::minor_units),
                    ],
                )
                .expect("seed lot");
        }
    }
    for c in &book.customers {
        let row = snapshot.customers.iter().find(|r| r.id == c.id);
        let field = |f: fn(&alm_core::projection::ProjectedCustomer) -> &String| {
            row.map_or_else(String::new, |r| f(r).clone())
        };
        let (name, mobile, addr1, addr2) = (
            field(|r| &r.name),
            field(|r| &r.mobile),
            field(|r| &r.addr1),
            field(|r| &r.addr2),
        );
        let (opening, date) = c.opening.map_or((0, None), |(a, d)| (a.minor_units(), d));
        client
            .execute(
                "INSERT INTO public.customers (id, name, registered, has_balance, has_opening,
                     opening_minor, opening_date, mobile, addr1, addr2)
                 VALUES ($1, $2, $3, $4, $5, $6, $7::text::date, $8, $9, $10)",
                &[
                    &c.id,
                    &name,
                    &c.registered,
                    &c.balance.is_some(),
                    &c.opening.is_some(),
                    &opening,
                    &date.map(|d| d.to_string()),
                    &mobile,
                    &addr1,
                    &addr2,
                ],
            )
            .expect("seed customer");
    }
    for (id, name) in &book.purchasing.suppliers {
        client
            .execute(
                "INSERT INTO public.suppliers (id, name, price_column) VALUES ($1, $2, $3)",
                &[
                    &sql_id(*id),
                    name,
                    &book.purchasing.price_suppliers.contains(id),
                ],
            )
            .expect("seed supplier");
    }
    for (product, quotes) in &book.purchasing.quotes {
        for (name, rate) in quotes {
            client
                .execute(
                    "INSERT INTO public.supplier_prices (product_id, supplier_id, rate_minor)
                     SELECT $1, id, $3 FROM public.suppliers WHERE name = $2",
                    &[&sql_id(*product), name, &rate.minor_units()],
                )
                .expect("seed quote");
        }
    }
    for (product, pack, piece, case) in &book.checkout_terms {
        client
            .execute(
                "INSERT INTO public.checkout_terms VALUES ($1, $2, $3, $4)",
                &[
                    &sql_id(*product),
                    &i32::try_from(*pack).unwrap(),
                    &piece.minor_units(),
                    &case.minor_units(),
                ],
            )
            .expect("seed checkout terms");
    }
    for (product, mrp) in &book.crates.piece_mrps {
        client
            .execute(
                "INSERT INTO public.crate_piece_mrps VALUES ($1, $2)",
                &[&sql_id(*product), &mrp.minor_units()],
            )
            .expect("seed piece MRP");
    }
}

impl PgAuthority {
    /// Connects to `ALM_PG_URL` (None when unset), opens the test transaction and seeds it with
    /// `seed`'s shop: products with their FIFO lots, customers, suppliers and quotes, so both
    /// authorities start from the same state.
    pub fn connect(seed: &LocalAuthority) -> Option<Self> {
        let url = match std::env::var("ALM_PG_URL") {
            Ok(url) => url,
            Err(std::env::VarError::NotPresent) => return None,
            Err(error) => panic!("invalid ALM_PG_URL: {error}"),
        };
        let mut client = Client::connect(&url, NoTls).expect("connect to test database");
        client.batch_execute("BEGIN").expect("begin");
        seed_shop(&mut client, seed);
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
        let claims = json!({"sub": "diff-account",
            "app_metadata": {"role": wire::role_claim(role)}})
        .to_string();
        client
            .execute(
                "SELECT set_config('request.jwt.claims', $1, true),
                     set_config('alm.allow_claim_roles', 'on', true),
                     set_config('request.headers', '{\"x-alm-device\": \"diff-device\"}', true)",
                &[&claims],
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

    /// The active FIFO layers of a product in FIFO order (inspection, as the test superuser).
    pub fn lots(&self, product: u64) -> Vec<Lot> {
        self.0
            .borrow_mut()
            .query(
                "SELECT received_on::text, remaining, unit_cost_minor FROM public.product_lots
                 WHERE product_id = $1 AND remaining > 0 ORDER BY received_on NULLS FIRST, id",
                &[&sql_id(product)],
            )
            .unwrap()
            .iter()
            .map(|r| {
                (
                    r.get(0),
                    u32::try_from(r.get::<_, i32>(1)).unwrap(),
                    r.get(2),
                )
            })
            .collect()
    }

    /// A product's fridge (display) count.
    pub fn fridge(&self, product: u64) -> Option<u32> {
        self.0
            .borrow_mut()
            .query_opt(
                "SELECT fridge FROM public.products WHERE id = $1",
                &[&sql_id(product)],
            )
            .unwrap()
            .map(|r| u32::try_from(r.get::<_, i32>(0)).unwrap())
    }
}

impl SalesAuthority for PgAuthority {
    fn submit(
        &mut self,
        role: Role,
        request: &RequestId,
        intent: &Intent,
    ) -> Result<Outcome, AuthorityError> {
        let (function, terms) = wire::command(intent);
        let value = self.as_role(
            role,
            &format!("SELECT public.{function}($1, $2)"),
            &[&request.as_str(), &terms],
        )?;
        wire::outcome(&value)
    }

    fn status(&self, role: Role, request: &RequestId) -> Result<Option<Outcome>, AuthorityError> {
        let value = self.as_role(
            role,
            "SELECT public.request_status($1)",
            &[&request.as_str()],
        )?;
        wire::status(&value)
    }

    fn lookup(
        &self,
        role: Role,
        request: &RequestId,
        intent: &Intent,
    ) -> Result<Lookup, AuthorityError> {
        let (function, terms) = wire::command(intent);
        let value = self.as_role(
            role,
            "SELECT public.request_lookup($1, $2, $3)",
            &[&request.as_str(), &function, &terms],
        )?;
        wire::lookup(&value)
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
        let claim = wire::role_claim(role);
        let value = self.as_role(
            role,
            "SELECT public.pull_changes($1, $2)",
            &[&since, &claim],
        )?;
        let mut batch = wire::decode_batch(&value, role)?;
        batch.as_of_ms = 0;
        Ok(batch)
    }
}
