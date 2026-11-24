//! Per-account, per-role read-only projection of posted data received from the authority.
//! A batch is applied with its checkpoint in one transaction; the file can be dropped and
//! rebuilt from the authority at any time. Owner-only tables exist only in an Owner file, and a
//! Manager file refuses owner values, so costs and FIFO values never reach a Manager device.
use std::fs::{self, OpenOptions};
use std::io::ErrorKind;
use std::path::Path;

use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior};

use alm_core::confirmation::manual_cashbook::CashBookAmounts;
use alm_core::projection::{
    OwnerValues, ProjectedCashDay, ProjectedCustomer, ProjectedProduct, ProjectedReceipt,
    ProjectedSale, ProjectedStock, ProjectionBatch, ProjectionSnapshot,
};
use alm_core::{BillDate, Money, Role};

use super::{DraftStorageError, connect, scoped::account_key};

mod books;
use books::{read_books, read_owner_books, write_books, write_owner_books};

const DIRECTORY: &str = "projections";

const SHARED_TABLES: &str = "
CREATE TABLE projection_scope (account_hex TEXT NOT NULL, role TEXT NOT NULL) STRICT;
CREATE TABLE projection_checkpoint (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    checkpoint INTEGER NOT NULL CHECK (checkpoint >= 0),
    as_of_ms INTEGER NOT NULL CHECK (as_of_ms >= 0)
) STRICT;
CREATE TABLE projected_notes (
    id INTEGER PRIMARY KEY CHECK (id = 1), sales_unsupported INTEGER
) STRICT;
CREATE TABLE projected_sales (
    position INTEGER PRIMARY KEY, draft_id INTEGER NOT NULL, invoice INTEGER NOT NULL,
    date TEXT NOT NULL, customer TEXT NOT NULL, items TEXT NOT NULL, lines INTEGER NOT NULL,
    packets INTEGER NOT NULL, total INTEGER NOT NULL, paid INTEGER NOT NULL,
    online INTEGER NOT NULL, cancelled INTEGER NOT NULL, balance INTEGER NOT NULL
) STRICT;
CREATE TABLE projected_receipts (
    position INTEGER PRIMARY KEY, slot INTEGER NOT NULL, date TEXT NOT NULL,
    customer TEXT NOT NULL, amount INTEGER NOT NULL, online INTEGER NOT NULL
) STRICT;
CREATE TABLE projected_customers (
    position INTEGER PRIMARY KEY, id TEXT NOT NULL, name TEXT NOT NULL, balance INTEGER NOT NULL
) STRICT;
CREATE TABLE projected_stock (
    position INTEGER PRIMARY KEY, product INTEGER NOT NULL, item TEXT NOT NULL,
    brand TEXT NOT NULL, on_hand INTEGER NOT NULL, reorder_level INTEGER NOT NULL
) STRICT;
CREATE TABLE projected_catalog (
    position INTEGER PRIMARY KEY, id INTEGER NOT NULL, name TEXT NOT NULL, rate INTEGER NOT NULL
) STRICT;
CREATE TABLE projected_cash_days (
    position INTEGER PRIMARY KEY, id INTEGER NOT NULL, date TEXT NOT NULL,
    cash INTEGER NOT NULL, paytm INTEGER NOT NULL, phonepe INTEGER NOT NULL,
    goods INTEGER NOT NULL, misc INTEGER NOT NULL, salary INTEGER NOT NULL, debt INTEGER NOT NULL
) STRICT;
";

