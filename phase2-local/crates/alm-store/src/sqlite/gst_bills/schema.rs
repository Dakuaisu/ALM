//! The standalone GST tables, created only when the GST store is opened, so draft-only files
//! keep their schema.
pub(super) const GST_TABLES: &str = "
CREATE TABLE IF NOT EXISTS standalone_gst_schema (
    version INTEGER PRIMARY KEY NOT NULL
) STRICT;
INSERT OR IGNORE INTO standalone_gst_schema (version) VALUES (1);
CREATE TABLE IF NOT EXISTS standalone_gst_bills (
    id TEXT PRIMARY KEY NOT NULL CHECK (id NOT GLOB '*[^0-9]*' AND length(id) BETWEEN 1 AND 20),
    number TEXT NOT NULL CHECK (number NOT GLOB '*[^0-9]*' AND length(number) BETWEEN 1 AND 20),
    date TEXT NOT NULL,
    customer_id TEXT,
    customer TEXT NOT NULL,
    gstin TEXT,
    cust_addr TEXT NOT NULL,
    cust_phone TEXT NOT NULL,
    cust_email TEXT NOT NULL,
    cust_state TEXT NOT NULL,
    cust_state_code TEXT NOT NULL,
    pos_state TEXT NOT NULL,
    pos_code TEXT NOT NULL,
    pay_mode TEXT NOT NULL,
    pay_terms TEXT NOT NULL,
    reverse_charge TEXT NOT NULL,
    transporter TEXT NOT NULL,
    vehicle_no TEXT NOT NULL,
    lr_no TEXT NOT NULL,
    eway_number TEXT,
    terms_delivery TEXT NOT NULL,
    dispatch TEXT NOT NULL,
    cancelled INTEGER NOT NULL CHECK (cancelled IN (0, 1)),
    created TEXT NOT NULL CHECK (created NOT GLOB '*[^0-9]*' AND length(created) BETWEEN 1 AND 20)
) STRICT;
CREATE TABLE IF NOT EXISTS standalone_gst_bill_lines (
    bill_id TEXT NOT NULL REFERENCES standalone_gst_bills(id),
    position INTEGER NOT NULL CHECK (position >= 0),
    item TEXT NOT NULL CHECK (length(item) > 0),
    product_id TEXT CHECK (product_id IS NULL OR product_id NOT GLOB '*[^0-9]*'),
    hsn TEXT NOT NULL,
    unit TEXT NOT NULL CHECK (unit IN ('Case', 'Piece')),
    quantity INTEGER NOT NULL CHECK (quantity BETWEEN 1 AND 4294967295),
    inclusive_rate INTEGER NOT NULL,
    gst_basis_points INTEGER NOT NULL CHECK (gst_basis_points BETWEEN 0 AND 4294967295),
    packet_qty INTEGER NOT NULL CHECK (packet_qty BETWEEN 1 AND 4294967295),
    amount INTEGER NOT NULL,
    PRIMARY KEY (bill_id, position)
) STRICT;
CREATE TABLE IF NOT EXISTS standalone_gst_deleted (
    bill_id TEXT PRIMARY KEY NOT NULL CHECK (bill_id NOT GLOB '*[^0-9]*')
) STRICT;
";
