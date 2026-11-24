use std::fmt::Write as _;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

#[path = "sqlite_dump.rs"]
mod sqlite_dump;
#[path = "sqlite_list.rs"]
mod sqlite_list;

const COLUMNS: &str = concat!(
    "table|manager_account_scope|account_hex\n",
    "table|provisional_draft_crates|draft_id\n",
    "table|provisional_draft_crates|definition\n",
    "table|provisional_draft_customers|draft_id\n",
    "table|provisional_draft_customers|kind\n",
    "table|provisional_draft_customers|customer_id\n",
    "table|provisional_draft_customers|name\n",
    "table|provisional_draft_dates|draft_id\n",
    "table|provisional_draft_dates|bill_date\n",
    "table|provisional_draft_lines|draft_id\n",
    "table|provisional_draft_lines|position\n",
    "table|provisional_draft_lines|product_id\n",
    "table|provisional_draft_lines|quantity\n",
    "table|provisional_draft_loading|draft_id\n",
    "table|provisional_draft_loading|rate\n",
    "table|provisional_drafts|id\n",
);

// Include extra objects as well as hidden/generated columns; skip implicit PK indexes.
const SCHEMA: &str = "SELECT m.type, m.name, p.name FROM sqlite_schema AS m
    LEFT JOIN pragma_table_xinfo(m.name) AS p ON m.type IN ('table', 'view')
    WHERE m.sql IS NOT NULL ORDER BY m.name, p.cid;";

const ROWS: &str = "SELECT 'draft', id FROM provisional_drafts ORDER BY id;
    SELECT 'customer', draft_id, kind, customer_id, name
        FROM provisional_draft_customers ORDER BY draft_id;
    SELECT 'date', draft_id, bill_date FROM provisional_draft_dates ORDER BY draft_id;
    SELECT 'line', draft_id, position, product_id, quantity
        FROM provisional_draft_lines ORDER BY draft_id, position;
    SELECT 'crate', draft_id, definition FROM provisional_draft_crates ORDER BY draft_id;";

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct SqliteEvidence {
    columns: String,
    scope: String,
    rows: String,
    dump: String,
}

impl SqliteEvidence {
    pub(crate) fn read(path: &Path) -> Self {
        assert!(path.is_file(), "routed SQLite file must exist");
        let columns = query(path, SCHEMA);
        assert_eq!(columns, COLUMNS, "exact Manager schema allowlist");
        Self {
            columns,
            scope: query(path, "SELECT account_hex FROM manager_account_scope;"),
            rows: query(path, ROWS),
            dump: sqlite_dump::sqlite_dump(&open(path)).expect("read-only SQLite dump"),
        }
    }

    pub(crate) fn dump(&self) -> &str {
        &self.dump
    }

    pub(crate) fn assert_no_markers(&self, markers: &[&str]) {
        let lower = self.dump().to_ascii_lowercase();
        for marker in markers {
            assert!(
                !lower.contains(&marker.to_ascii_lowercase()),
                "unexpected {marker} in Manager dump"
            );
        }
    }

    pub(crate) fn rows(&self) -> &str {
        &self.rows
    }

    pub(crate) fn assert_account_binding(&self, account: &str) {
        let key = account_hex(account);
        assert_eq!(
            self.scope,
            format!("{key}\n"),
            "exact Manager account binding"
        );
    }
}

pub(crate) fn account_hex(account: &str) -> String {
    let mut key = String::new();
    for byte in account.bytes() {
        write!(key, "{byte:02x}").unwrap();
    }
    key
}

fn open(path: &Path) -> rusqlite::Connection {
    sqlite_list::open_read_only(path).expect("open Manager SQLite read-only")
}

fn query(path: &Path, query: &str) -> String {
    sqlite_list::sqlite_list(&open(path), query).expect("read-only SQLite projection")
}

pub(crate) fn databases(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut dirs = vec![root.to_owned()];
    while let Some(dir) = dirs.pop() {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let kind = entry.file_type().unwrap();
            assert!(
                !kind.is_symlink(),
                "unexpected link under synthetic device root"
            );
            if kind.is_dir() {
                dirs.push(path);
            } else if kind.is_file() {
                let mut header = [0; 16];
                let read = File::open(&path).unwrap().read_exact(&mut header);
                if read.is_ok() && &header == b"SQLite format 3\0" {
                    files.push(path);
                }
            }
        }
    }
    files.sort();
    files
}
