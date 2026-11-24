//! The projection file's tables: shared, book and Owner-only.
use rusqlite::{Connection, Transaction};

use alm_core::Role;

pub(super) const SHARED_TABLES: &str = "
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
pub(super) const BOOK_TABLES: &str = "
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

pub(super) const OWNER_BOOK_TABLES: &str = "
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

pub(super) const OWNER_TABLES: &str = "
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

pub(super) fn clear_rows(tx: &Transaction<'_>, role: Role) -> rusqlite::Result<()> {
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
