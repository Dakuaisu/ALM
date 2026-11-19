use std::fmt;
use std::io::{self, Write};
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::thread;

use crate::DraftCustomer;
use crate::drafts::{DraftId, DraftLine, ProductId, ProvisionalDraft};

mod customer;
mod pending;

// IDs use canonical decimal text because SQLite integers are signed 64-bit.
const SCHEMA: &str = "
BEGIN IMMEDIATE;
CREATE TABLE IF NOT EXISTS provisional_drafts (
    id TEXT PRIMARY KEY NOT NULL CHECK (
        length(id) BETWEEN 1 AND 20 AND instr(id, char(0)) = 0
        AND id NOT GLOB '*[^0-9]*' AND (id = '0' OR id NOT GLOB '0*')
        AND (length(id) < 20 OR id <= '18446744073709551615')
    )
) STRICT;
CREATE TABLE IF NOT EXISTS provisional_draft_lines (
    draft_id TEXT NOT NULL REFERENCES provisional_drafts(id),
    position INTEGER NOT NULL CHECK (position >= 0),
    product_id TEXT NOT NULL CHECK (
        length(product_id) BETWEEN 1 AND 20 AND instr(product_id, char(0)) = 0
        AND product_id NOT GLOB '*[^0-9]*'
        AND (product_id = '0' OR product_id NOT GLOB '0*')
        AND (length(product_id) < 20 OR product_id <= '18446744073709551615')
    ),
    quantity INTEGER NOT NULL CHECK (quantity BETWEEN 1 AND 4294967295),
    PRIMARY KEY (draft_id, position)
) STRICT;
CREATE TABLE IF NOT EXISTS provisional_draft_customers (
    draft_id TEXT PRIMARY KEY NOT NULL REFERENCES provisional_drafts(id),
    kind INTEGER NOT NULL CHECK (kind IN (1, 2)),
    customer_id TEXT,
    name TEXT NOT NULL,
    CHECK ((kind = 1 AND customer_id IS NULL) OR (kind = 2 AND customer_id IS NOT NULL))
) STRICT;
CREATE TABLE IF NOT EXISTS pending_owner_confirmations (
    request_id TEXT PRIMARY KEY NOT NULL,
    snapshot TEXT NOT NULL
) STRICT;
COMMIT;
";

/// Failure to open, save, or reopen a local SQLite draft.
#[derive(Debug)]
#[non_exhaustive]
pub enum DraftStorageError {
    /// The database path must be absolute and contain no NUL bytes.
    InvalidPath,
    /// An I/O error occurred while running or communicating with SQLite.
    Io(io::Error),
    /// SQLite exited unsuccessfully.
    SqliteFailed {
        /// The SQLite child process's exit status.
        status: ExitStatus,
        /// The SQLite child process's diagnostic output.
        stderr: String,
    },
    /// The stored rows could not be decoded as a draft.
    InvalidOutput,
    /// A line position exceeds SQLite's integer range.
    TooManyLines,
}

impl fmt::Display for DraftStorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPath => {
                f.write_str("expected an absolute SQLite file path without NUL bytes")
            }
            Self::Io(err) => write!(f, "SQLite CLI I/O failed: {err}"),
            Self::SqliteFailed { status, stderr } => {
                write!(f, "sqlite3 failed ({status}): {}", stderr.trim_end())
            }
            Self::InvalidOutput => f.write_str("invalid SQLite draft output"),
            Self::TooManyLines => f.write_str("too many draft lines for SQLite"),
        }
    }
}

impl std::error::Error for DraftStorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::InvalidPath
            | Self::SqliteFailed { .. }
            | Self::InvalidOutput
            | Self::TooManyLines => None,
        }
    }
}

impl From<io::Error> for DraftStorageError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

pub(crate) struct SqliteDraftStore {
    path: PathBuf,
}

impl SqliteDraftStore {
    pub(crate) fn open(path: &Path) -> Result<Self, DraftStorageError> {
        if !path.is_absolute() || path.as_os_str().as_encoded_bytes().contains(&0) {
            return Err(DraftStorageError::InvalidPath);
        }
        let store = Self { path: path.into() };
        store.run(SCHEMA.as_bytes(), false)?;
        Ok(store)
    }

