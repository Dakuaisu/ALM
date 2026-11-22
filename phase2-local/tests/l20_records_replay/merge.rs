use super::model::{Collection, Db, DeleteTick, Envelope, Origin, Record, Row, Stamp, Tombstone};
use super::price_columns::PriceSupplierEntry;

const LOCAL_CAP: usize = 4_000;
const LOCAL_KEEP: usize = 3_000;

impl Envelope {
    pub(super) fn from_row<T>(row: &Row<T>, record: Record, device: &str) -> Self {
        Self::from_stamp((&row.id, &row.value.stamp), record, device)
    }

    fn from_stamp((id, stamp): (&str, &Stamp), record: Record, device: &str) -> Self {
        Self {
            id: id.into(),
            record,
            v: stamp.v.max(1),
            m: stamp.m,
            d: if stamp.d.is_empty() {
                device.into()
            } else {
                stamp.d.clone()
            },
        }
    }
}

impl Db {
    pub(super) fn changed(&self, pushed_m: u64, device: &str) -> Vec<Envelope> {
        let mut out = Vec::new();
        out.extend(changed_rows(
            &self.gst_bills,
            Record::GstBills,
            pushed_m,
            device,
        ));
        out.extend(changed_rows(
            &self.purchase_orders,
            Record::PurchaseOrders,
            pushed_m,
            device,
        ));
        out.extend(changed_rows(
            &self.products,
            Record::Products,
            pushed_m,
            device,
        ));
        out.extend(changed_rows(
            &self.suppliers,
            Record::Suppliers,
            pushed_m,
            device,
        ));
        out.extend(changed_rows(&self.log, Record::Log, pushed_m, device));
        out.extend(changed_rows(
            &self.deleted,
            Record::Deleted,
            pushed_m,
            device,
        ));
        for (key, value) in &self.stock {
            if value.stamp.m > pushed_m {
                out.push(Envelope::from_stamp(
                    (key, &value.stamp),
                    Record::Stock(value.clone()),
                    device,
                ));
            }
        }
        if self.meta.stamp.m > pushed_m {
            out.push(Envelope::from_stamp(
                ("meta", &self.meta.stamp),
                Record::Meta(self.meta.clone()),
                device,
            ));
        }
        out.sort_by_key(|record| record.m);
        out
    }

    pub(super) fn remove(&mut self, collection: Collection, id: &str, tick: DeleteTick<'_>) {
        self.remove_target(collection, id);
        if id.is_empty() {
            return;
        }
        let key = format!("{}:{id}", collection.name());
        if let Some(row) = self.deleted.iter_mut().find(|row| row.id == key) {
            row.value.data.ts = tick.ts;
            row.value.stamp.write(None, tick.write);
        } else {
            let mut stamp = Stamp::default();
            stamp.write(None, tick.write);
            self.deleted.push(Row::new(
                &key,
                Tombstone {
                    coll: collection,
                    rid: id.into(),
                    ts: tick.ts,
                },
                stamp,
            ));
        }
        if self.deleted.len() > LOCAL_CAP {
            self.deleted.drain(..self.deleted.len() - LOCAL_KEEP);
        }
    }

    pub(super) fn apply(&mut self, envelope: &Envelope, device: &str, origin: Origin) -> bool {
        if envelope.d == device && matches!(origin, Origin::Pull) {
            return false;
        }
        let id = &envelope.id;
        match &envelope.record {
            Record::Meta(value) => {
                if value.stamp.v <= self.meta.stamp.v {
                    return false;
                }
                let (rev, schema) = (self.meta.data.rev, self.meta.data.schema);
                self.meta = value.clone();
                self.meta.data.rev = rev;
                self.meta.data.schema = schema;
                true
            }
            Record::Stock(value) => {
                if self
                    .stock
                    .get(id)
                    .is_some_and(|old| old.stamp.v >= value.stamp.v)
                {
                    return false;
                }
                self.stock.insert(id.clone(), value.clone());
                true
            }
            Record::Deleted(marker) => {
                let stored = match self.deleted.iter_mut().find(|row| row.id == *id) {
                    Some(row) => replace(row, marker),
                    None => {
                        self.deleted.push(marker.clone());
                        true
                    }
                };
                let target = &marker.value.data;
                let removed =
                    !target.rid.is_empty() && self.remove_target(target.coll, &target.rid);
                stored || removed
            }
            Record::GstBills(row) => {
                !self.is_deleted(Collection::GstBills, id) && upsert(&mut self.gst_bills, id, row)
            }
            Record::Products(row) => {
                !self.is_deleted(Collection::Products, id) && upsert(&mut self.products, id, row)
            }
            Record::PurchaseOrders(row) => {
                !self.is_deleted(Collection::PurchaseOrders, id)
                    && upsert(&mut self.purchase_orders, id, row)
            }
            Record::Suppliers(row) => {
                !self.is_deleted(Collection::Suppliers, id) && upsert(&mut self.suppliers, id, row)
            }
            Record::Log(row) => {
                !self.is_deleted(Collection::Log, id) && upsert(&mut self.log, id, row)
            }
            Record::PriceSuppliers(row) => {
                if self.is_deleted(Collection::PriceSuppliers, id) {
                    return false;
                }
                if let Some(PriceSupplierEntry::Object(current)) = self
                    .price_suppliers
                    .iter_mut()
                    .find(|entry| entry.has_id(id))
                {
                    if row.version() <= current.version() {
                        return false;
                    }
                    current.clone_from(row);
                } else {
                    self.price_suppliers
                        .insert(0, PriceSupplierEntry::Object(row.clone()));
                }
                true
            }
        }
    }

    fn is_deleted(&self, collection: Collection, id: &str) -> bool {
        self.deleted.iter().any(|row| {
            let target = &row.value.data;
            target.coll == collection && target.rid == id
        })
    }

    fn remove_target(&mut self, collection: Collection, id: &str) -> bool {
        match collection {
            Collection::GstBills => remove_row(&mut self.gst_bills, id),
            Collection::Products => remove_row(&mut self.products, id),
            Collection::PurchaseOrders => remove_row(&mut self.purchase_orders, id),
            Collection::Suppliers => remove_row(&mut self.suppliers, id),
            Collection::Log => remove_row(&mut self.log, id),
            Collection::PriceSuppliers => {
                let before = self.price_suppliers.len();
                self.price_suppliers.retain(|entry| !entry.has_id(id));
                self.price_suppliers.len() != before
            }
        }
    }
}

fn changed_rows<'a, T: Clone>(
    rows: &'a [Row<T>],
    record: fn(Row<T>) -> Record,
    pushed_m: u64,
    device: &'a str,
) -> impl Iterator<Item = Envelope> + 'a {
    rows.iter()
        .filter(move |row| !row.id.is_empty() && row.value.stamp.m > pushed_m)
        .map(move |row| Envelope::from_row(row, record(row.clone()), device))
}

fn upsert<T: Clone>(rows: &mut Vec<Row<T>>, id: &str, incoming: &Row<T>) -> bool {
    if let Some(current) = rows.iter_mut().find(|row| row.id == id) {
        replace(current, incoming)
    } else {
        rows.insert(0, incoming.clone());
        true
    }
}

fn replace<T: Clone>(current: &mut Row<T>, incoming: &Row<T>) -> bool {
    if incoming.value.stamp.v <= current.value.stamp.v {
        return false;
    }
    *current = incoming.clone();
    true
}

fn remove_row<T>(rows: &mut Vec<Row<T>>, id: &str) -> bool {
    let before = rows.len();
    rows.retain(|row| row.id != id);
    rows.len() != before
}
