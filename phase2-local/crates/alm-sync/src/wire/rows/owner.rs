//! The Owner-only values of a projection batch.
use alm_core::Money;
use alm_core::projection::{OwnerValues, ProjectedPurchaseLine, ProjectedPurchaseOrder};
use serde_json::Value;

use super::{date, list, money, small, text};
use crate::AuthorityError;
use crate::wire::number;

pub(super) fn decode_owner(o: &Value) -> Result<OwnerValues, AuthorityError> {
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
        book: if o["book"].is_object() {
            alm_core::projection::book::from_json(&o["book"])
                .map_err(|_| AuthorityError::Invalid)?
        } else {
            alm_core::projection::book::OwnerBook::default()
        },
    })
}
