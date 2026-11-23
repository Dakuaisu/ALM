use serde::{Deserialize, Serialize};

use super::model::{
    Collection, Cursor, Db, DeleteTick, Envelope, Origin, Row, Stamp, Supplier, Tick,
};
use super::price_columns::PriceSupplierEntry;
use super::receipt_rows::Rupees;

#[derive(Clone)]
pub(super) struct Replica {
    pub db: Db,
    pub cursor: Cursor,
}

pub(super) struct Page {
    pub max_seq: u64,
    pub records: Vec<Envelope>,
}

impl Replica {
    pub fn pull(&mut self, page: &Page, device: &str) -> Vec<bool> {
        let outcomes = page
            .records
            .iter()
            .map(|envelope| self.db.apply(envelope, device, Origin::Pull))
            .collect();
        self.cursor.seq = self.cursor.seq.max(page.max_seq);
        outcomes
    }

    pub fn acknowledge_push(&mut self, accepted_batch: &[Envelope]) {
        if let Some(max) = accepted_batch.iter().map(|row| row.m).max() {
            self.cursor.pushed_m = self.cursor.pushed_m.max(max);
        }
    }

    pub fn restore_json(&mut self, bytes: &[u8]) -> serde_json::Result<()> {
        self.db = serde_json::from_slice(bytes)?;
        self.cursor.pushed_m = 0;
        Ok(())
    }

    pub fn rebuild(&mut self, bytes: &[u8], snapshot_seq: u64, now: u64) -> serde_json::Result<()> {
        self.db = serde_json::from_slice(bytes)?;
        self.cursor = Cursor {
            seq: snapshot_seq,
            pushed_m: now,
        };
        Ok(())
    }

    pub fn accept_sheet_copy(&mut self, bytes: &[u8]) -> serde_json::Result<()> {
        let SheetCopy { db } = serde_json::from_slice(bytes)?;
        self.db = db;
        Ok(())
    }
}

#[derive(Deserialize, Serialize)]
pub(super) struct SheetCopy {
    pub db: Db,
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
enum SheetAction {
    Push,
}

#[derive(Serialize)]
pub(super) struct SheetPush<'a> {
    action: SheetAction,
    db: &'a Db,
}

impl<'a> SheetPush<'a> {
    pub const fn new(db: &'a Db) -> Self {
        Self {
            action: SheetAction::Push,
            db,
        }
    }
}

impl Db {
    pub fn columns(&self) -> Vec<String> {
        self.supplier_columns()
            .iter()
            .map(PriceSupplierEntry::header)
            .collect()
    }

    fn supplier_columns(&self) -> Vec<PriceSupplierEntry> {
        let mut columns = self.price_suppliers.clone();
        for row in &self.suppliers {
            let name = row.value.data.name.trim();
            if !name.is_empty() && !columns.iter().any(|column| column.is_name(name)) {
                columns.push(name.into());
            }
        }
        columns
    }

    pub fn add_pricing_supplier(&mut self, id: &str, input: &str, tick: Tick<'_>) -> bool {
        let name = input.trim();
        if name.is_empty()
            || self
                .supplier_columns()
                .iter()
                .any(|column| column.is_name(name))
        {
            return false;
        }
        let mut stamp = Stamp::default();
        stamp.write(None, tick);
        self.price_suppliers.push(name.into());
        self.suppliers
            .push(Row::new(id, Supplier { name: name.into() }, stamp));
        true
    }

    pub fn delete_contact(&mut self, id: &str, tick: DeleteTick<'_>) {
        let name = self
            .suppliers
            .iter()
            .find(|row| row.id == id)
            .expect("synthetic selected contact")
            .value
            .data
            .name
            .clone();
        self.remove(Collection::Suppliers, id, tick);
        if self.products.iter().any(|row| {
            row.value
                .data
                .supplier_prices
                .get(&name)
                .is_some_and(Option::is_some)
        }) && !self
            .price_suppliers
            .iter()
            .any(|column| column.is_name(&name))
        {
            self.price_suppliers.push(PriceSupplierEntry::Name(name));
        }
    }

    pub fn set_quote(&mut self, id: &str, supplier: &str, quote: Option<i64>, tick: Tick<'_>) {
        let row = self
            .products
            .iter_mut()
            .find(|row| row.id == id)
            .expect("synthetic priced product");
        let prices = &mut row.value.data.supplier_prices;
        if let Some(quote) = quote.filter(|&quote| quote != 0) {
            prices.insert(supplier.into(), Some(Rupees::whole(quote)));
        } else {
            prices.remove(supplier);
        }
        row.value.stamp.write(None, tick);
    }
}
