//! Opt-in: the `SalesAuthority` post_sale and GST calls against the migrated local schema.
//! Set `ALM_PG_URL` to a disposable, migrated database; without it every test returns early.
#![forbid(unsafe_code)]

use std::num::NonZeroU32;

use alm_core::projection::ProjectionBatch;
use alm_core::{
    BillDate, DraftCustomer, DraftId, DraftLine, Money, ProductId, ProvisionalDraft, Role,
};
use alm_sync::{
    AuthorityError, Intent, Outcome, RequestId, SaleIntent, SaleReceipt, SalesAuthority,
};
use postgres::{Client, NoTls};
use serde_json::Value;

/// A test-only adapter from the trait to the schema's functions, inside one rolled-back
/// transaction. Not a device transport.
struct PgAuthority(Client);

impl SalesAuthority for PgAuthority {
    fn submit(
        &mut self,
        role: Role,
        request: &RequestId,
        intent: &Intent,
    ) -> Result<Outcome, AuthorityError> {
        if role != Role::Owner {
            return Err(AuthorityError::Denied);
        }
        if !matches!(intent, Intent::PostSale(_)) {
            return Err(AuthorityError::Invalid);
        }
        let (_, terms) = alm_sync::wire::command(intent);
        self.0
            .batch_execute("SAVEPOINT submit")
            .map_err(|_| AuthorityError::Unavailable)?;
        match self.0.query_one(
            "SELECT public.post_sale($1, $2)",
            &[&request.as_str(), &terms],
        ) {
            Ok(row) => {
                let receipt: Value = row.get(0);
                Ok(Outcome::Sale(SaleReceipt {
                    draft: receipt["draft_id"]
                        .as_str()
                        .and_then(|d| d.parse().ok())
                        .ok_or(AuthorityError::Invalid)?,
                    invoice: receipt["invoice"].as_u64().ok_or(AuthorityError::Invalid)?,
                    quantity: receipt["quantity"]
                        .as_u64()
                        .and_then(|q| u32::try_from(q).ok())
                        .ok_or(AuthorityError::Invalid)?,
                    total: Money::from_minor_units(
                        receipt["total_minor"]
                            .as_i64()
                            .ok_or(AuthorityError::Invalid)?,
                    ),
                }))
            }
            Err(error) => {
                self.0
                    .batch_execute("ROLLBACK TO SAVEPOINT submit")
                    .map_err(|_| AuthorityError::Unavailable)?;
                Err(match error.code().map(|code| code.code()) {
                    Some("ALM01") => AuthorityError::Conflict,
                    Some("ALM02") => AuthorityError::AlreadyPosted,
                    Some("ALM03") => AuthorityError::StaleReview,
                    Some(code) => AuthorityError::Rejected(code.to_owned()),
                    None => AuthorityError::Unavailable,
                })
            }
        }
    }

    fn status(&self, _: Role, _: &RequestId) -> Result<Option<Outcome>, AuthorityError> {
        Err(AuthorityError::Unavailable)
    }

    fn allocate_gst_number(&mut self, role: Role, bill_id: &str) -> Result<u64, AuthorityError> {
        if role != Role::Owner {
            return Err(AuthorityError::Denied);
        }
        let number: i64 = self
            .0
            .query_one("SELECT public.allocate_gst_bill_number($1)", &[&bill_id])
            .map_err(|_| AuthorityError::Unavailable)?
            .get(0);
        u64::try_from(number).map_err(|_| AuthorityError::Invalid)
    }

    fn pull(&self, _: Role, _: Option<u64>) -> Result<ProjectionBatch, AuthorityError> {
        Err(AuthorityError::Unavailable)
    }
}

fn authority() -> Option<PgAuthority> {
    let url = match std::env::var("ALM_PG_URL") {
        Ok(url) => url,
        Err(std::env::VarError::NotPresent) => return None,
        Err(error) => panic!("invalid ALM_PG_URL: {error}"),
    };
    let mut client = Client::connect(&url, NoTls).expect("connect to test database");
    client
        .batch_execute(
            "BEGIN; SELECT set_config('request.jwt.claims', '{\"app_metadata\": {\"role\": \"owner\"}}', true)",
        )
        .expect("begin as the owner");
    client
        .batch_execute(
            "INSERT INTO public.products (id, item, sell_minor, stock) VALUES (900000000000000002, 'Synthetic trait product', 125, 5);
             INSERT INTO public.product_lots (product_id, remaining) VALUES (900000000000000002, 5);
             INSERT INTO public.customers (id, name) VALUES ('trait-customer', 'Synthetic')",
        )
        .expect("seed");
    Some(PgAuthority(client))
}

fn sale(draft: u64, quantity: u32, price: i64) -> SaleIntent {
    let day = BillDate::new(2025, 4, 30).unwrap();
    SaleIntent {
        draft: ProvisionalDraft::new(
            DraftId::new(draft),
            vec![DraftLine::new(
                ProductId::new(900000000000000002),
                NonZeroU32::new(quantity).unwrap(),
            )],
        )
        .with_customer(DraftCustomer::Selected {
            id: alm_core::DraftCustomerId::new("trait-customer".into()),
            name: "Synthetic".into(),
        })
        .with_bill_date(Some(day)),
        date: day,
        paid: Money::from_minor_units(0),
        prices: vec![Money::from_minor_units(price)],
    }
}

#[test]
fn trait_post_sale_replays_and_conflicts_against_the_schema() {
    let Some(mut pg) = authority() else { return };
    let request = RequestId::new("trait-request-1").unwrap();
    let first = pg
        .post_sale(Role::Owner, &request, &sale(9_001, 2, 125))
        .expect("post");
    assert_eq!(
        (first.draft, first.quantity, first.total.minor_units()),
        (9_001, 2, 250)
    );
    assert_eq!(
        pg.post_sale(Role::Owner, &request, &sale(9_001, 2, 125)),
        Ok(first)
    );
    assert_eq!(
        pg.post_sale(Role::Owner, &request, &sale(9_001, 3, 125)),
        Err(AuthorityError::Conflict)
    );
    let other = RequestId::new("trait-request-2").unwrap();
    assert_eq!(
        pg.post_sale(Role::Owner, &other, &sale(9_002, 1, 99)),
        Err(AuthorityError::StaleReview)
    );
    assert_eq!(
        pg.post_sale(Role::Owner, &other, &sale(9_001, 1, 125)),
        Err(AuthorityError::AlreadyPosted)
    );
}

#[test]
fn trait_gst_allocation_is_idempotent_against_the_schema() {
    let Some(mut pg) = authority() else { return };
    let a = pg
        .allocate_gst_number(Role::Owner, "trait-gst-a")
        .expect("allocate");
    let b = pg
        .allocate_gst_number(Role::Owner, "trait-gst-b")
        .expect("allocate");
    assert_eq!(b, a + 1);
    assert_eq!(pg.allocate_gst_number(Role::Owner, "trait-gst-a"), Ok(a));
    assert_eq!(
        pg.allocate_gst_number(Role::Manager, "trait-gst-c"),
        Err(AuthorityError::Denied)
    );
}
