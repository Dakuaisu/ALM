//! Standalone GST bills in the device database: the local half of the HTML's `persist()`.
use std::collections::BTreeMap;
use std::num::NonZeroU32;
use std::path::Path;

use rusqlite::Row;

use alm_core::confirmation::{
    GstBill, GstBillId, GstBillLedger, GstBillLine, GstBillStore, GstUnit,
};
use alm_core::{BillDate, Money, ProductId};

use crate::DraftStorageError;

use super::{SqliteDraftStore, integer, text};

// Created only when the GST store is opened, so draft-only files keep their schema.
const GST_TABLES: &str = "
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

const SCHEMA_VERSION: i64 = 1;

/// Standalone GST bills and deletion markers in a device database file. Role checks stay in
/// `alm-core`; this type only stores what the fixture has already validated.
pub struct GstBillDatabase {
    store: SqliteDraftStore,
}

impl GstBillDatabase {
    /// Opens the GST tables in the trusted synthetic account's database under `device_root`.
    ///
    /// # Errors
    /// Rejects invalid IDs, unsafe locations, files bound to another account and newer schemas.
    pub fn open_scoped(device_root: &Path, account: &str) -> Result<Self, DraftStorageError> {
        Self::with_tables(SqliteDraftStore::open_scoped(device_root, account)?)
    }

    /// Opens the GST tables in an explicit legacy, single-account database file.
    ///
    /// # Errors
    /// Returns a storage error if SQLite cannot initialize the file.
    pub fn open(path: &Path) -> Result<Self, DraftStorageError> {
        Self::with_tables(SqliteDraftStore::open(path)?)
    }

    fn with_tables(mut store: SqliteDraftStore) -> Result<Self, DraftStorageError> {
        let tx = store.begin()?;
        tx.execute_batch(GST_TABLES)?;
        let version: i64 = tx.query_row(
            "SELECT max(version) FROM standalone_gst_schema",
            [],
            |row| row.get(0),
        )?;
        if version != SCHEMA_VERSION {
            return Err(DraftStorageError::InvalidOutput);
        }
        tx.commit()?;
        Ok(Self { store })
    }
}

fn unit_text(unit: GstUnit) -> &'static str {
    match unit {
        GstUnit::Case => "Case",
        GstUnit::Piece => "Piece",
    }
}

fn required(row: &Row<'_>, index: usize) -> Result<String, DraftStorageError> {
    text(row, index)?.ok_or(DraftStorageError::InvalidOutput)
}

fn decimal(row: &Row<'_>, index: usize) -> Result<u64, DraftStorageError> {
    required(row, index)?
        .parse()
        .map_err(|_| DraftStorageError::InvalidOutput)
}

fn small(row: &Row<'_>, index: usize) -> Result<u32, DraftStorageError> {
    integer(row, index)?
        .and_then(|value| u32::try_from(value).ok())
        .ok_or(DraftStorageError::InvalidOutput)
}

fn money_at(row: &Row<'_>, index: usize) -> Result<Money, DraftStorageError> {
    integer(row, index)?
        .map(Money::from_minor_units)
        .ok_or(DraftStorageError::InvalidOutput)
}

fn read_bill(row: &Row<'_>) -> Result<GstBill, DraftStorageError> {
    let date: BillDate = required(row, 2)?
        .parse()
        .map_err(|_| DraftStorageError::InvalidOutput)?;
    Ok(GstBill {
        id: GstBillId(decimal(row, 0)?),
        number: alm_core::confirmation::GstBillNumber(decimal(row, 1)?),
        date,
        customer_id: text(row, 3)?,
        customer: required(row, 4)?,
        gstin: text(row, 5)?,
        cust_addr: required(row, 6)?,
        cust_phone: required(row, 7)?,
        cust_email: required(row, 8)?,
        cust_state: required(row, 9)?,
        cust_state_code: required(row, 10)?,
        pos_state: required(row, 11)?,
        pos_code: required(row, 12)?,
        pay_mode: required(row, 13)?,
        pay_terms: required(row, 14)?,
        reverse_charge: required(row, 15)?,
        transporter: required(row, 16)?,
        vehicle_no: required(row, 17)?,
        lr_no: required(row, 18)?,
        eway_number: text(row, 19)?,
        terms_delivery: required(row, 20)?,
        dispatch: required(row, 21)?,
        cancelled: match integer(row, 22)? {
            Some(0) => false,
            Some(1) => true,
            _ => return Err(DraftStorageError::InvalidOutput),
        },
        created: decimal(row, 23)?,
        lines: Vec::new(),
    })
}

fn read_line(row: &Row<'_>) -> Result<(u64, i64, GstBillLine), DraftStorageError> {
    let nonzero =
        |index| NonZeroU32::new(small(row, index)?).ok_or(DraftStorageError::InvalidOutput);
    let product = text(row, 3)?
        .map(|id| id.parse().map(ProductId::new))
        .transpose()
        .map_err(|_| DraftStorageError::InvalidOutput)?;
    let unit = match required(row, 5)?.as_str() {
        "Case" => GstUnit::Case,
        "Piece" => GstUnit::Piece,
        _ => return Err(DraftStorageError::InvalidOutput),
    };
    Ok((
        decimal(row, 0)?,
        integer(row, 1)?.ok_or(DraftStorageError::InvalidOutput)?,
        GstBillLine {
            item: required(row, 2)?,
            product,
            hsn: required(row, 4)?,
            unit,
            quantity: nonzero(6)?,
            inclusive_rate: money_at(row, 7)?,
            gst_basis_points: small(row, 8)?,
            packet_qty: nonzero(9)?,
            amount: money_at(row, 10)?,
        },
    ))
}

