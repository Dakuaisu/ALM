use std::fmt;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::thread;

use crate::drafts::{DraftId, ProvisionalDraft};
use crate::{DraftCustomer, crate_row_codec};

mod customer;
mod draft_rows;
mod pending;
mod schema;
mod scoped;

/// Failure to open, save, or reopen a local SQLite draft.
#[derive(Debug)]
#[non_exhaustive]
pub enum DraftStorageError {
    /// The database path must be absolute and contain no NUL bytes.
    InvalidPath,
    /// A synthetic account ID is empty, too long, or contains disallowed characters.
    InvalidAccountScope,
    /// An existing scoped file is not bound to the selected synthetic account.
    AccountStoreMismatch,
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
            Self::InvalidPath => f.write_str("expected a safe absolute SQLite path or device root"),
            Self::InvalidAccountScope => f.write_str(
                "expected a synthetic account ID using ASCII letters, digits, '-' or '_'",
            ),
            Self::AccountStoreMismatch => {
                f.write_str("database is not bound to the selected account")
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
            | Self::InvalidAccountScope
            | Self::AccountStoreMismatch
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
        let sql = format!(
            "BEGIN IMMEDIATE;\n{}\n{}\nCOMMIT;",
            schema::DRAFT_TABLES,
            schema::OWNER_PENDING_TABLE
        );
        store.run(sql.as_bytes(), false)?;
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
             DELETE FROM provisional_draft_lines WHERE draft_id = '{id}';
             DELETE FROM provisional_draft_crates WHERE draft_id = '{id}';
             DELETE FROM provisional_draft_dates WHERE draft_id = '{id}';"
        )?;
        if let Some(date) = draft.bill_date() {
            writeln!(
                sql,
                "INSERT INTO provisional_draft_dates VALUES ('{id}', '{date}');"
            )?;
        }
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
        if let Some(row) = draft.crate_row() {
            writeln!(
                sql,
                "INSERT INTO provisional_draft_crates VALUES ('{id}', {});",
                customer::sql_text(&crate_row_codec::encode(row))
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
            "SELECT b.bill_date, COALESCE(c.kind, 0), hex(c.customer_id), hex(c.name), k.definition, l.position, l.product_id, l.quantity
             FROM provisional_drafts AS d
             LEFT JOIN provisional_draft_dates AS b ON b.draft_id = d.id
             LEFT JOIN provisional_draft_customers AS c ON c.draft_id = d.id
             LEFT JOIN provisional_draft_lines AS l ON l.draft_id = d.id
             LEFT JOIN provisional_draft_crates AS k ON k.draft_id = d.id
             WHERE d.id = '{}'
             ORDER BY l.position;\n",
            id.value()
        );
        let output = self.run(sql.as_bytes(), true)?;
        draft_rows::decode(id, &output)
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
