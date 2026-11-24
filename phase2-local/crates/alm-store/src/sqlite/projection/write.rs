//! Writes a snapshot's rows, replacing every stored row.
use rusqlite::Transaction;

use alm_core::projection::ProjectionSnapshot;
use alm_core::{Money, Role};

use super::books::{write_books, write_owner_books};
use super::{ProjectionError, clear_rows, signed};

pub(super) fn write_snapshot(
    tx: &Transaction<'_>,
    role: Role,
    s: &ProjectionSnapshot,
) -> Result<(), ProjectionError> {
    clear_rows(tx, role)?;
    let m = |money: Money| money.minor_units();
    let unsupported = s.sales_unsupported.map(signed).transpose()?;
    tx.execute("INSERT INTO projected_notes VALUES (1, ?1)", [unsupported])?;
    for (i, r) in s.sales.iter().enumerate() {
        tx.execute(
            "INSERT INTO projected_sales VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            rusqlite::params![
                signed(i as u64)?,
                signed(r.draft_id)?,
                signed(r.invoice)?,
                r.date.to_string(),
                r.customer,
                r.items.join("\n"),
                r.lines,
                r.packets,
                m(r.total),
                m(r.paid),
                r.online,
                r.cancelled,
                m(r.balance)
            ],
        )?;
    }
    for (i, r) in s.receipts.iter().enumerate() {
        tx.execute(
            "INSERT INTO projected_receipts VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                signed(i as u64)?,
                signed(r.slot)?,
                r.date.to_string(),
                r.customer,
                m(r.amount),
                r.online
            ],
        )?;
    }
    for (i, r) in s.customers.iter().enumerate() {
        tx.execute(
            "INSERT INTO projected_customers VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![signed(i as u64)?, r.id, r.name, m(r.balance)],
        )?;
        tx.execute(
            "INSERT INTO projected_customer_contacts VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![signed(i as u64)?, r.mobile, r.addr1, r.addr2],
        )?;
    }
    for (i, r) in s.stock.iter().enumerate() {
        tx.execute(
            "INSERT INTO projected_stock VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                signed(i as u64)?,
                signed(r.product)?,
                r.item,
                r.brand,
                r.on_hand,
                r.reorder_level
            ],
        )?;
    }
    for (i, r) in s.catalog.iter().enumerate() {
        tx.execute(
            "INSERT INTO projected_catalog VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![signed(i as u64)?, signed(r.id)?, r.name, m(r.rate)],
        )?;
    }
    for (i, r) in s.cash_days.iter().enumerate() {
        let a = r.amounts;
        tx.execute(
            "INSERT INTO projected_cash_days VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                signed(i as u64)?,
                signed(r.id)?,
                r.date.to_string(),
                m(a.cash),
                m(a.paytm),
                m(a.phonepe),
                m(a.goods),
                m(a.misc),
                m(a.salary),
                m(a.debt)
            ],
        )?;
    }
    write_books(tx, s)?;
    match (&s.owner, role) {
        (Some(owner), Role::Owner) => {
            for (draft, revenue, cost) in &owner.sale_costs {
                tx.execute(
                    "INSERT INTO owner_sale_costs VALUES (?1, ?2, ?3)",
                    (signed(*draft)?, m(*revenue), cost.map(m)),
                )?;
            }
            tx.execute(
                "INSERT INTO owner_stock_value VALUES (1, ?1)",
                [owner.stock_value.map(m)],
            )?;
            write_owner_books(tx, owner)?;
        }
        (None, _) => {}
        (Some(_), Role::Manager) => return Err(ProjectionError::Rejected),
    }
    Ok(())
}