impl GstBillStore for GstBillDatabase {
    type Error = DraftStorageError;

    fn load_gst_bills(&self) -> Result<GstBillLedger, DraftStorageError> {
        let conn = &self.store.conn;
        let mut bills = BTreeMap::new();
        let mut query = conn.prepare(
            "SELECT id, number, date, customer_id, customer, gstin, cust_addr, cust_phone,
                        cust_email, cust_state, cust_state_code, pos_state, pos_code, pay_mode,
                        pay_terms, reverse_charge, transporter, vehicle_no, lr_no, eway_number,
                        terms_delivery, dispatch, cancelled, created
                 FROM standalone_gst_bills",
        )?;
        let mut rows = query.query([])?;
        while let Some(row) = rows.next()? {
            let bill = read_bill(row)?;
            bills.insert(bill.id, bill);
        }
        let mut query = conn.prepare(
            "SELECT bill_id, position, item, product_id, hsn, unit, quantity, inclusive_rate,
                        gst_basis_points, packet_qty, amount
                 FROM standalone_gst_bill_lines ORDER BY bill_id, position",
        )?;
        let mut rows = query.query([])?;
        while let Some(row) = rows.next()? {
            let (bill, position, line) = read_line(row)?;
            let bill = bills
                .get_mut(&GstBillId(bill))
                .ok_or(DraftStorageError::InvalidOutput)?;
            if usize::try_from(position) != Ok(bill.lines.len()) {
                return Err(DraftStorageError::InvalidOutput);
            }
            bill.lines.push(line);
        }
        let mut query = conn.prepare("SELECT bill_id FROM standalone_gst_deleted")?;
        let mut rows = query.query([])?;
        let mut deleted = Vec::new();
        while let Some(row) = rows.next()? {
            deleted.push(GstBillId(decimal(row, 0)?));
        }
        deleted.sort();
        Ok(GstBillLedger {
            bills: bills.into_values().collect(),
            deleted,
        })
    }

    fn put_gst_bill(&mut self, bill: &GstBill) -> Result<(), DraftStorageError> {
        let id = bill.id.0.to_string();
        let tx = self.store.begin()?;
        tx.execute(
            "INSERT INTO standalone_gst_bills VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                 ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24)
             ON CONFLICT(id) DO UPDATE SET
                 number = excluded.number, date = excluded.date,
                 customer_id = excluded.customer_id, customer = excluded.customer,
                 gstin = excluded.gstin, cust_addr = excluded.cust_addr,
                 cust_phone = excluded.cust_phone, cust_email = excluded.cust_email,
                 cust_state = excluded.cust_state, cust_state_code = excluded.cust_state_code,
                 pos_state = excluded.pos_state, pos_code = excluded.pos_code,
                 pay_mode = excluded.pay_mode, pay_terms = excluded.pay_terms,
                 reverse_charge = excluded.reverse_charge, transporter = excluded.transporter,
                 vehicle_no = excluded.vehicle_no, lr_no = excluded.lr_no,
                 eway_number = excluded.eway_number, terms_delivery = excluded.terms_delivery,
                 dispatch = excluded.dispatch, cancelled = excluded.cancelled,
                 created = excluded.created",
            rusqlite::params![
                id,
                bill.number.0.to_string(),
                bill.date.to_string(),
                bill.customer_id,
                bill.customer,
                bill.gstin,
                bill.cust_addr,
                bill.cust_phone,
                bill.cust_email,
                bill.cust_state,
                bill.cust_state_code,
                bill.pos_state,
                bill.pos_code,
                bill.pay_mode,
                bill.pay_terms,
                bill.reverse_charge,
                bill.transporter,
                bill.vehicle_no,
                bill.lr_no,
                bill.eway_number,
                bill.terms_delivery,
                bill.dispatch,
                bill.cancelled,
                bill.created.to_string(),
            ],
        )?;
        tx.execute(
            "DELETE FROM standalone_gst_bill_lines WHERE bill_id = ?1",
            [&id],
        )?;
        {
            let mut insert = tx.prepare(
                "INSERT INTO standalone_gst_bill_lines VALUES
                     (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            )?;
            for (position, line) in bill.lines.iter().enumerate() {
                let position =
                    i64::try_from(position).map_err(|_| DraftStorageError::TooManyLines)?;
                insert.execute(rusqlite::params![
                    id,
                    position,
                    line.item,
                    line.product.map(|product| product.value().to_string()),
                    line.hsn,
                    unit_text(line.unit),
                    line.quantity.get(),
                    line.inclusive_rate.minor_units(),
                    line.gst_basis_points,
                    line.packet_qty.get(),
                    line.amount.minor_units(),
                ])?;
            }
        }
        Ok(tx.commit()?)
    }

    fn delete_gst_bill(&mut self, id: GstBillId) -> Result<(), DraftStorageError> {
        let id = id.0.to_string();
        let tx = self.store.begin()?;
        tx.execute(
            "DELETE FROM standalone_gst_bill_lines WHERE bill_id = ?1",
            [&id],
        )?;
        tx.execute("DELETE FROM standalone_gst_bills WHERE id = ?1", [&id])?;
        tx.execute(
            "INSERT INTO standalone_gst_deleted (bill_id) VALUES (?1) ON CONFLICT DO NOTHING",
            [&id],
        )?;
        Ok(tx.commit()?)
    }
}