/// Added with the catalog, purchasing, crate, godown and GST screens; created on older files.
const BOOK_TABLES: &str = "
CREATE TABLE IF NOT EXISTS projected_products (
    position INTEGER PRIMARY KEY, id INTEGER NOT NULL, item TEXT NOT NULL, brand TEXT NOT NULL,
    sell INTEGER NOT NULL, reorder_level INTEGER NOT NULL
) STRICT;
CREATE TABLE IF NOT EXISTS projected_suppliers (
    position INTEGER PRIMARY KEY, id INTEGER NOT NULL, name TEXT NOT NULL
) STRICT;
CREATE TABLE IF NOT EXISTS projected_crates (
    position INTEGER PRIMARY KEY, id INTEGER NOT NULL, name TEXT NOT NULL,
    price INTEGER NOT NULL, components INTEGER NOT NULL
) STRICT;
CREATE TABLE IF NOT EXISTS projected_godown (
    position INTEGER PRIMARY KEY, item TEXT NOT NULL, cases INTEGER NOT NULL, note TEXT NOT NULL
) STRICT;
CREATE TABLE IF NOT EXISTS projected_gst_bills (
    position INTEGER PRIMARY KEY, id INTEGER NOT NULL, number INTEGER NOT NULL,
    date TEXT NOT NULL, customer TEXT NOT NULL, cancelled INTEGER NOT NULL
) STRICT;
";

const OWNER_BOOK_TABLES: &str = "
CREATE TABLE IF NOT EXISTS owner_buy_rates (product INTEGER PRIMARY KEY, buy INTEGER NOT NULL) STRICT;
CREATE TABLE IF NOT EXISTS owner_supplier_prices (
    product INTEGER NOT NULL, supplier INTEGER NOT NULL, rate INTEGER NOT NULL,
    PRIMARY KEY (product, supplier)
) STRICT;
CREATE TABLE IF NOT EXISTS owner_purchase_orders (
    position INTEGER PRIMARY KEY, id INTEGER NOT NULL, supplier TEXT NOT NULL,
    date TEXT NOT NULL, status TEXT NOT NULL
) STRICT;
CREATE TABLE IF NOT EXISTS owner_purchase_lines (
    order_position INTEGER NOT NULL, position INTEGER NOT NULL, item TEXT NOT NULL,
    product INTEGER, ordered INTEGER NOT NULL, received INTEGER NOT NULL,
    remaining INTEGER NOT NULL, rate INTEGER NOT NULL, PRIMARY KEY (order_position, position)
) STRICT;
CREATE TABLE IF NOT EXISTS owner_state_sections (section TEXT PRIMARY KEY, body TEXT NOT NULL) STRICT;
";

const OWNER_TABLES: &str = "
CREATE TABLE owner_sale_costs (
    draft_id INTEGER PRIMARY KEY, revenue INTEGER NOT NULL, cost INTEGER
) STRICT;
CREATE TABLE owner_stock_value (id INTEGER PRIMARY KEY CHECK (id = 1), value INTEGER) STRICT;
";

const PROJECTED: [&str; 12] = [
    "projected_notes",
    "projected_sales",
    "projected_receipts",
    "projected_customers",
    "projected_stock",
    "projected_catalog",
    "projected_cash_days",
    "projected_products",
    "projected_suppliers",
    "projected_crates",
    "projected_godown",
    "projected_gst_bills",
];

const OWNER_PROJECTED: [&str; 7] = [
    "owner_sale_costs",
    "owner_stock_value",
    "owner_buy_rates",
    "owner_supplier_prices",
    "owner_purchase_orders",
    "owner_purchase_lines",
    "owner_state_sections",
];

const fn role_text(role: Role) -> &'static str {
    match role {
        Role::Owner => "owner",
        Role::Manager => "manager",
    }
}

#[derive(Debug)]
pub enum ProjectionError {
    Storage(DraftStorageError),
    /// The batch is for another role, older than the stored checkpoint, or carries data this
    /// role's projection must not hold.
    Rejected,
}

impl From<DraftStorageError> for ProjectionError {
    fn from(error: DraftStorageError) -> Self {
        Self::Storage(error)
    }
}

impl From<rusqlite::Error> for ProjectionError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Storage(error.into())
    }
}

/// The stored projection with the checkpoint it reflects.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredProjection {
    pub checkpoint: u64,
    pub as_of_ms: u64,
    pub snapshot: ProjectionSnapshot,
}

pub struct ProjectionStore {
    conn: Connection,
    role: Role,
}

