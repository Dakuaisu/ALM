//! Reads the stored rows back into a snapshot.
use rusqlite::{Connection, OptionalExtension};

use alm_core::confirmation::manual_cashbook::CashBookAmounts;
use alm_core::projection::{
    OwnerValues, ProjectedCashDay, ProjectedCustomer, ProjectedProduct, ProjectedReceipt,
    ProjectedSale, ProjectedStock, ProjectionSnapshot,
};
use alm_core::{BillDate, DraftId, Money, Role};

use super::books::{read_books, read_owner_books};
use super::{DraftStorageError, unsigned};

pub(super) fn rows<T>(
    conn: &Connection,
    sql: &str,
    read: impl Fn(&rusqlite::Row<'_>) -> Result<T, DraftStorageError>,
) -> Result<Vec<T>, DraftStorageError> {
    let mut query = conn.prepare(sql)?;
    let mut cursor = query.query([])?;
    let mut out = Vec::new();
    while let Some(row) = cursor.next()? {
        out.push(read(row)?);
    }
    Ok(out)
}

pub(super) fn read_snapshot(
    conn: &Connection,
    role: Role,
) -> Result<ProjectionSnapshot, DraftStorageError> {
    let sales = rows(
        conn,
        "SELECT draft_id, invoice, date, customer, items, lines, packets, total, paid, online, cancelled, balance FROM projected_sales ORDER BY position",
        |r| {
            let items: String = r.get(4)?;
            Ok(ProjectedSale {
                draft_id: draft(r, 0)?,
                invoice: id(r, 1)?,
                date: date(r, 2)?,
                customer: r.get(3)?,
                items: if items.is_empty() {
                    Vec::new()
                } else {
                    items.split('\n').map(str::to_owned).collect()
                },
                lines: r.get(5)?,
                packets: r.get(6)?,
                total: money(r, 7)?,
                paid: money(r, 8)?,
                online: r.get(9)?,
                cancelled: r.get(10)?,
                balance: money(r, 11)?,
            })
        },
    )?;
    let receipts = rows(
        conn,
        "SELECT slot, date, customer, amount, online FROM projected_receipts ORDER BY position",
        |r| {
            Ok(ProjectedReceipt {
                slot: id(r, 0)?,
                date: date(r, 1)?,
                customer: r.get(2)?,
                amount: money(r, 3)?,
                online: r.get(4)?,
            })
        },
    )?;
    let customers = rows(
        conn,
        "SELECT c.id, c.name, c.balance, coalesce(k.mobile, ''), coalesce(k.addr1, ''),
             coalesce(k.addr2, '')
         FROM projected_customers c LEFT JOIN projected_customer_contacts k USING (position)
         ORDER BY c.position",
        |r| {
            Ok(ProjectedCustomer {
                id: r.get(0)?,
                name: r.get(1)?,
                balance: money(r, 2)?,
                mobile: r.get(3)?,
                addr1: r.get(4)?,
                addr2: r.get(5)?,
            })
        },
    )?;
    let stock = rows(
        conn,
        "SELECT product, item, brand, on_hand, reorder_level FROM projected_stock ORDER BY position",
        |r| {
            Ok(ProjectedStock {
                product: id(r, 0)?,
                item: r.get(1)?,
                brand: r.get(2)?,
                on_hand: r.get(3)?,
                reorder_level: r.get(4)?,
            })
        },
    )?;
    let catalog = rows(
        conn,
        "SELECT id, name, rate FROM projected_catalog ORDER BY position",
        |r| {
            Ok(ProjectedProduct {
                id: id(r, 0)?,
                name: r.get(1)?,
                rate: money(r, 2)?,
            })
        },
    )?;
    let cash_days = rows(
        conn,
        "SELECT id, date, cash, paytm, phonepe, goods, misc, salary, debt FROM projected_cash_days ORDER BY position",
        |r| {
            Ok(ProjectedCashDay {
                id: id(r, 0)?,
                date: date(r, 1)?,
                amounts: CashBookAmounts {
                    cash: money(r, 2)?,
                    paytm: money(r, 3)?,
                    phonepe: money(r, 4)?,
                    goods: money(r, 5)?,
                    misc: money(r, 6)?,
                    salary: money(r, 7)?,
                    debt: money(r, 8)?,
                },
            })
        },
    )?;
    let owner = if role == Role::Owner {
        let sale_costs = rows(
            conn,
            "SELECT draft_id, revenue, cost FROM owner_sale_costs ORDER BY position",
            |r| {
                let cost: Option<i64> = r.get(2)?;
                Ok((
                    draft(r, 0)?,
                    money(r, 1)?,
                    cost.map(Money::from_minor_units),
                ))
            },
        )?;
        let value: Option<Option<i64>> = conn
            .query_row("SELECT value FROM owner_stock_value", [], |r| r.get(0))
            .optional()?;
        Some(OwnerValues {
            sale_costs,
            stock_value: value.flatten().map(Money::from_minor_units),
            ..OwnerValues::default()
        })
    } else {
        None
    };
    let books = read_books(conn)?;
    let owner = match owner {
        Some(owner) => Some(read_owner_books(conn, owner)?),
        None => None,
    };
    let unsupported: Option<Option<String>> = conn
        .query_row("SELECT sales_unsupported FROM projected_notes", [], |r| {
            r.get(0)
        })
        .optional()?;
    Ok(ProjectionSnapshot {
        sales_unsupported: unsupported
            .flatten()
            .map(|text| DraftId::parse(&text).map_err(|_| DraftStorageError::InvalidOutput))
            .transpose()?,
        sales,
        receipts,
        customers,
        stock,
        catalog,
        cash_days,
        owner,
        ..books
    })
}

pub(super) fn date(row: &rusqlite::Row<'_>, index: usize) -> Result<BillDate, DraftStorageError> {
    row.get::<_, String>(index)?
        .parse()
        .map_err(|_| DraftStorageError::InvalidOutput)
}

pub(super) fn money(row: &rusqlite::Row<'_>, index: usize) -> Result<Money, DraftStorageError> {
    Ok(Money::from_minor_units(row.get(index)?))
}

pub(super) fn id(row: &rusqlite::Row<'_>, index: usize) -> Result<u64, DraftStorageError> {
    unsigned(row.get(index)?)
}

fn draft(row: &rusqlite::Row<'_>, index: usize) -> Result<DraftId, DraftStorageError> {
    DraftId::parse(&row.get::<_, String>(index)?).map_err(|_| DraftStorageError::InvalidOutput)
}
