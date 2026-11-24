//! `migrate()` (ALM_Business_Manager.html L464–L620) on the parsed `DB` object, in the HTML's
//! order. `uid()` is a deterministic stream (`<prefix>_mig<n>`) and `Date.now()` / `todayISO()`
//! come from [`MigrateClock`]. One departure: `freshDB().meta` carries the deployed shop's
//! identity (name, owner, mobile, address, GSTIN, FSSAI, MSME); it is not copied into code, so
//! a missing identity field is left blank instead.
use std::collections::BTreeMap;

use serde_json::{Map, Value, json};

use super::MigrateClock;
use super::js::{num, string, truthy};

/// `freshDB().meta` keys without the shop identity and `created`.
const META_DEFAULTS: [&str; 6] = [
    "loadingCharge",
    "syncUrl",
    "lastSync",
    "defaultReorder",
    "gstRate",
    "gstHsn",
];
const META_IDENTITY: [&str; 8] = [
    "name", "tagline", "owner", "mobile", "address", "gst", "fssai", "msme",
];

fn meta_default(key: &str) -> Value {
    match key {
        "loadingCharge" | "lastSync" => json!(0),
        "syncUrl" => json!(""),
        "defaultReorder" => json!(5),
        "gstRate" => json!(18),
        "gstHsn" => json!("2202"),
        _ => json!(""),
    }
}

struct Uid(u64);

impl Uid {
    fn next(&mut self, prefix: &str) -> String {
        self.0 += 1;
        format!("{prefix}_mig{}", self.0)
    }
}

fn array<'a>(db: &'a mut Map<String, Value>, key: &str) -> &'a mut Vec<Value> {
    let slot = db.entry(key.to_owned()).or_insert(Value::Null);
    if !slot.is_array() {
        *slot = Value::Array(Vec::new());
    }
    match slot {
        Value::Array(rows) => rows,
        _ => unreachable!("set to an array above"),
    }
}

fn objects_only(rows: &mut Vec<Value>) -> usize {
    let before = rows.len();
    rows.retain(Value::is_object);
    before - rows.len()
}

fn set(record: &mut Value, key: &str, value: Value) {
    if let Value::Object(map) = record {
        map.insert(key.to_owned(), value);
    }
}

fn is_null(record: &Value, key: &str) -> bool {
    record.get(key).is_none_or(Value::is_null)
}

fn stringify(record: &mut Value, key: &str) -> bool {
    match record.get(key) {
        Some(Value::String(_)) => false,
        None | Some(Value::Null) => {
            set(record, key, json!(""));
            true
        }
        Some(other) => {
            let text = string(other);
            set(record, key, Value::String(text));
            true
        }
    }
}

