//! The Owner-only buy rates, supplier quotes, purchase orders and state sections.
use rusqlite::{Connection, Transaction};

use alm_core::Money;
use alm_core::projection::{OwnerValues, ProjectedPurchaseLine, ProjectedPurchaseOrder};

use super::super::read::rows;
use super::super::{DraftStorageError, ProjectionError, date, id, money, signed};
use super::position;

pub(in crate::sqlite::projection) fn write_owner_books(
    tx: &Transaction<'_>,
    owner: &OwnerValues,
) -> Result<(), ProjectionError> {
    let m = Money::minor_units;
    for (product, buy) in &owner.buy_rates {
        tx.execute(
            "INSERT INTO owner_buy_rates VALUES (?1, ?2)",
            (signed(*product)?, m(*buy)),
        )?;
    }
    for (product, supplier, rate) in &owner.supplier_prices {
        tx.execute(
            "INSERT INTO owner_supplier_prices VALUES (?1, ?2, ?3)",
            (signed(*product)?, signed(*supplier)?, m(*rate)),
        )?;
    }
    for (i, order) in owner.purchase_orders.iter().enumerate() {
        tx.execute(
            "INSERT INTO owner_purchase_orders VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                position(i)?,
                signed(order.id)?,
                order.supplier,
                order.date.to_string(),
                order.status
            ],
        )?;
        for (j, line) in order.lines.iter().enumerate() {
            tx.execute(
                "INSERT INTO owner_purchase_lines VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params![
                    position(i)?,
                    position(j)?,
                    line.item,
                    line.product.map(signed).transpose()?,
                    line.ordered,
                    line.received,
                    line.remaining,
                    m(line.rate)
                ],
            )?;
        }
    }
    for (section, body) in &owner.state {
        tx.execute(
            "INSERT INTO owner_state_sections VALUES (?1, ?2)",
            (section, body),
        )?;
    }
    Ok(())
}

pub(in crate::sqlite::projection) fn read_owner_books(
    conn: &Connection,
    owner: OwnerValues,
) -> Result<OwnerValues, DraftStorageError> {
    let buy_rates = rows(
        conn,
        "SELECT product, buy FROM owner_buy_rates ORDER BY rowid",
        |r| Ok((id(r, 0)?, money(r, 1)?)),
    )?;
    let supplier_prices = rows(
        conn,
        "SELECT product, supplier, rate FROM owner_supplier_prices ORDER BY rowid",
        |r| Ok((id(r, 0)?, id(r, 1)?, money(r, 2)?)),
    )?;
    let mut purchase_orders = rows(
        conn,
        "SELECT id, supplier, date, status FROM owner_purchase_orders ORDER BY position",
        |r| {
            Ok(ProjectedPurchaseOrder {
                id: id(r, 0)?,
                supplier: r.get(1)?,
                date: date(r, 2)?,
                status: r.get(3)?,
                lines: Vec::new(),
            })
        },
    )?;
    let lines = rows(
        conn,
        "SELECT order_position, item, product, ordered, received, remaining, rate FROM owner_purchase_lines ORDER BY order_position, position",
        |r| {
            let product: Option<i64> = r.get(2)?;
            Ok((
                id(r, 0)?,
                ProjectedPurchaseLine {
                    item: r.get(1)?,
                    product: product
                        .map(|p| u64::try_from(p).map_err(|_| DraftStorageError::InvalidOutput))
                        .transpose()?,
                    ordered: r.get(3)?,
                    received: r.get(4)?,
                    remaining: r.get(5)?,
                    rate: money(r, 6)?,
                },
            ))
        },
    )?;
    for (order, line) in lines {
        usize::try_from(order)
            .ok()
            .and_then(|i| purchase_orders.get_mut(i))
            .ok_or(DraftStorageError::InvalidOutput)?
            .lines
            .push(line);
    }
    let state = rows(
        conn,
        "SELECT section, body FROM owner_state_sections ORDER BY section",
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    Ok(OwnerValues {
        buy_rates,
        supplier_prices,
        purchase_orders,
        state,
        ..owner
    })
}
