// IDs use canonical decimal text because SQLite integers are signed 64-bit.
pub(super) const SCHEMA: &str = "
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
CREATE TABLE IF NOT EXISTS provisional_draft_crates (
    draft_id TEXT PRIMARY KEY NOT NULL REFERENCES provisional_drafts(id),
    definition TEXT NOT NULL CHECK (
        length(definition) > 0 AND instr(definition, char(0)) = 0
        AND definition NOT GLOB '*[^0-9,;]*'
    )
) STRICT;
CREATE TABLE IF NOT EXISTS provisional_draft_dates (
    draft_id TEXT PRIMARY KEY NOT NULL REFERENCES provisional_drafts(id),
    bill_date TEXT NOT NULL CHECK (
        length(bill_date) = 10 AND instr(bill_date, char(0)) = 0
        AND bill_date GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'
    )
) STRICT;
COMMIT;
";
