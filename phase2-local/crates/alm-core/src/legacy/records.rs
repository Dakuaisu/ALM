//! Typed views of the migrated records, read with the HTML's coercions (`+x || 0`, text
//! fields as strings). Amounts stay JavaScript numbers (rupees, packets) so the control totals
//! repeat the HTML's arithmetic; each view keeps the record's legacy ID for provenance.
use serde_json::Value;

use super::LegacyBackup;
use super::js::{field, num, text, truthy};

#[derive(Clone, Debug, PartialEq)]
pub struct LegacyProduct {
    pub id: String,
    pub item: String,
    pub brand: String,
    pub buy_rate: f64,
    pub sell_rate: f64,
    pub avg_cost: f64,
    pub opening_cost: f64,
    pub packet_qty: f64,
    pub mrp: f64,
    /// `reorderLevel`; `None` when absent.
    pub reorder: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LegacyCustomer {
    pub id: String,
    pub name: String,
    pub mobile: String,
    pub addr1: String,
    pub addr2: String,
    pub opening: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LegacySupplier {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LegacyComponent {
    pub item: String,
    pub pid: Option<String>,
    pub unit: String,
    pub qty: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LegacyLine {
    pub item: String,
    pub pid: Option<String>,
    pub unit: String,
    pub packet_qty: f64,
    pub rate: f64,
    pub qty: f64,
    pub amount: f64,
    /// A crate line: its name and contents.
    pub crate_line: Option<(String, Vec<LegacyComponent>)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LegacySale {
    pub id: String,
    pub no: f64,
    pub date: String,
    pub customer_id: String,
    pub customer_name: String,
    pub payment_mode: String,
    pub lines: Vec<LegacyLine>,
    pub loading: f64,
    pub loading_rate: f64,
    pub total: f64,
    pub paid: f64,
    pub cancelled: bool,
    pub created: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LegacyPayment {
    pub id: String,
    pub customer_id: String,
    pub customer_name: String,
    pub amount: f64,
    pub mode: String,
    pub date: String,
}

/// A Cash Book day (`DB.expenses`).
#[derive(Clone, Debug, PartialEq)]
pub struct LegacyCashDay {
    pub id: String,
    pub date: String,
    pub cash: f64,
    pub paytm: f64,
    pub phonepe: f64,
    pub debt: f64,
    pub goods: f64,
    pub goods_desc: String,
    pub misc: f64,
    pub misc_desc: String,
    pub salary: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LegacyCrate {
    pub id: String,
    pub name: String,
    pub price: f64,
    pub components: Vec<LegacyComponent>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LegacyGodown {
    pub id: String,
    pub item: String,
    pub qty: f64,
    pub note: String,
}

/// `DB.stock[key]`.
#[derive(Clone, Debug, PartialEq)]
pub struct LegacyStockEntry {
    pub key: String,
    pub opening: f64,
    pub fridge: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LegacyStockLog {
    pub id: String,
    pub key: String,
    pub item: String,
    pub delta: f64,
    pub eff: bool,
    pub reason: String,
}

/// Where a FIFO lot came from: a legacy purchase line or a purchase-order receipt.
#[derive(Clone, Debug, PartialEq)]
pub struct LegacyLotSource {
    /// `purchases:<id>:<line>` or `purchaseOrders:<id>:<line>:<receipt>`.
    pub legacy_key: String,
    pub key: String,
    pub date: String,
    pub qty: f64,
    /// `None` when the HTML prices the lot at the product's average cost.
    pub rate: Option<f64>,
}

fn id(record: &Value) -> String {
    text(record, "id")
}

fn pid(record: &Value) -> Option<String> {
    record
        .get("pid")
        .filter(|v| truthy(v))
        .map(super::js::string)
}

fn component(value: &Value) -> LegacyComponent {
    LegacyComponent {
        item: text(value, "item"),
        pid: pid(value),
        unit: text(value, "unit"),
        qty: field(value, "qty"),
    }
}

/// A raw sale or purchase line as `LegacyLine`.
pub(super) fn line_of(value: &Value) -> LegacyLine {
    let crate_line = (value.get("isCrate").is_some_and(truthy)).then(|| {
        (
            text(value, "name"),
            value
                .get("components")
                .and_then(Value::as_array)
                .map(|c| c.iter().map(component).collect())
                .unwrap_or_default(),
        )
    });
    LegacyLine {
        item: text(value, "item"),
        pid: pid(value),
        unit: text(value, "unit"),
        packet_qty: field(value, "packetQty"),
        rate: field(value, "rate"),
        qty: field(value, "qty"),
        amount: field(value, "amount"),
        crate_line,
    }
}

impl LegacyBackup {
    pub fn products(&self) -> Vec<LegacyProduct> {
        self.collection("products")
            .iter()
            .map(|p| LegacyProduct {
                id: id(p),
                item: text(p, "item"),
                brand: text(p, "company"),
                buy_rate: field(p, "buyRate"),
                sell_rate: field(p, "sellRate"),
                avg_cost: field(p, "avgCost"),
                opening_cost: field(p, "openingCost"),
                packet_qty: field(p, "packetQty"),
                mrp: field(p, "mrp"),
                reorder: p.get("reorderLevel").filter(|v| !v.is_null()).map(num),
            })
            .collect()
    }

    pub fn customers(&self) -> Vec<LegacyCustomer> {
        self.collection("customers")
            .iter()
            .map(|c| LegacyCustomer {
                id: id(c),
                name: text(c, "name"),
                mobile: text(c, "mobile"),
                addr1: text(c, "addr1"),
                addr2: text(c, "addr2"),
                opening: field(c, "openingBalance"),
            })
            .collect()
    }

    pub fn suppliers(&self) -> Vec<LegacySupplier> {
        self.collection("suppliers")
            .iter()
            .map(|s| LegacySupplier {
                id: id(s),
                name: text(s, "name"),
            })
            .collect()
    }

    pub fn sales(&self) -> Vec<LegacySale> {
        self.collection("sales")
            .iter()
            .map(|s| LegacySale {
                id: id(s),
                no: field(s, "no"),
                date: text(s, "date"),
                customer_id: text(s, "customerId"),
                customer_name: text(s, "customerName"),
                payment_mode: text(s, "paymentMode"),
                lines: s
                    .get("items")
                    .and_then(Value::as_array)
                    .map(|items| items.iter().map(line_of).collect())
                    .unwrap_or_default(),
                loading: field(s, "loading"),
                loading_rate: field(s, "loadingRate"),
                total: field(s, "total"),
                paid: field(s, "paid"),
                cancelled: s.get("cancelled").is_some_and(truthy),
                created: field(s, "created"),
            })
            .collect()
    }

    pub fn payments(&self) -> Vec<LegacyPayment> {
        self.collection("payments")
            .iter()
            .map(|p| LegacyPayment {
                id: id(p),
                customer_id: text(p, "customerId"),
                customer_name: text(p, "customerName"),
                amount: field(p, "amount"),
                mode: text(p, "mode"),
                date: text(p, "date"),
            })
            .collect()
    }

    pub fn cash_days(&self) -> Vec<LegacyCashDay> {
        self.collection("expenses")
            .iter()
            .map(|e| LegacyCashDay {
                id: id(e),
                date: text(e, "date"),
                cash: field(e, "cash"),
                paytm: field(e, "paytm"),
                phonepe: field(e, "phonepe"),
                debt: field(e, "debt"),
                goods: field(e, "goodsAmt"),
                goods_desc: text(e, "goodsDesc"),
                misc: field(e, "miscAmt"),
                misc_desc: text(e, "miscDesc"),
                salary: field(e, "salary"),
            })
            .collect()
    }

    pub fn crates(&self) -> Vec<LegacyCrate> {
        self.collection("crates")
            .iter()
            .map(|c| LegacyCrate {
                id: id(c),
                name: text(c, "name"),
                price: field(c, "price"),
                components: c
                    .get("components")
                    .and_then(Value::as_array)
                    .map(|items| items.iter().map(component).collect())
                    .unwrap_or_default(),
            })
            .collect()
    }

    pub fn godown(&self) -> Vec<LegacyGodown> {
        self.collection("godown")
            .iter()
            .map(|g| LegacyGodown {
                id: id(g),
                item: text(g, "item"),
                qty: field(g, "qty"),
                note: text(g, "note"),
            })
            .collect()
    }

    /// `DB.stock`, by key in key order.
    pub fn stock_entries(&self) -> Vec<LegacyStockEntry> {
        self.db
            .get("stock")
            .and_then(Value::as_object)
            .map(|map| {
                map.iter()
                    .map(|(key, entry)| LegacyStockEntry {
                        key: key.clone(),
                        opening: field(entry, "opening"),
                        fridge: field(entry, "fridge"),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn stock_log(&self) -> Vec<LegacyStockLog> {
        self.collection("stockLog")
            .iter()
            .map(|e| LegacyStockLog {
                id: id(e),
                key: text(e, "key"),
                item: text(e, "item"),
                delta: field(e, "delta"),
                eff: e.get("eff").is_some_and(truthy),
                reason: text(e, "reason"),
            })
            .collect()
    }

    /// `prodByItem`: the first product with exactly this name.
    pub fn product_by_item(&self, item: &str) -> Option<LegacyProduct> {
        self.products().into_iter().find(|p| p.item == item)
    }

    /// `keyOfName`: the product's ID, or `n:<name>` for a free-text item.
    pub fn key_of_name(&self, item: &str) -> String {
        self.product_by_item(item)
            .map_or_else(|| format!("n:{item}"), |p| p.id)
    }

    /// `prodOfLine`: by `pid`, else by exact name.
    pub fn product_of(&self, pid: Option<&str>, item: &str) -> Option<LegacyProduct> {
        let products = self.products();
        pid.and_then(|pid| products.iter().find(|p| p.id == pid).cloned())
            .or_else(|| {
                (!item.is_empty())
                    .then(|| self.product_by_item(item))
                    .flatten()
            })
    }

    fn key_of(&self, pid: Option<&str>, item: &str) -> String {
        pid.map(str::to_owned)
            .or_else(|| self.product_of(None, item).map(|p| p.id))
            .unwrap_or_else(|| format!("n:{item}"))
    }

    /// `linePackets(li)`: `(key, item, packets)` for each product the line moves.
    pub fn line_packets(&self, line: &LegacyLine) -> Vec<(String, String, f64)> {
        let packets = |pid: Option<&str>, item: &str, unit: &str, packet_qty: f64, q: f64| {
            let product = self.product_of(pid, item);
            let pq = if packet_qty != 0.0 {
                packet_qty
            } else {
                product
                    .as_ref()
                    .map(|p| p.packet_qty)
                    .filter(|q| *q != 0.0)
                    .unwrap_or(1.0)
            };
            (
                self.key_of(pid, item),
                item.to_owned(),
                if unit == "Piece" { q / pq } else { q },
            )
        };
        match &line.crate_line {
            Some((_, components)) => components
                .iter()
                .map(|c| packets(c.pid.as_deref(), &c.item, &c.unit, 0.0, c.qty * line.qty))
                .collect(),
            None => vec![packets(
                line.pid.as_deref(),
                &line.item,
                &line.unit,
                line.packet_qty,
                line.qty,
            )],
        }
    }

    /// Every lot `lotsMapAll` builds from purchases and purchase-order receipts, in record
    /// order (before the date sort).
    pub fn purchase_lots(&self) -> Vec<LegacyLotSource> {
        let mut lots = Vec::new();
        for purchase in self.collection("purchases") {
            let pid = id(purchase);
            let date = Some(text(purchase, "date"))
                .filter(|d| !d.is_empty())
                .unwrap_or_else(|| "0000-01-02".into());
            for (n, item) in purchase
                .get("items")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .enumerate()
            {
                let li = line_of(item);
                for (key, _, packets) in self.line_packets(&li) {
                    if packets > 0.0 {
                        lots.push(LegacyLotSource {
                            legacy_key: format!("purchases:{pid}:{n}"),
                            key,
                            date: date.clone(),
                            qty: packets,
                            rate: (li.rate > 0.0).then_some(li.rate),
                        });
                    }
                }
            }
        }
        for order in self.collection("purchaseOrders") {
            let oid = id(order);
            let order_date = Some(text(order, "date"))
                .filter(|d| !d.is_empty())
                .unwrap_or_else(|| "0000-01-02".into());
            for (n, item) in order
                .get("items")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .enumerate()
            {
                let li = line_of(item);
                let key = li.pid.clone().unwrap_or_else(|| self.key_of_name(&li.item));
                let receipts = item
                    .get("receipts")
                    .and_then(Value::as_array)
                    .filter(|r| !r.is_empty());
                match receipts {
                    Some(receipts) => {
                        for (r, receipt) in receipts.iter().enumerate() {
                            let qty = field(receipt, "qty");
                            if qty > 0.0 {
                                let rate = field(receipt, "rate");
                                lots.push(LegacyLotSource {
                                    legacy_key: format!("purchaseOrders:{oid}:{n}:{r}"),
                                    key: key.clone(),
                                    date: Some(text(receipt, "date"))
                                        .filter(|d| !d.is_empty())
                                        .unwrap_or_else(|| order_date.clone()),
                                    qty,
                                    rate: if rate > 0.0 {
                                        Some(rate)
                                    } else {
                                        (li.rate > 0.0).then_some(li.rate)
                                    },
                                });
                            }
                        }
                    }
                    None => {
                        let received = field(item, "received");
                        if received > 0.0 {
                            lots.push(LegacyLotSource {
                                legacy_key: format!("purchaseOrders:{oid}:{n}"),
                                key,
                                date: order_date.clone(),
                                qty: received,
                                rate: (li.rate > 0.0).then_some(li.rate),
                            });
                        }
                    }
                }
            }
        }
        lots
    }
}
