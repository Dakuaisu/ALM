use std::env;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::fixtures::{Account, DraftSpec};

const COLUMNS: &str = concat!(
    "manager_account_scope|account_hex\n",
    "provisional_draft_crates|draft_id\n",
    "provisional_draft_crates|definition\n",
    "provisional_draft_customers|draft_id\n",
    "provisional_draft_customers|kind\n",
    "provisional_draft_customers|customer_id\n",
    "provisional_draft_customers|name\n",
    "provisional_draft_dates|draft_id\n",
    "provisional_draft_dates|bill_date\n",
    "provisional_draft_lines|draft_id\n",
    "provisional_draft_lines|position\n",
    "provisional_draft_lines|product_id\n",
    "provisional_draft_lines|quantity\n",
    "provisional_drafts|id\n",
);

const SCHEMA: &str = "SELECT m.name, p.name FROM sqlite_schema AS m
    JOIN pragma_table_xinfo(m.name) AS p
    WHERE m.type = 'table' OR m.type = 'view' ORDER BY m.name, p.cid;";

const ROWS: &str = "SELECT 'draft', id FROM provisional_drafts ORDER BY id;
    SELECT 'customer', draft_id, kind, customer_id, name
        FROM provisional_draft_customers ORDER BY draft_id;
    SELECT 'date', draft_id, bill_date FROM provisional_draft_dates ORDER BY draft_id;
    SELECT 'line', draft_id, position, product_id, quantity
        FROM provisional_draft_lines ORDER BY draft_id, position;
    SELECT 'crate', draft_id, definition FROM provisional_draft_crates ORDER BY draft_id;";

#[derive(Debug, Eq, PartialEq)]
pub(super) struct SqliteEvidence {
    columns: String,
    scope: String,
    rows: String,
    dump: String,
}

impl SqliteEvidence {
    pub fn read(path: &Path) -> Self {
        assert!(path.is_file(), "routed SQLite file must exist");
        let columns = query(path, SCHEMA);
        assert_eq!(columns, COLUMNS, "exact Manager schema allowlist");
        Self {
            columns,
            scope: query(path, "SELECT account_hex FROM manager_account_scope;"),
            rows: query(path, ROWS),
            dump: query(path, ".dump"),
        }
    }

    pub fn assert_empty(&self, account: Account) {
        self.assert_scope(account);
        assert!(
            self.rows.is_empty(),
            "expected no draft rows: {}",
            self.rows
        );
        assert!(!self.has_marker(account.marker()));
    }

    pub fn assert_draft(&self, spec: &DraftSpec) {
        self.assert_scope(spec.account);
        let [(first_id, first_qty), (second_id, second_qty)] = spec.lines;
        assert_eq!(
            self.rows,
            format!(
                "draft|21\ncustomer|21|2|{}|{}\ndate|21|{}\nline|21|0|{first_id}|{first_qty}\nline|21|1|{second_id}|{second_qty}\ncrate|21|{}\n",
                spec.buyer, spec.name, spec.date, spec.crate_sql
            ),
        );
        assert!(self.has_marker(spec.account.marker()));
    }

    pub fn has_marker(&self, marker: &str) -> bool {
        self.dump.contains(marker)
    }

    pub fn rows(&self) -> &str {
        &self.rows
    }

    fn assert_scope(&self, account: Account) {
        let key = account_hex(account);
        assert_eq!(self.scope, format!("{key}\n"));
        assert!(!self.has_marker(account.other().marker()));
        assert!(!self.dump.contains(&account_hex(account.other())));
        let lower = self.dump.to_ascii_lowercase();
        for excluded in ["cost", "margin", "profit", "invoice", "payment", "stock"] {
            assert!(
                !lower.contains(excluded),
                "unexpected {excluded} in Manager dump"
            );
        }
    }
}

fn account_hex(account: Account) -> String {
    use std::fmt::Write;
    let mut key = String::new();
    for byte in account.key().bytes() {
        write!(key, "{byte:02x}").unwrap();
    }
    key
}

fn query(path: &Path, query: &str) -> String {
    let home = path.parent().expect("database directory");
    let result = Command::new("sqlite3")
        .env_clear()
        .env("PATH", env::var_os("PATH").expect("PATH for sqlite3"))
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("TMPDIR", home)
        .args([
            "-readonly",
            "-batch",
            "-bail",
            "-list",
            "-noheader",
            "-separator",
            "|",
        ])
        .arg(path)
        .arg(query)
        .output()
        .expect("sqlite3 is required for Manager evidence");
    assert!(
        result.status.success(),
        "read-only SQLite failed: {result:?}"
    );
    String::from_utf8(result.stdout).expect("text SQLite projection")
}

pub(super) fn databases(root: &Path) -> Vec<PathBuf> {
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
