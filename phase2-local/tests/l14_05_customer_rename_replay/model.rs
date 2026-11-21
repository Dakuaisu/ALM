use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Stamp {
    #[serde(rename = "_m")]
    pub m: u64,
    #[serde(rename = "_v")]
    pub v: u64,
    #[serde(rename = "_d")]
    pub d: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Customer {
    pub id: String,
    pub name: String,
    #[serde(flatten)]
    pub stamp: Stamp,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Dependent {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub customer_id: Option<String>,
    pub customer_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cancelled: Option<bool>,
    #[serde(flatten)]
    pub stamp: Stamp,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Meta {
    pub rev: u64,
    #[serde(flatten)]
    pub stamp: Stamp,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Database {
    pub customers: Vec<Customer>,
    pub sales: Vec<Dependent>,
    pub payments: Vec<Dependent>,
    pub meta: Meta,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "collection", content = "json", rename_all = "lowercase")]
pub enum Record {
    Sales(Dependent),
    Payments(Dependent),
    Customers(Customer),
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Envelope {
    pub id: String,
    #[serde(flatten)]
    pub record: Record,
    pub v: u64,
    pub m: u64,
    pub d: String,
}

pub fn baseline() -> Database {
    serde_json::from_str(r#"{
        "customers":[
            {"id":"c-b","name":"Shared","_m":91,"_v":3,"_d":"seed"},
            {"id":"c-a","name":"Shared","_m":92,"_v":4,"_d":"seed"}
        ],
        "sales":[
            {"id":"s-a","customerId":"c-a","customerName":"Shared","cancelled":false,"_m":99,"_v":4,"_d":"seed"},
            {"id":"s-cancel","customerId":"c-a","customerName":"Shared","cancelled":true,"_m":99,"_v":4,"_d":"seed"},
            {"id":"s-b","customerId":"c-b","customerName":"Shared","_m":80,"_v":2,"_d":"seed"},
            {"id":"s-none","customerName":"Shared","_m":81,"_v":2,"_d":"seed"}
        ],
        "payments":[
            {"id":"p-a","customerId":"c-a","customerName":"Shared","_m":99,"_v":4,"_d":"seed"},
            {"id":"p-b","customerId":"c-b","customerName":"Shared","_m":82,"_v":2,"_d":"seed"},
            {"id":"p-none","customerName":"Shared","_m":83,"_v":2,"_d":"seed"}
        ],
        "meta":{"rev":7,"_m":90,"_v":2,"_d":"seed"}
    }"#).unwrap()
}

pub fn rename(db: &mut Database, id: &str, input: &str) -> bool {
    let name = input.trim();
    if name.is_empty()
        || db
            .customers
            .iter()
            .any(|row| row.id != id && row.name.trim().eq_ignore_ascii_case(name))
    {
        return false;
    }
    for row in db.sales.iter_mut().chain(&mut db.payments) {
        if row.customer_id.as_deref() == Some(id) {
            row.customer_name = name.into();
        }
    }
    let customer = db.customers.iter_mut().find(|row| row.id == id).unwrap();
    customer.name = name.into();
    customer.stamp.v += 1;
    customer.stamp.m = 200;
    customer.stamp.d = "sender".into();
    db.meta.rev += 1;
    true
}

pub fn changed(db: &Database) -> Vec<Envelope> {
    let mut out = Vec::new();
    for (row, record) in db
        .sales
        .iter()
        .map(|r| (r, Record::Sales(r.clone())))
        .chain(db.payments.iter().map(|r| (r, Record::Payments(r.clone()))))
    {
        if row.stamp.m > 100 {
            out.push(Envelope {
                id: row.id.clone(),
                record,
                v: row.stamp.v,
                m: row.stamp.m,
                d: row.stamp.d.clone(),
            });
        }
    }
    for row in &db.customers {
        if row.stamp.m > 100 {
            out.push(Envelope {
                id: row.id.clone(),
                record: Record::Customers(row.clone()),
                v: row.stamp.v,
                m: row.stamp.m,
                d: row.stamp.d.clone(),
            });
        }
    }
    out.sort_by_key(|entry| entry.m);
    out
}

pub fn apply(db: &mut Database, entries: &[Envelope]) -> Vec<bool> {
    entries
        .iter()
        .map(|entry| match &entry.record {
            Record::Sales(record) => replace(&mut db.sales, &entry.id, record),
            Record::Payments(record) => replace(&mut db.payments, &entry.id, record),
            Record::Customers(record) => {
                let current = db.customers.iter_mut().find(|r| r.id == entry.id).unwrap();
                if record.stamp.v > current.stamp.v {
                    *current = record.clone();
                    true
                } else {
                    false
                }
            }
        })
        .collect()
}

fn replace(rows: &mut [Dependent], id: &str, record: &Dependent) -> bool {
    let current = rows.iter_mut().find(|r| r.id == id).unwrap();
    if record.stamp.v > current.stamp.v {
        *current = record.clone();
        true
    } else {
        false
    }
}