    pub(crate) fn save(&self, draft: &ProvisionalDraft) -> Result<(), DraftStorageError> {
        let id = draft.id().value();
        let mut sql = Vec::new();
        writeln!(
            sql,
            "PRAGMA foreign_keys = ON;
             PRAGMA synchronous = FULL;
             BEGIN IMMEDIATE;
             INSERT INTO provisional_drafts (id) VALUES ('{id}') ON CONFLICT(id) DO NOTHING;
             DELETE FROM provisional_draft_customers WHERE draft_id = '{id}';
             DELETE FROM provisional_draft_lines WHERE draft_id = '{id}';"
        )?;
        match draft.customer() {
            DraftCustomer::Unassigned => {}
            DraftCustomer::Typed(name) => writeln!(
                sql,
                "INSERT INTO provisional_draft_customers VALUES ('{id}', 1, NULL, {});",
                customer::sql_text(name)
            )?,
            DraftCustomer::Selected {
                id: customer_id,
                name,
            } => writeln!(
                sql,
                "INSERT INTO provisional_draft_customers VALUES ('{id}', 2, {}, {});",
                customer::sql_text(customer_id.value()),
                customer::sql_text(name)
            )?,
        }
        for (position, line) in draft.lines().iter().enumerate() {
            let position = i64::try_from(position).map_err(|_| DraftStorageError::TooManyLines)?;
            let product = line.product_id().value();
            let quantity = line.quantity().get();
            writeln!(
                sql,
                "INSERT INTO provisional_draft_lines (draft_id, position, product_id, quantity)
                 VALUES ('{id}', {position}, '{product}', {quantity});"
            )?;
        }
        sql.extend_from_slice(b"COMMIT;\n");
        self.run(&sql, false).map(|_| ())
    }

    pub(crate) fn reopen(
        &self,
        id: DraftId,
    ) -> Result<Option<ProvisionalDraft>, DraftStorageError> {
        let sql = format!(
            "SELECT COALESCE(c.kind, 0), hex(c.customer_id), hex(c.name), l.position, l.product_id, l.quantity
             FROM provisional_drafts AS d
             LEFT JOIN provisional_draft_customers AS c ON c.draft_id = d.id
             LEFT JOIN provisional_draft_lines AS l ON l.draft_id = d.id
             WHERE d.id = '{}'
             ORDER BY l.position;\n",
            id.value()
        );
        let output = self.run(sql.as_bytes(), true)?;
        let output = std::str::from_utf8(&output).map_err(|_| DraftStorageError::InvalidOutput)?;
        let mut rows = output.lines();
        let Some(first) = rows.next() else {
            return Ok(None);
        };
        let mut lines = Vec::new();
        let mut customer = None;
        for (position, row) in std::iter::once(first).chain(rows).enumerate() {
            let mut columns = row.split('|');
            let kind = columns.next().ok_or(DraftStorageError::InvalidOutput)?;
            let customer_id = columns.next().ok_or(DraftStorageError::InvalidOutput)?;
            let name = columns.next().ok_or(DraftStorageError::InvalidOutput)?;
            let decoded = customer::decode(kind, customer_id, name)?;
            if customer
                .as_ref()
                .is_some_and(|existing| existing != &decoded)
            {
                return Err(DraftStorageError::InvalidOutput);
            }
            customer = Some(decoded);
            let (Some(index), Some(product), Some(quantity), None) = (
                columns.next(),
                columns.next(),
                columns.next(),
                columns.next(),
            ) else {
                return Err(DraftStorageError::InvalidOutput);
            };
            if index == "NULL" && product == "NULL" && quantity == "NULL" && position == 0 {
                continue;
            }
            if index.parse::<usize>() != Ok(position) {
                return Err(DraftStorageError::InvalidOutput);
            }
            let product = product
                .parse()
                .map_err(|_| DraftStorageError::InvalidOutput)?;
            let quantity = quantity
                .parse::<NonZeroU32>()
                .map_err(|_| DraftStorageError::InvalidOutput)?;
            lines.push(DraftLine::new(ProductId::new(product), quantity));
        }
        Ok(Some(ProvisionalDraft::new(id, lines).with_customer(
            customer.ok_or(DraftStorageError::InvalidOutput)?,
        )))
    }

    fn run(&self, sql: &[u8], read_only: bool) -> Result<Vec<u8>, DraftStorageError> {
        // Bail before COMMIT on any SQL failure; closing the connection rolls back.
        let mut command = Command::new("sqlite3");
        command.args([
            "-batch",
            "-bail",
            "-init",
            "/dev/null",
            "-list",
            "-noheader",
            "-separator",
            "|",
            "-nullvalue",
            "NULL",
        ]);
        if read_only {
            command.arg("-readonly");
        }
        let mut child = command
            .arg(&self.path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // Feed SQL via stdin rather than argv, draining output concurrently.
        let stdin = child.stdin.take();
        let (output, written) = thread::scope(|scope| {
            let writer = thread::Builder::new().spawn_scoped(scope, move || {
                let mut stdin = stdin.ok_or_else(|| io::Error::other("missing SQLite stdin"))?;
                stdin.write_all(sql)
            });
            let output = child.wait_with_output();
            let written = writer.and_then(|worker| {
                worker
                    .join()
                    .map_err(|_| io::Error::other("SQLite stdin writer panicked"))?
            });
            (output, written)
        });
        let output = output?;
        if !output.status.success() {
            return Err(DraftStorageError::SqliteFailed {
                status: output.status,
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }
        written?;
        Ok(output.stdout)
    }
}
