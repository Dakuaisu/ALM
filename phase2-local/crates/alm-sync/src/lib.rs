//! Prepared PostgreSQL calls for the local sales schema.
#![forbid(unsafe_code)]

use postgres::{Client, Error};
use serde_json::Value;

/// Posts reviewed terms or returns the stored receipt for an identical replay.
///
/// # Errors
/// Returns the original PostgreSQL error, including its SQLSTATE.
pub fn post_sale(
    client: &mut Client,
    request_id: &str,
    terms_json: &Value,
) -> Result<Value, Error> {
    let statement = client.prepare("SELECT public.post_sale($1, $2)")?;
    client
        .query_one(&statement, &[&request_id, terms_json])?
        .try_get(0)
}

/// Allocates or replays the GST number associated with a bill.
///
/// # Errors
/// Returns the original PostgreSQL error, including its SQLSTATE.
pub fn allocate_gst_bill_number(client: &mut Client, bill_id: &str) -> Result<i64, Error> {
    let statement = client.prepare("SELECT public.allocate_gst_bill_number($1)")?;
    client.query_one(&statement, &[&bill_id])?.try_get(0)
}