impl ProjectionStore {
    /// Opens (or creates) the projection file of `account` for `role` under `device_root`.
    ///
    /// # Errors
    /// Rejects invalid IDs, unsafe locations and a file bound to another account or role.
    pub fn open_scoped(
        device_root: &Path,
        account: &str,
        role: Role,
    ) -> Result<Self, DraftStorageError> {
        let key = account_key(account)?;
        if !device_root.is_absolute() {
            return Err(DraftStorageError::InvalidPath);
        }
        let directory = device_root.canonicalize()?.join(DIRECTORY);
        match fs::create_dir(&directory) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error.into()),
        }
        if !fs::symlink_metadata(&directory)?.is_dir() {
            return Err(DraftStorageError::InvalidPath);
        }
        let path = directory.join(format!("account-{key}-{}.sqlite3", role_text(role)));
        let created = match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => {
                drop(file);
                true
            }
            Err(error) if error.kind() == ErrorKind::AlreadyExists => false,
            Err(error) => return Err(error.into()),
        };
        if !fs::symlink_metadata(&path)?.is_file() {
            return Err(DraftStorageError::InvalidPath);
        }
        let mut conn = connect(&path)?;
        if created {
            let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
            tx.execute_batch(SHARED_TABLES)?;
            tx.execute_batch(BOOK_TABLES)?;
            if role == Role::Owner {
                tx.execute_batch(OWNER_TABLES)?;
                tx.execute_batch(OWNER_BOOK_TABLES)?;
            }
            tx.execute(
                "INSERT INTO projection_scope VALUES (?1, ?2)",
                (&key, role_text(role)),
            )?;
            tx.commit()?;
        } else {
            let bound: Option<(String, String)> = conn
                .query_row(
                    "SELECT account_hex, role FROM projection_scope",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()
                .map_err(|_| DraftStorageError::AccountStoreMismatch)?;
            if bound != Some((key, role_text(role).to_owned())) {
                return Err(DraftStorageError::AccountStoreMismatch);
            }
        }
        let current: i64 = conn.query_row(
            "SELECT count(*) FROM sqlite_master WHERE name = 'projected_products'",
            [],
            |row| row.get(0),
        )?;
        if current == 0 {
            // An older file: add the new tables and rebuild from the authority on next pull.
            let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
            tx.execute_batch(BOOK_TABLES)?;
            if role == Role::Owner {
                tx.execute_batch(OWNER_BOOK_TABLES)?;
            }
            clear_rows(&tx, role)?;
            tx.execute("DELETE FROM projection_checkpoint", [])?;
            tx.commit()?;
        }
        conn.pragma_update(None, "journal_mode", "WAL")?;
        Ok(Self { conn, role })
    }

    pub const fn role(&self) -> Role {
        self.role
    }

    /// The checkpoint the stored rows reflect, if any batch was applied.
    ///
    /// # Errors
    /// Fails on storage errors.
    pub fn checkpoint(&self) -> Result<Option<(u64, u64)>, DraftStorageError> {
        let row: Option<(i64, i64)> = self
            .conn
            .query_row(
                "SELECT checkpoint, as_of_ms FROM projection_checkpoint",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        row.map(|(c, a)| Ok((unsigned(c)?, unsigned(a)?)))
            .transpose()
    }

    /// Applies a batch and its checkpoint in one transaction: a snapshot replaces every row;
    /// without one only the checkpoint time moves.
    ///
    /// # Errors
    /// `Rejected` for another role, an older checkpoint or owner values on a Manager file;
    /// nothing changes on any error.
    pub fn apply(&mut self, batch: &ProjectionBatch) -> Result<(), ProjectionError> {
        if batch.role != self.role {
            return Err(ProjectionError::Rejected);
        }
        if let Some((stored, _)) = self.checkpoint()?
            && batch.checkpoint < stored
        {
            return Err(ProjectionError::Rejected);
        }
        if self.role != Role::Owner && batch.snapshot.as_ref().is_some_and(|s| s.owner.is_some()) {
            return Err(ProjectionError::Rejected);
        }
        let checkpoint = signed(batch.checkpoint)?;
        let as_of = signed(batch.as_of_ms)?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(snapshot) = &batch.snapshot {
            write_snapshot(&tx, self.role, snapshot)?;
        }
        tx.execute(
            "INSERT INTO projection_checkpoint (id, checkpoint, as_of_ms) VALUES (1, ?1, ?2)
             ON CONFLICT(id) DO UPDATE SET checkpoint = ?1, as_of_ms = ?2",
            (checkpoint, as_of),
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Drops every projected row and the checkpoint, so the next pull rebuilds from scratch.
    ///
    /// # Errors
    /// Fails on storage errors.
    pub fn clear(&mut self) -> Result<(), DraftStorageError> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        clear_rows(&tx, self.role)?;
        tx.execute("DELETE FROM projection_checkpoint", [])?;
        tx.commit()?;
        Ok(())
    }

    /// The stored projection, if any batch was applied.
    ///
    /// # Errors
    /// Fails on storage errors or corrupt rows.
    pub fn load(&self) -> Result<Option<StoredProjection>, DraftStorageError> {
        let Some((checkpoint, as_of_ms)) = self.checkpoint()? else {
            return Ok(None);
        };
        Ok(Some(StoredProjection {
            checkpoint,
            as_of_ms,
            snapshot: read_snapshot(&self.conn, self.role)?,
        }))
    }
}

fn signed(value: u64) -> Result<i64, ProjectionError> {
    i64::try_from(value).map_err(|_| ProjectionError::Rejected)
}

fn unsigned(value: i64) -> Result<u64, DraftStorageError> {
    u64::try_from(value).map_err(|_| DraftStorageError::InvalidOutput)
}

fn clear_rows(tx: &Transaction<'_>, role: Role) -> rusqlite::Result<()> {
    for table in PROJECTED {
        if table_exists(tx, table)? {
            tx.execute(&format!("DELETE FROM {table}"), [])?;
        }
    }
    if role == Role::Owner {
        for table in OWNER_PROJECTED {
            if table_exists(tx, table)? {
                tx.execute(&format!("DELETE FROM {table}"), [])?;
            }
        }
    }
    Ok(())
}

fn table_exists(conn: &Connection, table: &str) -> rusqlite::Result<bool> {
    conn.query_row(
        "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
        [table],
        |row| row.get::<_, i64>(0),
    )
    .map(|count| count > 0)
}

fn write_snapshot(
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

fn rows<T>(
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

fn date(row: &rusqlite::Row<'_>, index: usize) -> Result<BillDate, DraftStorageError> {
    row.get::<_, String>(index)?
        .parse()
        .map_err(|_| DraftStorageError::InvalidOutput)
}

fn money(row: &rusqlite::Row<'_>, index: usize) -> Result<Money, DraftStorageError> {
    Ok(Money::from_minor_units(row.get(index)?))
}

fn id(row: &rusqlite::Row<'_>, index: usize) -> Result<u64, DraftStorageError> {
    unsigned(row.get(index)?)
}

fn read_snapshot(conn: &Connection, role: Role) -> Result<ProjectionSnapshot, DraftStorageError> {
    let sales = rows(
        conn,
        "SELECT draft_id, invoice, date, customer, items, lines, packets, total, paid, online, cancelled, balance FROM projected_sales ORDER BY position",
        |r| {
            let items: String = r.get(4)?;
            Ok(ProjectedSale {
                draft_id: id(r, 0)?,
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
        "SELECT id, name, balance FROM projected_customers ORDER BY position",
        |r| {
            Ok(ProjectedCustomer {
                id: r.get(0)?,
                name: r.get(1)?,
                balance: money(r, 2)?,
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
            "SELECT draft_id, revenue, cost FROM owner_sale_costs ORDER BY draft_id",
            |r| {
                let cost: Option<i64> = r.get(2)?;
                Ok((id(r, 0)?, money(r, 1)?, cost.map(Money::from_minor_units)))
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
    let unsupported: Option<Option<i64>> = conn
        .query_row("SELECT sales_unsupported FROM projected_notes", [], |r| {
            r.get(0)
        })
        .optional()?;
    Ok(ProjectionSnapshot {
        sales_unsupported: unsupported.flatten().map(unsigned).transpose()?,
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
