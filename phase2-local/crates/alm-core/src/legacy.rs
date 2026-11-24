//! The HTML's backup file (`backupJSON`, ALM_Business_Manager.html L10016: the whole `DB`
//! object) read for a migration rehearsal. `restoreJSON` (L10021) requires `products`, then runs
//! `migrate()` (L464–L620); [`LegacyBackup::parse`] does the same, deterministically: the
//! clock, today's date and the `uid` stream are inputs, not the device's. Every collection the
//! HTML keeps is read and kept with its raw record (provenance); the control totals and the
//! record views are computed from it by the core. Synthetic data only (R-8 rehearsal).
use serde_json::{Map, Value};

mod digest;
pub mod import;
mod js;
mod migrate;
mod records;

pub use digest::sha256_hex;
pub use records::{
    LegacyCashDay, LegacyComponent, LegacyCrate, LegacyCustomer, LegacyGodown, LegacyLine,
    LegacyLotSource, LegacyPayment, LegacyProduct, LegacySale, LegacyStockEntry, LegacyStockLog,
    LegacySupplier,
};

/// Every collection `freshDB()` holds, in its order; `stock` and `meta` are objects.
pub const COLLECTIONS: [&str; 17] = [
    "products",
    "customers",
    "suppliers",
    "sales",
    "purchases",
    "purchaseOrders",
    "expenses",
    "payments",
    "stock",
    "crates",
    "priceSuppliers",
    "gstBills",
    "stockLog",
    "log",
    "godown",
    "deleted",
    "meta",
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LegacyError {
    /// Not JSON, or not a JSON object.
    NotJson,
    /// `restoreJSON`'s check: `if (!d.products) throw 0`.
    NoProducts,
}

/// The clock `migrate()` reads, given explicitly.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MigrateClock<'a> {
    /// `Date.now()`.
    pub now_ms: u64,
    /// `todayISO()`.
    pub today: &'a str,
}

/// A parsed and migrated legacy backup.
#[derive(Clone, Debug, PartialEq)]
pub struct LegacyBackup {
    /// The `DB` object after `migrate()`.
    pub db: Map<String, Value>,
    /// SHA-256 of the file's bytes, lowercase hex.
    pub digest: String,
    /// The `meta.schema` the file carried before `migrate()` (0 when absent).
    pub source_schema: u64,
    /// What `migrate()` changed, in order (one line per fix-up applied).
    pub fixes: Vec<String>,
}

impl LegacyBackup {
    /// Reads a backup file as `restoreJSON` does and runs `migrate()` on it.
    ///
    /// # Errors
    /// Rejects text that is not a JSON object, or one without a truthy `products`.
    pub fn parse(bytes: &[u8], clock: MigrateClock<'_>) -> Result<Self, LegacyError> {
        let value: Value = serde_json::from_slice(bytes).map_err(|_| LegacyError::NotJson)?;
        let Value::Object(mut db) = value else {
            return Err(LegacyError::NotJson);
        };
        if !js::truthy(db.get("products").unwrap_or(&Value::Null)) {
            return Err(LegacyError::NoProducts);
        }
        let source_schema = db
            .get("meta")
            .and_then(|m| m.get("schema"))
            .map_or(0.0, js::num)
            .max(0.0) as u64;
        let fixes = migrate::migrate(&mut db, clock);
        Ok(Self {
            db,
            digest: sha256_hex(bytes),
            source_schema,
            fixes,
        })
    }

    /// One collection's records (an empty slice for `stock` and `meta`, which are objects).
    pub fn collection(&self, name: &str) -> &[Value] {
        self.db
            .get(name)
            .and_then(Value::as_array)
            .map_or(&[], Vec::as_slice)
    }

    pub fn meta(&self) -> &Map<String, Value> {
        static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
        self.db
            .get("meta")
            .and_then(Value::as_object)
            .unwrap_or_else(|| EMPTY.get_or_init(Map::new))
    }

    /// Record counts per collection, in [`COLLECTIONS`] order (`stock` counts its keys, `meta`
    /// is one record).
    pub fn counts(&self) -> Vec<(&'static str, usize)> {
        COLLECTIONS
            .iter()
            .map(|&name| {
                let n = match self.db.get(name) {
                    Some(Value::Array(rows)) => rows.len(),
                    Some(Value::Object(map)) if name == "stock" => map.len(),
                    Some(Value::Object(_)) => 1,
                    _ => 0,
                };
                (name, n)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests;
