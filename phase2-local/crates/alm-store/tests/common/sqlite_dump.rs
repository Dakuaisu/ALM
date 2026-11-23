use std::fmt::Write as _;

use rusqlite::Connection;

/// Logical text dump in the spirit of `sqlite3 .dump`: schema SQL plus one quoted
/// `INSERT` per row, tables first and then indexes, triggers and views.
pub(crate) fn sqlite_dump(conn: &Connection) -> rusqlite::Result<String> {
    let mut output = String::from("BEGIN TRANSACTION;\n");
    let objects = conn
        .prepare(
            "SELECT type, name, sql FROM sqlite_schema
             WHERE sql IS NOT NULL AND name NOT LIKE 'sqlite_%'
             ORDER BY type != 'table', rowid",
        )?
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for (kind, name, sql) in objects {
        writeln!(output, "{sql};").unwrap();
        if kind != "table" {
            continue;
        }
        let table = name.replace('"', "\"\"");
        let columns = conn
            .prepare("SELECT name FROM pragma_table_info(?1) ORDER BY cid")?
            .query_map([&name], |row| row.get::<_, String>(0))?
            .map(|column| {
                column.map(|column| format!("quote(\"{}\")", column.replace('"', "\"\"")))
            })
            .collect::<rusqlite::Result<Vec<_>>>()?
            .join(" || ',' || ");
        let mut rows = conn.prepare(&format!("SELECT {columns} FROM \"{table}\""))?;
        let mut rows = rows.query([])?;
        while let Some(row) = rows.next()? {
            let values: String = row.get(0)?;
            writeln!(output, "INSERT INTO \"{table}\" VALUES({values});").unwrap();
        }
    }
    output.push_str("COMMIT;\n");
    Ok(output)
}
