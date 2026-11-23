use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Stamp {
    #[serde(rename = "_m")]
    m: u64,
    #[serde(rename = "_v")]
    v: u64,
    #[serde(rename = "_d")]
    d: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Supplier {
    id: String,
    name: String,
    mobile: String,
    note: String,
    #[serde(flatten)]
    stamp: Stamp,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Purchase {
    id: String,
    #[serde(rename = "supId")]
    sup_id: String,
    supplier: String,
    #[serde(flatten)]
    stamp: Stamp,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Product {
    id: String,
    #[serde(rename = "supplierPrices")]
    supplier_prices: BTreeMap<String, Option<i64>>,
    #[serde(flatten)]
    stamp: Stamp,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Meta {
    rev: u64,
    #[serde(flatten)]
    stamp: Stamp,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Database {
    suppliers: Vec<Supplier>,
    purchases: Vec<Purchase>,
    purchase_orders: Vec<Purchase>,
    products: Vec<Product>,
    price_suppliers: Vec<String>,
    meta: Meta,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "collection", content = "json")]
enum Record {
    #[serde(rename = "suppliers")]
    Supplier(Supplier),
    #[serde(rename = "purchases")]
    Purchase(Purchase),
    #[serde(rename = "purchaseOrders")]
    PurchaseOrder(Purchase),
    #[serde(rename = "products")]
    Product(Product),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Envelope {
    id: String,
    #[serde(flatten)]
    record: Record,
    v: u64,
    m: u64,
    d: String,
}

fn baseline() -> Database {
    serde_json::from_value(json!({
        "suppliers":[
            {"id":"s2","name":"Target","mobile":"2","note":"other","_m":91,"_v":1,"_d":"seed"},
            {"id":"s1","name":"Old","mobile":"1","note":"note","_m":90,"_v":2,"_d":"seed"}
        ],
        "purchases":[
            {"id":"p1","supId":"other","supplier":"Old","_m":99,"_v":3,"_d":"seed"},
            {"id":"p2","supId":"s1","supplier":"old","_m":99,"_v":3,"_d":"seed"}
        ],
        "purchaseOrders":[
            {"id":"o1","supId":"other","supplier":"Old","_m":99,"_v":3,"_d":"seed"},
            {"id":"o2","supId":"s1","supplier":"old","_m":99,"_v":3,"_d":"seed"}
        ],
        "products":[
            {"id":"missing-target","supplierPrices":{"Old":11},"_m":99,"_v":3,"_d":"seed"},
            {"id":"null-target","supplierPrices":{"Old":12,"Target":null},"_m":99,"_v":3,"_d":"seed"},
            {"id":"existing-target","supplierPrices":{"Old":13,"Target":44},"_m":99,"_v":3,"_d":"seed"},
            {"id":"zero-target","supplierPrices":{"Old":14,"Target":0},"_m":99,"_v":3,"_d":"seed"},
            {"id":"null-old","supplierPrices":{"Old":null,"Target":55},"_m":99,"_v":3,"_d":"seed"}
        ],
        "priceSuppliers":["Target","Old","Other","Old","other","Other"],
        "meta":{"rev":7,"_m":90,"_v":2,"_d":"seed"}
    })).unwrap()
}

// Models the editor and renameSupplierRefs, HTML 757-774 and 10483-10498.
fn save(db: &mut Database, id: &str, input: &str) -> bool {
    let name = input.trim();
    if name.is_empty() {
        return false;
    }
    let index = db.suppliers.iter().position(|row| row.id == id).unwrap();
    let old = db.suppliers[index].name.trim().to_owned();
    if !old.is_empty() && old != name {
        for row in db.purchases.iter_mut().chain(&mut db.purchase_orders) {
            if row.supplier == old {
                row.supplier = name.into();
            }
        }
        for label in &mut db.price_suppliers {
            if *label == old {
                *label = name.into();
            }
        }
        let mut seen = Vec::new();
        db.price_suppliers.retain(|label| {
            if seen.contains(label) {
                false
            } else {
                seen.push(label.clone());
                true
            }
        });
        for product in &mut db.products {
            if let Some(Some(value)) = product.supplier_prices.get(&old).copied() {
                if product
                    .supplier_prices
                    .get(name)
                    .is_none_or(Option::is_none)
                {
                    product.supplier_prices.insert(name.into(), Some(value));
                }
                product.supplier_prices.remove(&old);
            }
        }
    }
    let supplier = &mut db.suppliers[index];
    supplier.name = name.into();
    supplier.stamp = Stamp {
        m: 200,
        v: supplier.stamp.v + 1,
        d: "sender".into(),
    };
    db.meta.rev += 1;
    true
}

fn changed(db: &Database) -> Vec<Envelope> {
    let mut result = Vec::new();
    for (id, stamp, record) in db
        .purchase_orders
        .iter()
        .map(|r| (&r.id, &r.stamp, Record::PurchaseOrder(r.clone())))
        .chain(
            db.purchases
                .iter()
                .map(|r| (&r.id, &r.stamp, Record::Purchase(r.clone()))),
        )
        .chain(
            db.products
                .iter()
                .map(|r| (&r.id, &r.stamp, Record::Product(r.clone()))),
        )
        .chain(
            db.suppliers
                .iter()
                .map(|r| (&r.id, &r.stamp, Record::Supplier(r.clone()))),
        )
    {
        if stamp.m > 100 {
            result.push(Envelope {
                id: id.clone(),
                record,
                v: stamp.v,
                m: stamp.m,
                d: stamp.d.clone(),
            });
        }
    }
    result.sort_by_key(|row| row.m);
    result
}

fn replace<T: Clone>(
    rows: &mut [T],
    incoming: (&str, &T),
    identity: impl Fn(&T) -> (&str, u64),
) -> bool {
    let current = rows
        .iter_mut()
        .find(|row| identity(row).0 == incoming.0)
        .unwrap();
    if identity(incoming.1).1 > identity(current).1 {
        *current = incoming.1.clone();
        true
    } else {
        false
    }
}

fn apply(db: &mut Database, envelope: &Envelope) -> bool {
    match &envelope.record {
        Record::Supplier(r) => {
            replace(&mut db.suppliers, (&envelope.id, r), |x| (&x.id, x.stamp.v))
        }
        Record::Purchase(r) => {
            replace(&mut db.purchases, (&envelope.id, r), |x| (&x.id, x.stamp.v))
        }
        Record::PurchaseOrder(r) => replace(&mut db.purchase_orders, (&envelope.id, r), |x| {
            (&x.id, x.stamp.v)
        }),
        Record::Product(r) => replace(&mut db.products, (&envelope.id, r), |x| (&x.id, x.stamp.v)),
    }
}

#[cfg(test)]
mod assertions;
