use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize, de::IgnoredAny};

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct Stamp {
    #[serde(default, rename = "_v")]
    pub v: u64,
    #[serde(default, rename = "_m")]
    pub m: u64,
    #[serde(default, rename = "_d")]
    pub d: String,
}

impl Stamp {
    pub fn new(v: u64, m: u64, d: &str) -> Self {
        Self { v, m, d: d.into() }
    }

    pub fn write(&mut self, previous: Option<u64>, tick: Tick<'_>) {
        let version = previous.filter(|&v| v != 0).unwrap_or(self.v);
        *self = Self::new(
            version.checked_add(1).expect("scripted version fits"),
            tick.m,
            tick.d,
        );
    }
}

#[derive(Clone, Copy)]
pub(super) struct Tick<'a> {
    pub m: u64,
    pub d: &'a str,
}

#[derive(Clone, Copy)]
pub(super) struct DeleteTick<'a> {
    pub ts: u64,
    pub write: Tick<'a>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct Marked<T> {
    #[serde(flatten)]
    pub data: T,
    #[serde(flatten)]
    pub stamp: Stamp,
}

impl<T> Marked<T> {
    pub const fn new(data: T, stamp: Stamp) -> Self {
        Self { data, stamp }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct Row<T> {
    pub id: String,
    #[serde(flatten)]
    pub value: Marked<T>,
}

impl<T> Row<T> {
    pub fn new(id: &str, data: T, stamp: Stamp) -> Self {
        Self {
            id: id.into(),
            value: Marked::new(data, stamp),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct Supplier {
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Product {
    pub item: String,
    pub supplier_prices: BTreeMap<String, Option<i64>>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct GstBill {
    pub total: i64,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Meta {
    pub rev: u64,
    pub schema: u64,
    pub default_reorder: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct Stock {
    pub opening: i64,
    pub fridge: i64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) enum Collection {
    GstBills,
    Products,
    Suppliers,
}

impl Collection {
    pub const fn name(self) -> &'static str {
        match self {
            Self::GstBills => "gstBills",
            Self::Products => "products",
            Self::Suppliers => "suppliers",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct Tombstone {
    pub coll: Collection,
    pub rid: String,
    pub ts: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "collection", content = "json", rename_all = "camelCase")]
pub(super) enum Record {
    GstBills(Row<GstBill>),
    Products(Row<Product>),
    Suppliers(Row<Supplier>),
    Deleted(Row<Tombstone>),
    Stock(Marked<Stock>),
    Meta(Marked<Meta>),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct Envelope {
    pub id: String,
    #[serde(flatten)]
    pub record: Record,
    pub v: u64,
    pub m: u64,
    pub d: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub(super) struct Db {
    pub gst_bills: Vec<Row<GstBill>>,
    pub products: Vec<Row<Product>>,
    pub suppliers: Vec<Row<Supplier>>,
    pub deleted: Vec<Row<Tombstone>>,
    pub stock: BTreeMap<String, Marked<Stock>>,
    pub meta: Marked<Meta>,
    #[serde(default, deserialize_with = "load_price_suppliers")]
    pub price_suppliers: Vec<String>,
}

fn load_price_suppliers<'de, D: Deserializer<'de>>(input: D) -> Result<Vec<String>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum PriceSuppliers {
        Names(Vec<String>),
        NotANameList(IgnoredAny),
    }
    Ok(match PriceSuppliers::deserialize(input)? {
        PriceSuppliers::Names(names) => names,
        PriceSuppliers::NotANameList(_) => Vec::new(),
    })
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Cursor {
    pub seq: u64,
    pub pushed_m: u64,
}

#[derive(Clone, Copy)]
pub(super) enum Origin {
    Pull,
    StalePush,
}
