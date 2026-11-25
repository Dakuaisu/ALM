//! The legacy import mapping (`MAPPING_VERSION`): each migrated legacy record becomes one
//! authority record of a kind, keyed by its legacy ID, with amounts in paise and quantities
//! whole. A record that cannot be mapped (no or duplicate legacy ID, tombstoned in the same
//! file, an amount below a paisa, a fractional quantity) is quarantined with its reason, never
//! dropped or merged. [`apply`] is the authority's rule: same file again changes nothing; a
//! legacy record already imported with the same body maps to its authority record; with another
//! body, or with a reference that resolves to no imported record, it is quarantined.
use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use super::LegacyBackup;
use super::js::{num, string, truthy};

pub const MAPPING_VERSION: &str = "legacy-map-1";

#[derive(Clone, Copy)]
pub(super) enum F {
    Text,
    OptText,
    Money,
    OptMoney,
    Qty,
    OptQty,
    Int,
    Bool,
    List(&'static [(&'static str, F)]),
}

const COMPONENT: &[(&str, F)] = &[
    ("item", F::Text),
    ("pid", F::OptText),
    ("unit", F::Text),
    ("qty", F::Qty),
];
const SALE_LINE: &[(&str, F)] = &[
    ("item", F::Text),
    ("pid", F::OptText),
    ("isCrate", F::Bool),
    ("name", F::Text),
    ("components", F::List(COMPONENT)),
    ("unit", F::Text),
    ("packSize", F::Text),
    ("uom", F::Text),
    ("packetQty", F::Qty),
    ("mrp", F::Money),
    ("rate", F::Money),
    ("qty", F::Qty),
    ("amount", F::Money),
    ("costAtSale", F::OptMoney),
];
const RECEIPT: &[(&str, F)] = &[("date", F::Text), ("qty", F::Qty), ("rate", F::Money)];
const PO_LINE: &[(&str, F)] = &[
    ("item", F::Text),
    ("pid", F::OptText),
    ("rate", F::Money),
    ("ordered", F::Qty),
    ("received", F::Qty),
    ("cancelled", F::Qty),
    ("receipts", F::List(RECEIPT)),
];
const GST_LINE: &[(&str, F)] = &[
    ("item", F::Text),
    ("pid", F::OptText),
    ("hsn", F::Text),
    ("unit", F::Text),
    ("qty", F::Qty),
    ("rate", F::Money),
    ("gst", F::Money),
    ("packetQty", F::Qty),
    ("amount", F::Money),
];

pub(super) type Fields = &'static [(&'static str, F)];

