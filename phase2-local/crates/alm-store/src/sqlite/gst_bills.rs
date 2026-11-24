//! Standalone GST bills in the device database: the local half of the HTML's `persist()`.
use std::collections::BTreeMap;
use std::path::Path;

use alm_core::confirmation::{GstBill, GstBillId, GstBillLedger, GstBillStore};

use crate::DraftStorageError;

use super::SqliteDraftStore;

mod rows;
mod schema;
use rows::{decimal, read_bill, read_line, unit_text};
use schema::GST_TABLES;

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
