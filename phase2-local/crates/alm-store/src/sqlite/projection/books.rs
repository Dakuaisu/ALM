//! Projected products, suppliers, crates, godown rows and GST bills, and the Owner-only buy
//! rates, supplier quotes, purchase orders and state sections.
use rusqlite::{Connection, Transaction};

use alm_core::Money;
use alm_core::projection::{
    ProjectedCrate, ProjectedGodownRow, ProjectedGstBill, ProjectedProductTerms, ProjectedSupplier,
    ProjectionSnapshot,
};

use super::read::rows;
use super::{DraftStorageError, ProjectionError, date, id, money, signed};

mod owner;
pub(super) use owner::{read_owner_books, write_owner_books};

pub(super) fn position(i: usize) -> Result<i64, ProjectionError> {
    signed(u64::try_from(i).map_err(|_| ProjectionError::Rejected)?)
}

pub(super) fn write_books(
    tx: &Transaction<'_>,
    s: &ProjectionSnapshot,
) -> Result<(), ProjectionError> {
    let m = Money::minor_units;
    for (i, r) in s.products.iter().enumerate() {
        tx.execute(
            "INSERT INTO projected_products VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                position(i)?,
                signed(r.id)?,
                r.item,
                r.brand,
                m(r.sell),
                r.reorder_level
            ],
        )?;
    }
    for (i, r) in s.suppliers.iter().enumerate() {
        tx.execute(
            "INSERT INTO projected_suppliers VALUES (?1, ?2, ?3)",
            rusqlite::params![position(i)?, signed(r.id)?, r.name],
        )?;
    }
    for (i, r) in s.crates.iter().enumerate() {
        tx.execute(
            "INSERT INTO projected_crates VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                position(i)?,
                signed(r.id)?,
                r.name,
                m(r.price),
                r.components
            ],
        )?;
    }
    for (i, r) in s.godown.iter().enumerate() {
        tx.execute(
            "INSERT INTO projected_godown VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![position(i)?, r.item, r.cases, r.note],
        )?;
    }
    for (i, r) in s.gst_bills.iter().enumerate() {
        tx.execute(
            "INSERT INTO projected_gst_bills VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                position(i)?,
                signed(r.id)?,
                signed(r.number)?,
                r.date.to_string(),
                r.customer,
                r.cancelled
            ],
        )?;
    }
    Ok(())
}

pub(super) fn read_books(conn: &Connection) -> Result<ProjectionSnapshot, DraftStorageError> {
    Ok(ProjectionSnapshot {
        products: rows(
            conn,
            "SELECT id, item, brand, sell, reorder_level FROM projected_products ORDER BY position",
            |r| {
                Ok(ProjectedProductTerms {
                    id: id(r, 0)?,
                    item: r.get(1)?,
                    brand: r.get(2)?,
                    sell: money(r, 3)?,
                    reorder_level: r.get(4)?,
                })
            },
        )?,
        suppliers: rows(
            conn,
            "SELECT id, name FROM projected_suppliers ORDER BY position",
            |r| {
                Ok(ProjectedSupplier {
                    id: id(r, 0)?,
                    name: r.get(1)?,
                })
            },
        )?,
        crates: rows(
            conn,
            "SELECT id, name, price, components FROM projected_crates ORDER BY position",
            |r| {
                Ok(ProjectedCrate {
                    id: id(r, 0)?,
                    name: r.get(1)?,
                    price: money(r, 2)?,
                    components: r.get(3)?,
                })
            },
        )?,
        godown: rows(
            conn,
            "SELECT item, cases, note FROM projected_godown ORDER BY position",
            |r| {
                Ok(ProjectedGodownRow {
                    item: r.get(0)?,
                    cases: r.get(1)?,
                    note: r.get(2)?,
                })
            },
        )?,
        gst_bills: rows(
            conn,
            "SELECT id, number, date, customer, cancelled FROM projected_gst_bills ORDER BY position",
            |r| {
                Ok(ProjectedGstBill {
                    id: id(r, 0)?,
                    number: id(r, 1)?,
                    date: date(r, 2)?,
                    customer: r.get(3)?,
                    cancelled: r.get(4)?,
                })
            },
        )?,
        ..ProjectionSnapshot::default()
    })
}