/// `(kind, collection, fields)` in import order: referenced kinds first.
pub(super) const KINDS: &[(&str, &str, Fields)] = &[
    (
        "meta",
        "meta",
        &[
            ("name", F::Text),
            ("tagline", F::Text),
            ("loadingCharge", F::Money),
            ("defaultReorder", F::Qty),
            ("gstRate", F::Money),
            ("gstHsn", F::Text),
            ("schema", F::Qty),
        ],
    ),
    (
        "customer",
        "customers",
        &[
            ("name", F::Text),
            ("mobile", F::Text),
            ("addr1", F::Text),
            ("addr2", F::Text),
            ("openingBalance", F::Money),
            ("gstin", F::Text),
        ],
    ),
    ("supplier", "suppliers", &[("name", F::Text)]),
    ("price_supplier", "priceSuppliers", &[("name", F::Text)]),
    (
        "product",
        "products",
        &[
            ("item", F::Text),
            ("company", F::Text),
            ("packSize", F::Text),
            ("uom", F::Text),
            ("packetQty", F::Qty),
            ("mrp", F::Money),
            ("buyRate", F::Money),
            ("avgCost", F::Money),
            ("openingCost", F::Money),
            ("margin", F::Money),
            ("sellRate", F::Money),
            ("reorderLevel", F::OptQty),
            ("gstRate", F::OptMoney),
            ("cess", F::OptMoney),
            ("hsn", F::OptText),
        ],
    ),
    ("stock", "stock", &[("opening", F::Qty), ("fridge", F::Qty)]),
    (
        "stock_log",
        "stockLog",
        &[
            ("key", F::Text),
            ("pid", F::OptText),
            ("item", F::Text),
            ("date", F::Text),
            ("delta", F::Qty),
            ("eff", F::Bool),
            ("reason", F::Text),
            ("note", F::Text),
        ],
    ),
    (
        "purchase",
        "purchases",
        &[
            ("date", F::Text),
            ("supplier", F::Text),
            ("items", F::List(SALE_LINE)),
        ],
    ),
    (
        "purchase_order",
        "purchaseOrders",
        &[
            ("no", F::Qty),
            ("date", F::Text),
            ("supplier", F::Text),
            ("supId", F::OptText),
            ("note", F::Text),
            ("cancelled", F::Bool),
            ("items", F::List(PO_LINE)),
        ],
    ),
    (
        "crate",
        "crates",
        &[
            ("name", F::Text),
            ("price", F::Money),
            ("components", F::List(COMPONENT)),
        ],
    ),
    (
        "sale",
        "sales",
        &[
            ("no", F::Qty),
            ("date", F::Text),
            ("customerId", F::Text),
            ("customerName", F::Text),
            ("paymentMode", F::Text),
            ("items", F::List(SALE_LINE)),
            ("loading", F::Money),
            ("loadingRate", F::Money),
            ("subtotal", F::Money),
            ("total", F::Money),
            ("paid", F::Money),
            ("cancelled", F::Bool),
            ("note", F::Text),
            ("created", F::Int),
        ],
    ),
    (
        "payment",
        "payments",
        &[
            ("customerId", F::Text),
            ("customerName", F::Text),
            ("amount", F::Money),
            ("mode", F::Text),
            ("date", F::Text),
            ("note", F::Text),
        ],
    ),
    (
        "gst_bill",
        "gstBills",
        &[
            ("no", F::Qty),
            ("date", F::Text),
            ("customerId", F::Text),
            ("customerName", F::Text),
            ("custGstin", F::Text),
            ("cancelled", F::Bool),
            ("items", F::List(GST_LINE)),
        ],
    ),
    (
        "cash_day",
        "expenses",
        &[
            ("date", F::Text),
            ("cash", F::Money),
            ("paytm", F::Money),
            ("phonepe", F::Money),
            ("debt", F::Money),
            ("goodsDesc", F::Text),
            ("goodsAmt", F::Money),
            ("miscDesc", F::Text),
            ("miscAmt", F::Money),
            ("salary", F::Money),
        ],
    ),
    (
        "godown",
        "godown",
        &[("item", F::Text), ("qty", F::Qty), ("note", F::Text)],
    ),
    (
        "log",
        "log",
        &[
            ("ts", F::Int),
            ("date", F::Text),
            ("device", F::Text),
            ("type", F::Text),
            ("msg", F::Text),
        ],
    ),
    (
        "deleted",
        "deleted",
        &[("coll", F::Text), ("rid", F::Text), ("ts", F::Int)],
    ),
];

/// The legacy collection a kind comes from.
pub fn collection_of(kind: &str) -> Option<&'static str> {
    KINDS.iter().find(|(k, ..)| *k == kind).map(|(_, c, _)| *c)
}

fn whole(x: f64) -> bool {
    x.is_finite() && x.fract() == 0.0 && x.abs() < 9.0e15
}

