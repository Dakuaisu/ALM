//! Set `ALM_PG_URL` to a disposable, migrated local database, then run:
//! `cargo test --locked --offline -p alm-sync --test postgres_replay`
#![forbid(unsafe_code)]

use postgres::{Client, Error, NoTls};
use serde_json::{Value, json};

fn client() -> Option<Client> {
    let url = match std::env::var("ALM_PG_URL") {
        Ok(url) => url,
        Err(std::env::VarError::NotPresent) => return None,
        Err(error) => panic!("invalid ALM_PG_URL: {error}"),
    };
    let mut client = Client::connect(&url, NoTls).expect("connect to test database");
    client
        .batch_execute(
            "BEGIN; SELECT set_config('request.jwt.claims', '{\"app_metadata\": {\"role\": \"owner\"}}', true),
                 set_config('alm.allow_claim_roles', 'on', true)",
        )
        .expect("begin test transaction as the owner");
    Some(client)
}

fn post_sale(client: &mut Client, request_id: &str, terms_json: &Value) -> Result<Value, Error> {
    let statement = client.prepare("SELECT public.post_sale($1, $2)")?;
    client
        .query_one(&statement, &[&request_id, terms_json])?
        .try_get(0)
}

fn allocate_gst_bill_number(client: &mut Client, bill_id: &str) -> Result<i64, Error> {
    let statement = client.prepare("SELECT public.allocate_gst_bill_number($1)")?;
    client.query_one(&statement, &[&bill_id])?.try_get(0)
}

fn terms(draft: &str, quantity: i32) -> Value {
    json!({"draft_id": draft, "lines": [{"product_id": 900000000000000001_i64,
        "quantity": quantity, "unit_price_minor": 125}],
        "customer_id": "replay-customer", "customer": "Synthetic", "date": "2025-04-30"})
}

fn seed(client: &mut Client) {
    client
        .execute(
            "INSERT INTO public.products (id, item, sell_minor, stock) VALUES ($1, $2, $3, $4)",
            &[
                &900000000000000001_i64,
                &"Synthetic product",
                &125_i64,
                &5_i32,
            ],
        )
        .expect("insert synthetic product");
    client
        .batch_execute(
            "INSERT INTO public.product_lots (product_id, remaining) VALUES (900000000000000001, 5);
             INSERT INTO public.customers (id, name) VALUES ('replay-customer', 'Synthetic')",
        )
        .expect("insert its lot and a customer");
}

fn stock(client: &mut Client) -> i32 {
    client
        .query_one(
            "SELECT stock FROM public.products WHERE id = $1",
            &[&900000000000000001_i64],
        )
        .expect("read stock")
        .get(0)
}

#[test]
fn successful_sale_and_lost_response_replay_have_one_effect() {
    let Some(mut client) = client() else { return };
    seed(&mut client);
    let first = post_sale(
        &mut client,
        "sync-replay-request",
        &terms("sync-replay-draft", 2),
    )
    .expect("post sale");
    assert_eq!(first["draft_id"], "sync-replay-draft");
    assert_eq!(first["quantity"], 2);
    assert_eq!(first["total_minor"], 250);
    assert_eq!(stock(&mut client), 3);
    client
        .execute(
            "UPDATE public.products SET sell_minor = 400 WHERE id = $1",
            &[&900000000000000001_i64],
        )
        .expect("change price");
    let retry = post_sale(
        &mut client,
        "sync-replay-request",
        &terms("sync-replay-draft", 2),
    )
    .expect("replay sale");
    assert_eq!(retry, first);
    assert_eq!(stock(&mut client), 3);
    let row = client.query_one("SELECT (SELECT count(*) FROM public.sale_requests WHERE request_id = $1), (SELECT count(*) FROM public.sales WHERE draft_id = $2)", &[&"sync-replay-request", &"sync-replay-draft"]).expect("read effects");
    assert_eq!((row.get::<_, i64>(0), row.get::<_, i64>(1)), (1, 1));
}

#[test]
fn reused_request_with_different_review_conflicts() {
    let Some(mut client) = client() else { return };
    seed(&mut client);
    post_sale(
        &mut client,
        "sync-conflict-request",
        &terms("sync-conflict-draft", 2),
    )
    .expect("post sale");
    client
        .batch_execute("SAVEPOINT before_conflict")
        .expect("savepoint");
    let error = post_sale(
        &mut client,
        "sync-conflict-request",
        &terms("sync-conflict-draft", 3),
    )
    .expect_err("conflict");
    assert_eq!(error.code().map(|code| code.code()), Some("ALM01"));
    client
        .batch_execute("ROLLBACK TO SAVEPOINT before_conflict")
        .expect("recover transaction");
    assert_eq!(stock(&mut client), 3);
}

#[test]
fn changed_draft_or_terms_rejects_stale_review() {
    let Some(mut client) = client() else { return };
    seed(&mut client);
    post_sale(
        &mut client,
        "sync-draft-request",
        &terms("sync-draft-one", 2),
    )
    .expect("post sale");
    client
        .batch_execute("SAVEPOINT before_conflict")
        .expect("savepoint");
    let error = post_sale(
        &mut client,
        "sync-draft-request",
        &terms("sync-draft-two", 1),
    )
    .expect_err("conflict");
    assert_eq!(error.code().map(|code| code.code()), Some("ALM01"));
    client
        .batch_execute("ROLLBACK TO SAVEPOINT before_conflict")
        .expect("recover transaction");
    assert_eq!(stock(&mut client), 3);
}

#[test]
fn gst_allocation_replays_the_same_number() {
    let Some(mut client) = client() else { return };
    let first = allocate_gst_bill_number(&mut client, "sync-gst-a").expect("allocate");
    let second = allocate_gst_bill_number(&mut client, "sync-gst-b").expect("allocate second");
    assert_eq!(second, first + 1);
    assert_eq!(
        allocate_gst_bill_number(&mut client, "sync-gst-a").expect("replay"),
        first
    );
}
