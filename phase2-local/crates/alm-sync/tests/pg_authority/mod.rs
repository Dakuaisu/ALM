//! DEV-ONLY Postgres implementation of `SalesAuthority` over the migrated local schema, for the
//! conformance and differential suites. Each call runs as the `authenticated` role with the
//! caller's role in the JWT claims (`auth.jwt()`), inside one long transaction that is rolled
//! back when the authority is dropped; a savepoint per call stands in for the function's own
//! transaction. The JSON mapping is `alm_sync::wire`, shared with the REST transport.
#![allow(dead_code)]

use std::cell::RefCell;

use alm_core::projection::ProjectionBatch;
use alm_core::{ProductId, Role};
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
        let snapshot = seed.pull(Role::Owner, None).unwrap().snapshot.unwrap();
        let owner = snapshot.owner.as_ref().expect("an owner seed");
        for p in &snapshot.products {
            let stock = snapshot
                .stock
                .iter()
                .find(|s| s.product == p.id)
                .map_or(0, |s| s.on_hand);
            let buy = owner
                .buy_rates
                .iter()
                .find(|(id, _)| *id == p.id)
                .map_or(0, |(_, b)| b.minor_units());
            let provisional = snapshot
                .catalog
                .iter()
                .find(|c| c.id == p.id)
                .map(|c| c.rate.minor_units());
            client
                .execute(
                    "INSERT INTO public.products (id, item, brand, sell_minor, buy_minor, stock, reorder_level, provisional_minor)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
                    &[
                        &sql_id(p.id),
                        &p.item,
                        &p.brand,
                        &p.sell.minor_units(),
                        &buy,
                        &i32::try_from(stock).unwrap(),
                        &i32::try_from(p.reorder_level).unwrap(),
                        &provisional,
                    ],
                )
                .expect("seed product");
            let lots = seed
                .open_stock_lots(Role::Owner, ProductId::new(p.id))
                .expect("seed lots");
            for lot in &lots.rows {
                client
                    .execute(
                        "INSERT INTO public.product_lots (product_id, received_on, remaining, unit_cost_minor)
                         VALUES ($1, $2::text::date, $3, $4)",
                        &[
                            &sql_id(p.id),
                            &lot.date.map(|d| d.to_string()),
                            &i32::try_from(lot.remaining.get()).unwrap(),
                            &lot.unit_cost.map(alm_core::Money::minor_units),
                        ],
                    )
                    .expect("seed lot");
            }
        }
        for c in &snapshot.customers {
            assert_eq!(c.balance.minor_units(), 0, "seed customers start settled");
            client
                .execute(
                    "INSERT INTO public.customers (id, name) VALUES ($1, $2)",
                    &[&c.id, &c.name],
                )
                .expect("seed customer");
        }
        for s in &snapshot.suppliers {
            client
                .execute(
                    "INSERT INTO public.suppliers (id, name) VALUES ($1, $2)",
                    &[&sql_id(s.id), &s.name],
                )
                .expect("seed supplier");
        }
        for (product, supplier, rate) in &owner.supplier_prices {
            client
                .execute(
                    "INSERT INTO public.supplier_prices (product_id, supplier_id, rate_minor) VALUES ($1, $2, $3)",
                    &[&sql_id(*product), &sql_id(*supplier), &rate.minor_units()],
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
        let claims = json!({"app_metadata": {"role": wire::role_claim(role)}}).to_string();
        client
            .execute(
                "SELECT set_config('request.jwt.claims', $1, true)",
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
        let mut value = self.as_role(role, "SELECT public.pull_projection($1)", &[&since])?;
        if !value["rows"].is_null() && value["full"] != json!(true) {
            // Something changed: the device applies full replacements, so read everything.
            value = self.as_role(role, "SELECT public.pull_projection($1)", &[&None::<i64>])?;
        }
        let mut batch = wire::decode_batch(&value, role)?;
        batch.as_of_ms = 0;
        Ok(batch)
    }
}