pub(super) fn encode(record: &Value, fields: &[(&str, F)], issues: &mut Vec<String>) -> Value {
    let mut out = Map::new();
    for &(name, f) in fields {
        let raw = record.get(name).filter(|v| !v.is_null());
        let value = match f {
            F::Text => json!(raw.map(string).unwrap_or_default()),
            F::OptText => raw.map_or(Value::Null, |v| json!(string(v))),
            F::Money | F::OptMoney => match (f, raw) {
                (F::OptMoney, None) => Value::Null,
                _ => {
                    let x = raw.map_or(0.0, num) * 100.0;
                    if (x - x.round()).abs() > 1e-6 {
                        issues.push(format!("{name} {} is below a paisa", x / 100.0));
                    }
                    json!(x.round() as i64)
                }
            },
            F::Qty | F::OptQty => match (f, raw) {
                (F::OptQty, None) => Value::Null,
                _ => {
                    let x = raw.map_or(0.0, num);
                    if !whole(x) {
                        issues.push(format!("{name} {x} is not a whole quantity"));
                    }
                    json!(x.round() as i64)
                }
            },
            F::Int => json!(raw.map_or(0.0, num).round() as i64),
            F::Bool => json!(raw.is_some_and(truthy)),
            F::List(sub) => Value::Array(
                raw.and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter(|v| v.is_object())
                    .map(|v| encode(v, sub, issues))
                    .collect(),
            ),
        };
        out.insert(name.to_owned(), value);
    }
    Value::Object(out)
}

fn decode(body: &Value, fields: &[(&str, F)]) -> Value {
    let mut out = Map::new();
    for &(name, f) in fields {
        let v = &body[name];
        let value = match f {
            F::Money | F::OptMoney if v.is_null() => Value::Null,
            F::Money | F::OptMoney => {
                serde_json::Number::from_f64(v.as_i64().unwrap_or(0) as f64 / 100.0)
                    .map_or(Value::Null, Value::Number)
            }
            F::List(sub) => Value::Array(
                v.as_array()
                    .into_iter()
                    .flatten()
                    .map(|x| decode(x, sub))
                    .collect(),
            ),
            _ => v.clone(),
        };
        out.insert(name.to_owned(), value);
    }
    Value::Object(out)
}

pub(super) fn refs_of(kind: &str, body: &Value) -> Vec<(String, String)> {
    let mut refs: Vec<(String, String)> = Vec::new();
    let mut add = |k: &str, id: &str| {
        let pair = (k.to_owned(), id.to_owned());
        if !id.is_empty() && !refs.contains(&pair) {
            refs.push(pair);
        }
    };
    let text = |v: &Value| v.as_str().unwrap_or_default().to_owned();
    match kind {
        "sale" | "payment" | "gst_bill" => add("customer", &text(&body["customerId"])),
        "purchase_order" => add("supplier", &text(&body["supId"])),
        "stock" | "stock_log" => {}
        _ => {}
    }
    if kind == "stock_log" {
        let key = text(&body["key"]);
        if !key.starts_with("n:") {
            add("product", &key);
        }
    }
    let lines = body["items"]
        .as_array()
        .into_iter()
        .flatten()
        .chain(body["components"].as_array().into_iter().flatten());
    for line in lines {
        add("product", &text(&line["pid"]));
        for c in line["components"].as_array().into_iter().flatten() {
            add("product", &text(&c["pid"]));
        }
    }
    refs
}

/// The records a mapped record refers to; a stock entry also refers to its product.
pub(super) fn record_refs(kind: &str, legacy_id: &str, body: &Value) -> Vec<(String, String)> {
    let mut refs = refs_of(kind, body);
    if kind == "stock" && !legacy_id.starts_with("n:") {
        refs.insert(0, ("product".into(), legacy_id.to_owned()));
    }
    refs
}

/// One legacy record mapped for the authority.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PlannedRecord {
    pub kind: String,
    pub legacy_id: String,
    pub body: Value,
    /// `(kind, legacy ID)` of the records it refers to, in order.
    pub refs: Vec<(String, String)>,
    /// Where the record sits in the source file (a JSON pointer such as `/sales/3`); empty when
    /// `migrate()` created it.
    #[serde(default)]
    pub source: String,
}