#[allow(clippy::too_many_lines)]
pub(super) fn migrate(db: &mut Map<String, Value>, clock: MigrateClock<'_>) -> Vec<String> {
    let mut fixes = Vec::new();
    let mut uid = Uid(0);
    let now = json!(clock.now_ms);

    // Defensive defaults first.
    if !db.get("meta").is_some_and(Value::is_object) {
        db.insert("meta".into(), Value::Object(Map::new()));
        fixes.push("meta: missing, started empty".into());
    }
    {
        let Some(Value::Object(meta)) = db.get_mut("meta") else {
            unreachable!("meta is an object")
        };
        for key in META_DEFAULTS.iter().chain(META_IDENTITY.iter()).copied() {
            if meta.get(key).is_none_or(Value::is_null) {
                meta.insert(key.into(), meta_default(key));
                fixes.push(format!("meta.{key}: default"));
            }
        }
    }
    for key in [
        "products",
        "customers",
        "suppliers",
        "sales",
        "purchases",
        "purchaseOrders",
        "expenses",
        "payments",
        "crates",
        "priceSuppliers",
        "gstBills",
    ] {
        let was_array = db.get(key).is_some_and(Value::is_array);
        let rows = array(db, key);
        if !was_array {
            fixes.push(format!("{key}: not a list, started empty"));
        } else if key != "priceSuppliers" {
            let dropped = objects_only(rows);
            if dropped > 0 {
                fixes.push(format!("{key}: {dropped} non-record entries dropped"));
            }
        }
    }
    for product in array(db, "products") {
        if stringify(product, "item") {
            fixes.push("products: item made text".into());
        }
    }
    for key in ["customers", "suppliers"] {
        for party in array(db, key) {
            if stringify(party, "name") {
                fixes.push(format!("{key}: name made text"));
            }
        }
    }
    for key in ["sales", "purchases", "purchaseOrders", "gstBills"] {
        for record in array(db, key) {
            let items = match record.get("items") {
                Some(Value::Array(items)) => {
                    items.iter().filter(|li| li.is_object()).cloned().collect()
                }
                _ => Vec::new(),
            };
            if record.get("items").and_then(Value::as_array).map(Vec::len) != Some(items.len()) {
                fixes.push(format!("{key}: items list repaired"));
            }
            set(record, "items", Value::Array(items));
        }
    }
    if !db.get("stock").is_some_and(Value::is_object) {
        db.insert("stock".into(), Value::Object(Map::new()));
        fixes.push("stock: not an object, started empty".into());
    }
    for product in array(db, "products") {
        if is_null(product, "avgCost") {
            let buy = product.get("buyRate").map_or(0.0, num);
            set(product, "avgCost", json!(buy));
            fixes.push("products: avgCost from buyRate".into());
        }
    }
    {
        let Some(Value::Object(meta)) = db.get_mut("meta") else {
            unreachable!("meta is an object")
        };
        if meta.get("gstRate").is_none_or(Value::is_null) {
            meta.insert("gstRate".into(), json!(18));
        }
        if !meta.get("gstHsn").is_some_and(truthy) {
            meta.insert("gstHsn".into(), json!("2202"));
        }
    }
    for key in ["stockLog", "log", "godown", "deleted"] {
        let was_array = db.get(key).is_some_and(Value::is_array);
        let rows = array(db, key);
        if was_array {
            let dropped = objects_only(rows);
            if dropped > 0 {
                fixes.push(format!("{key}: {dropped} non-record entries dropped"));
            }
        } else {
            fixes.push(format!("{key}: not a list, started empty"));
        }
    }
    for row in array(db, "godown") {
        if stringify(row, "item") {
            fixes.push("godown: item made text".into());
        }
    }
    {
        let Some(Value::Object(meta)) = db.get_mut("meta") else {
            unreachable!("meta is an object")
        };
        if meta.get("rev").map_or(0.0, num).partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
            meta.insert("rev".into(), json!(0));
        }
    }
    for product in array(db, "products") {
        if is_null(product, "openingCost") {
            let avg = product.get("avgCost").map_or(0.0, num);
            let cost = if avg > 0.0 {
                avg
            } else {
                product.get("buyRate").map_or(0.0, num)
            };
            set(product, "openingCost", json!(cost));
            fixes.push("products: openingCost from avgCost/buyRate".into());
        }
    }

    let schema = db
        .get("meta")
        .and_then(|m| m.get("schema"))
        .map_or(0.0, num);
    if schema < 2.0 {
        fixes.push("v1.3: id-link products, lines, stock and suppliers".into());
        let mut by_name: BTreeMap<String, String> = BTreeMap::new();
        let mut dup_names = Vec::new();
        for product in array(db, "products") {
            if !product.get("id").is_some_and(truthy) {
                set(product, "id", Value::String(uid.next("p")));
                fixes.push("products: id assigned".into());
            }
            let key = product.get("item").map(string).unwrap_or_default();
            match by_name.entry(key) {
                std::collections::btree_map::Entry::Occupied(entry) => {
                    dup_names.push(entry.key().clone());
                }
                std::collections::btree_map::Entry::Vacant(entry) => {
                    entry.insert(product.get("id").map(string).unwrap_or_default());
                }
            }
        }
        let pid_of = |item: Option<&Value>| -> Option<String> {
            let item = item.filter(|v| !v.is_null())?;
            by_name.get(&string(item)).cloned()
        };
        for entry in array(db, "stockLog") {
            if is_null(entry, "eff") {
                set(entry, "eff", json!(false));
            }
            if !entry.get("key").is_some_and(truthy) {
                let key = pid_of(entry.get("item")).unwrap_or_else(|| {
                    format!(
                        "n:{}",
                        entry
                            .get("item")
                            .filter(|v| truthy(v))
                            .map(string)
                            .unwrap_or_default()
                    )
                });
                set(entry, "key", Value::String(key));
            }
        }
        let old_stock = match db.get("stock") {
            Some(Value::Object(map)) => map.clone(),
            _ => Map::new(),
        };
        let mut new_stock = Map::new();
        let mut migrated_adjustments = Vec::new();
        for (name, entry) in &old_stock {
            let key =
                pid_of(Some(&Value::String(name.clone()))).unwrap_or_else(|| format!("n:{name}"));
            let target = new_stock.entry(key.clone()).or_insert_with(
                || json!({"opening": 0, "fridge": 0, "_v": 1, "_m": now, "_d": "legacy"}),
            );
            let opening =
                target.get("opening").map_or(0.0, num) + entry.get("opening").map_or(0.0, num);
            let fridge =
                target.get("fridge").map_or(0.0, num) + entry.get("fridge").map_or(0.0, num);
            set(target, "opening", json!(opening));
            set(target, "fridge", json!(fridge));
            let adjust = entry.get("adjust").map_or(0.0, num);
            if adjust != 0.0 {
                migrated_adjustments.push(json!({
                    "id": uid.next("adj"), "ts": now, "date": clock.today, "device": "migrate",
                    "key": key, "item": name, "from": 0, "to": 0, "delta": -adjust,
                    "reason": "Migrated balance (old adjustments)", "note": "", "eff": true,
                    "migrated": true, "_v": 1, "_m": now, "_d": "legacy"
                }));
                fixes.push(format!(
                    "stock[{name}]: adjust {adjust} moved to the stock log"
                ));
            }
        }
        array(db, "stockLog").extend(migrated_adjustments);
        db.insert("stock".into(), Value::Object(new_stock));
        let fix_line = |line: &mut Value| {
            if !line.get("pid").is_some_and(truthy) {
                let pid = pid_of(line.get("item"));
                set(line, "pid", pid.map_or(Value::Null, Value::String));
            }
            if let Some(Value::Array(components)) = line.get_mut("components") {
                for component in components {
                    if !component.get("pid").is_some_and(truthy) {
                        let pid = pid_of(component.get("item"));
                        set(component, "pid", pid.map_or(Value::Null, Value::String));
                    }
                }
            }
        };
        for key in ["sales", "purchases", "purchaseOrders", "gstBills"] {
            for record in array(db, key) {
                if let Some(Value::Array(items)) = record.get_mut("items") {
                    items.iter_mut().for_each(fix_line);
                }
            }
        }
        for template in array(db, "crates") {
            if let Some(Value::Array(components)) = template.get_mut("components") {
                for component in components {
                    if !component.get("pid").is_some_and(truthy) {
                        let pid = pid_of(component.get("item"));
                        set(component, "pid", pid.map_or(Value::Null, Value::String));
                    }
                }
            }
        }
        let mut by_supplier: BTreeMap<String, String> = BTreeMap::new();
        for supplier in array(db, "suppliers") {
            if !supplier.get("id").is_some_and(truthy) {
                set(supplier, "id", Value::String(uid.next("s")));
                fixes.push("suppliers: id assigned".into());
            }
            let key = supplier.get("name").map(string).unwrap_or_default();
            by_supplier
                .entry(key)
                .or_insert_with(|| supplier.get("id").map(string).unwrap_or_default());
        }
        for order in array(db, "purchaseOrders") {
            if !order.get("supId").is_some_and(truthy) {
                let name = order
                    .get("supplier")
                    .filter(|v| truthy(v))
                    .map(string)
                    .unwrap_or_default();
                let id = by_supplier.get(&name).cloned();
                set(order, "supId", id.map_or(Value::Null, Value::String));
            }
        }
        for key in [
            "sales",
            "gstBills",
            "payments",
            "expenses",
            "purchaseOrders",
            "purchases",
            "products",
            "customers",
            "suppliers",
            "crates",
            "godown",
            "stockLog",
        ] {
            for record in array(db, key) {
                if is_null(record, "_v") {
                    let created = record.get("created").map_or(0.0, num);
                    let ts = record.get("ts").map_or(0.0, num);
                    let stamp = if created != 0.0 {
                        json!(created)
                    } else if ts != 0.0 {
                        json!(ts)
                    } else {
                        now.clone()
                    };
                    set(record, "_v", json!(1));
                    set(record, "_m", stamp);
                    set(record, "_d", json!("legacy"));
                }
            }
        }
        let orphans: Vec<String> = match db.get("stock") {
            Some(Value::Object(map)) => map
                .keys()
                .filter_map(|k| k.strip_prefix("n:").map(str::to_owned))
                .collect(),
            _ => Vec::new(),
        };
        if !orphans.is_empty() || !dup_names.is_empty() {
            let mut message = String::from("Data hygiene: ");
            if !dup_names.is_empty() {
                message += &format!("duplicate product name(s): {}. ", dup_names.join(", "));
            }
            if !orphans.is_empty() {
                message += &format!(
                    "{} old product name(s) still hold stock history from renames: {}",
                    orphans.len(),
                    orphans
                        .iter()
                        .take(12)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
            fixes.push(message.clone());
            let entry = json!({"id": uid.next("lg"), "ts": now, "date": clock.today,
                "device": "migrate", "type": "migrate", "msg": message, "_v": 1, "_m": now,
                "_d": "legacy"});
            array(db, "log").push(entry);
        }
        set_meta(db, "schema", json!(2));
        array(db, "log").push(json!({"ts": now, "date": clock.today, "device": "migrate",
            "type": "migrate",
            "msg": "Upgraded data to v1.3 (id-linked stock & lines, log-derived adjustments)"}));
    }
    let schema = db
        .get("meta")
        .and_then(|m| m.get("schema"))
        .map_or(0.0, num);
    if schema < 3.0 {
        fixes.push("v1.4: log entries and meta made syncable".into());
        for entry in array(db, "log") {
            if !entry.get("id").is_some_and(truthy) {
                set(entry, "id", Value::String(uid.next("lg")));
            }
            if is_null(entry, "_v") {
                let ts = entry.get("ts").map_or(0.0, num);
                let device = entry
                    .get("device")
                    .filter(|v| truthy(v))
                    .map_or_else(|| "legacy".to_owned(), string);
                set(entry, "_v", json!(1));
                set(entry, "_m", if ts != 0.0 { json!(ts) } else { now.clone() });
                set(entry, "_d", Value::String(device));
            }
        }
        if db
            .get("meta")
            .and_then(|m| m.get("_v"))
            .is_none_or(Value::is_null)
        {
            set_meta(db, "_v", json!(1));
            set_meta(db, "_m", now.clone());
            set_meta(db, "_d", json!("legacy"));
        }
        set_meta(db, "schema", json!(3));
    }
    fixes
}

fn set_meta(db: &mut Map<String, Value>, key: &str, value: Value) {
    if let Some(Value::Object(meta)) = db.get_mut("meta") {
        meta.insert(key.into(), value);
    }
}
