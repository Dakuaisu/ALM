use std::fmt::Write as _;
use std::path::Path;

use rusqlite::fallible_iterator::FallibleIterator;
use rusqlite::types::ValueRef;
use rusqlite::{Batch, Connection, OpenFlags};

/// Runs every statement in `sql` and renders rows like `sqlite3 -list -noheader`:
/// `|`-separated columns, NULL as an empty field, one row per line.
pub(crate) fn sqlite_list(conn: &Connection, sql: &str) -> rusqlite::Result<String> {
    let mut output = String::new();
    let mut batch = Batch::new(conn, sql);
    while let Some(mut statement) = batch.next()? {
        let columns = statement.column_count();
        let mut rows = statement.query([])?;
        while let Some(row) = rows.next()? {
            for index in 0..columns {
                if index > 0 {
                    output.push('|');
                }
                match row.get_ref(index)? {
                    ValueRef::Null => {}
                    ValueRef::Integer(value) => write!(output, "{value}").unwrap(),
                    ValueRef::Real(value) => write!(output, "{value}").unwrap(),
                    ValueRef::Text(bytes) | ValueRef::Blob(bytes) => {
                        output.push_str(&String::from_utf8_lossy(bytes));
                    }
                }
            }
            output.push('\n');
        }
    }
    Ok(output)
}

pub(crate) fn open_read_only(path: &Path) -> rusqlite::Result<Connection> {
    Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
}