/// A legacy record the mapping cannot place, with why.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PlannedQuarantine {
    pub kind: String,
    pub legacy_id: String,
    pub reason: String,
    /// As [`PlannedRecord::source`].
    #[serde(default)]
    pub source: String,
}

/// What a device sends the authority to import one legacy file.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ImportPlan {
    /// SHA-256 of the source file.
    pub digest: String,
    pub version: String,
    pub records: Vec<PlannedRecord>,
    pub quarantine: Vec<PlannedQuarantine>,
    /// The raw source file the plan was mapped from (UTF-8 JSON), uploaded with the plan; the
    /// authority stores it Owner-only and refuses a plan whose digest is not its SHA-256.
    #[serde(default)]
    pub source: String,
}

/// Why the authority refuses a plan's source file.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceError {
    /// No source file was uploaded with the plan.
    Missing,
    /// The source's SHA-256 is not the plan's digest.
    DigestMismatch,
}

/// The authority's source check: the uploaded file's SHA-256 must be the plan's digest.
///
/// # Errors
/// [`SourceError`] when the file is missing or its digest differs.
pub fn check_source(plan: &ImportPlan) -> Result<(), SourceError> {
    if plan.source.is_empty() {
        return Err(SourceError::Missing);
    }
    if super::sha256_hex(plan.source.as_bytes()) != plan.digest {
        return Err(SourceError::DigestMismatch);
    }
    Ok(())
}

/// Reads a legacy file as an import does ([`LegacyBackup::parse_for_import`]), maps it with
/// [`plan`], points each record at its place in the file and attaches the file.
///
/// # Errors
/// As [`LegacyBackup::parse_for_import`]; a file that is not UTF-8 is not JSON.
pub fn plan_file(bytes: &[u8]) -> Result<(LegacyBackup, ImportPlan), super::LegacyError> {
    let backup = LegacyBackup::parse_for_import(bytes)?;
    let text = std::str::from_utf8(bytes).map_err(|_| super::LegacyError::NotJson)?;
    let raw: Value = serde_json::from_str(text).map_err(|_| super::LegacyError::NotJson)?;
    let mut plan = plan(&backup);
    let locate = Locator::new(&raw);
    for record in &mut plan.records {
        record.source = locate.find(&record.kind, &record.legacy_id);
    }
    for entry in &mut plan.quarantine {
        entry.source = locate.find(&entry.kind, &entry.legacy_id);
    }
    plan.source = text.to_owned();
    Ok((backup, plan))
}

/// JSON pointers into the raw file, by collection and legacy ID.
struct Locator<'a> {
    raw: &'a Value,
    ids: std::collections::BTreeMap<(&'static str, String), usize>,
}

fn pointer_token(text: &str) -> String {
    text.replace('~', "~0").replace('/', "~1")
}

impl<'a> Locator<'a> {
    fn new(raw: &'a Value) -> Self {
        let mut ids = std::collections::BTreeMap::new();
        for &(_, collection, _) in KINDS {
            for (index, row) in raw
                .get(collection)
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .enumerate()
            {
                let id = match row {
                    Value::String(name) if collection == "priceSuppliers" => name.clone(),
                    _ => match row.get("id").filter(|id| truthy(id)) {
                        Some(id) => string(id),
                        None => continue,
                    },
                };
                ids.entry((collection, id)).or_insert(index);
            }
        }
        Self { raw, ids }
    }

    fn find(&self, kind: &str, legacy_id: &str) -> String {
        let Some(collection) = collection_of(kind) else {
            return String::new();
        };
        match collection {
            "meta" if self.raw.get("meta").is_some() => "/meta".into(),
            "stock"
                if self
                    .raw
                    .get("stock")
                    .and_then(|s| s.get(legacy_id))
                    .is_some() =>
            {
                format!("/stock/{}", pointer_token(legacy_id))
            }
            _ => {
                if let Some(index) = self.ids.get(&(collection, legacy_id.to_owned())) {
                    return format!("/{collection}/{index}");
                }
                // `collection[i]`: a record without a legacy ID at that place.
                legacy_id
                    .strip_prefix(collection)
                    .and_then(|rest| rest.strip_prefix('['))
                    .and_then(|rest| rest.strip_suffix(']'))
                    .and_then(|index| index.parse::<usize>().ok())
                    .filter(|&index| {
                        self.raw
                            .get(collection)
                            .and_then(Value::as_array)
                            .is_some_and(|rows| index < rows.len())
                    })
                    .map_or_else(String::new, |index| format!("/{collection}/{index}"))
            }
        }
    }
}

impl ImportPlan {
    /// The plan as one line of JSON (the intent's payload).
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn from_json(text: &str) -> Option<Self> {
        serde_json::from_str(text).ok()
    }
}

/// Maps every migrated record of `backup`.
pub fn plan(backup: &LegacyBackup) -> ImportPlan {
    let mut records = Vec::new();
    let mut quarantine = Vec::new();
    let tombstones: BTreeSet<(String, String)> = backup
        .collection("deleted")
        .iter()
        .map(|t| {
            (
                t.get("coll").map(string).unwrap_or_default(),
                t.get("rid").map(string).unwrap_or_default(),
            )
        })
        .collect();
    for &(kind, collection, fields) in KINDS {
        let rows: Vec<(String, Value)> = match collection {
            "meta" => vec![("meta".into(), Value::Object(backup.meta().clone()))],
            "stock" => backup
                .db
                .get("stock")
                .and_then(Value::as_object)
                .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                .unwrap_or_default(),
            "priceSuppliers" => backup
                .collection(collection)
                .iter()
                .enumerate()
                .map(|(i, v)| match v {
                    Value::String(name) => (name.clone(), json!({"name": name})),
                    _ => (format!("{collection}[{i}]"), Value::Null),
                })
                .collect(),
            _ => backup
                .collection(collection)
                .iter()
                .enumerate()
                .map(|(i, v)| {
                    let id = v
                        .get("id")
                        .filter(|id| truthy(id))
                        .map_or_else(|| format!("{collection}[{i}]"), string);
                    (id, v.clone())
                })
                .collect(),
        };
        let mut seen = BTreeSet::new();
        for (index, (legacy_id, record)) in rows.into_iter().enumerate() {
            let quarantine_as = |reason: String| PlannedQuarantine {
                kind: kind.into(),
                legacy_id: legacy_id.clone(),
                reason,
                source: String::new(),
            };
            if record.is_null() {
                quarantine.push(quarantine_as("not a supplier name".into()));
                continue;
            }
            if legacy_id == format!("{collection}[{index}]") {
                quarantine.push(quarantine_as("no legacy ID".into()));
                continue;
            }
            if !seen.insert(legacy_id.clone()) {
                quarantine.push(quarantine_as("duplicate legacy ID in the file".into()));
                continue;
            }
            if tombstones.contains(&(collection.to_owned(), legacy_id.clone())) {
                quarantine.push(quarantine_as(format!(
                    "deleted in the same file (tombstone {collection}:{legacy_id})"
                )));
                continue;
            }
            let mut issues = Vec::new();
            let body = encode(&record, fields, &mut issues);
            if !issues.is_empty() {
                quarantine.push(quarantine_as(issues.join("; ")));
                continue;
            }
            let refs = record_refs(kind, &legacy_id, &body);
            records.push(PlannedRecord {
                kind: kind.into(),
                legacy_id,
                body,
                refs,
                source: String::new(),
            });
        }
    }
    ImportPlan {
        digest: backup.digest.clone(),
        version: MAPPING_VERSION.into(),
        records,
        quarantine,
        source: String::new(),
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryStatus {
    Imported,
    /// Imported earlier with the same body: mapped to that authority record.
    Already,
    Quarantined,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ManifestEntry {
    pub kind: String,
    pub legacy_id: String,
    pub authority_id: Option<String>,
    pub status: EntryStatus,
    pub reason: String,
    /// Where the record sits in the stored source file (a JSON pointer; empty when
    /// `migrate()` created it).
    #[serde(default)]
    pub source: String,
}

/// One imported file: its digest, mapping version and every record's fate.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ImportManifest {
    pub id: u64,
    pub digest: String,
    pub version: String,
    pub entries: Vec<ManifestEntry>,
}

impl ImportManifest {
    pub fn count(&self, status: EntryStatus) -> usize {
        self.entries.iter().filter(|e| e.status == status).count()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ImportedRecord {
    pub authority_id: String,
    pub kind: String,
    pub legacy_id: String,
    pub body: Value,
    pub manifest: u64,
}

/// Everything the authority holds from legacy imports (Owner book rows).
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct LegacyImports {
    pub manifests: Vec<ImportManifest>,
    pub records: Vec<ImportedRecord>,
    /// The Owner's decision on each quarantined entry that has one (never automatic).
    #[serde(default)]
    pub resolutions: Vec<super::resolve::Resolution>,
}

/// The authority's import rule. Returns the manifest's ID (an existing one when the same file
/// and mapping were imported before, with nothing changed).
pub fn apply(imports: &mut LegacyImports, plan: &ImportPlan) -> u64 {
    if let Some(m) = imports
        .manifests
        .iter()
        .find(|m| m.digest == plan.digest && m.version == plan.version)
    {
        return m.id;
    }
    let id = imports.manifests.iter().map(|m| m.id).max().unwrap_or(0) + 1;
    // Records by (kind, legacy ID) and the count per kind, kept as records are added: one
    // lookup per record instead of a scan of every record held.
    let mut placed: BTreeMap<(String, String), usize> = BTreeMap::new();
    let mut per_kind: BTreeMap<String, usize> = BTreeMap::new();
    for (index, r) in imports.records.iter().enumerate() {
        placed
            .entry((r.kind.clone(), r.legacy_id.clone()))
            .or_insert(index);
        *per_kind.entry(r.kind.clone()).or_default() += 1;
    }
    let mut entries: Vec<ManifestEntry> = plan
        .quarantine
        .iter()
        .map(|q| ManifestEntry {
            kind: q.kind.clone(),
            legacy_id: q.legacy_id.clone(),
            authority_id: None,
            status: EntryStatus::Quarantined,
            reason: q.reason.clone(),
            source: q.source.clone(),
        })
        .collect();
    for record in &plan.records {
        let entry = |authority_id: Option<String>, status, reason: String| ManifestEntry {
            kind: record.kind.clone(),
            legacy_id: record.legacy_id.clone(),
            authority_id,
            status,
            reason,
            source: record.source.clone(),
        };
        let key = (record.kind.clone(), record.legacy_id.clone());
        if let Some(existing) = placed.get(&key).map(|&i| &imports.records[i]) {
            entries.push(if existing.body == record.body {
                entry(
                    Some(existing.authority_id.clone()),
                    EntryStatus::Already,
                    String::new(),
                )
            } else {
                entry(
                    None,
                    EntryStatus::Quarantined,
                    format!("differs from the imported record {}", existing.authority_id),
                )
            });
            continue;
        }
        if let Some((kind, legacy)) = record
            .refs
            .iter()
            .find(|(kind, legacy)| !placed.contains_key(&(kind.clone(), legacy.clone())))
        {
            entries.push(entry(
                None,
                EntryStatus::Quarantined,
                format!("unresolved reference {kind} {legacy}"),
            ));
            continue;
        }
        let count = per_kind.entry(record.kind.clone()).or_default();
        *count += 1;
        let authority_id = format!("{}-{count}", record.kind);
        placed.insert(key, imports.records.len());
        imports.records.push(ImportedRecord {
            authority_id: authority_id.clone(),
            kind: record.kind.clone(),
            legacy_id: record.legacy_id.clone(),
            body: record.body.clone(),
            manifest: id,
        });
        entries.push(entry(
            Some(authority_id),
            EntryStatus::Imported,
            String::new(),
        ));
    }
    imports.manifests.push(ImportManifest {
        id,
        digest: plan.digest.clone(),
        version: plan.version.clone(),
        entries,
    });
    id
}

/// The legacy `DB` object rebuilt from what manifest `id` placed in the authority (imported or
/// already-imported records), as the controls read it.
pub fn rebuilt(imports: &LegacyImports, id: u64) -> Option<LegacyBackup> {
    let manifest = imports.manifests.iter().find(|m| m.id == id)?;
    let mut db = Map::new();
    for &(_, collection, _) in KINDS {
        db.insert(
            collection.to_owned(),
            match collection {
                "meta" | "stock" => Value::Object(Map::new()),
                _ => Value::Array(Vec::new()),
            },
        );
    }
    let by_id: BTreeMap<&str, &ImportedRecord> = imports
        .records
        .iter()
        .map(|r| (r.authority_id.as_str(), r))
        .collect();
    for entry in &manifest.entries {
        let Some(&record) = entry.authority_id.as_deref().and_then(|a| by_id.get(a)) else {
            continue;
        };
        let Some(&(_, collection, fields)) = KINDS.iter().find(|(k, ..)| *k == record.kind) else {
            continue;
        };
        let mut value = decode(&record.body, fields);
        match collection {
            "meta" => {
                db.insert("meta".into(), value);
            }
            "stock" => {
                if let Some(Value::Object(stock)) = db.get_mut("stock") {
                    stock.insert(record.legacy_id.clone(), value);
                }
            }
            "priceSuppliers" => {
                if let Some(Value::Array(rows)) = db.get_mut(collection) {
                    rows.push(value["name"].clone());
                }
            }
            _ => {
                if let Value::Object(map) = &mut value {
                    map.insert("id".into(), json!(record.legacy_id));
                }
                if let Some(Value::Array(rows)) = db.get_mut(collection) {
                    rows.push(value);
                }
            }
        }
    }
    Some(LegacyBackup {
        db,
        digest: manifest.digest.clone(),
        source_schema: 3,
        fixes: Vec::new(),
    })
}

/// The migrated backup without the records manifest `id` quarantined (a duplicate legacy ID
/// quarantines its later occurrences only).
pub fn without_quarantined(backup: &LegacyBackup, manifest: &ImportManifest) -> LegacyBackup {
    let quarantined: BTreeSet<(&str, &str)> = manifest
        .entries
        .iter()
        .filter(|e| e.status == EntryStatus::Quarantined && !e.reason.starts_with("duplicate"))
        .filter_map(|e| Some((collection_of(&e.kind)?, e.legacy_id.as_str())))
        .collect();
    let mut out = backup.clone();
    for &(_, collection, _) in KINDS {
        match out.db.get_mut(collection) {
            Some(Value::Object(map)) if collection == "stock" => {
                map.retain(|key, _| !quarantined.contains(&(collection, key.as_str())));
            }
            Some(Value::Array(rows)) => {
                let mut seen = BTreeSet::new();
                let mut index = 0;
                rows.retain(|row| {
                    let unnamed = format!("{collection}[{index}]");
                    index += 1;
                    let id = match row {
                        Value::String(name) => name.clone(),
                        Value::Object(_) => row
                            .get("id")
                            .filter(|id| truthy(id))
                            .map_or_else(|| unnamed.clone(), string),
                        _ => unnamed.clone(),
                    };
                    let first = seen.insert(id.clone());
                    first && id != unnamed && !quarantined.contains(&(collection, id.as_str()))
                });
            }
            _ => {}
        }
    }
    out
}
