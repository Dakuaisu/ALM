//! L09-04: source model for disposable legacy rename fixtures, not execution of the HTML.
//! ALM_Business_Manager.html:735-755 (cascade), 7814-7822 (save),
//! 961-998 (stamp/deletes), 9472-9496 (delta). A name-key change is not a stock transfer.

use std::collections::BTreeMap;

const OLD: &str = "Orange 250ml";
const NEW: &str = "Orange Glass 250ml";
const PID: ProductId = ProductId("p9");
const PUSHED_M: u64 = 100;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ProductId(&'static str);

// Legacy envelope in _m, _v, _d order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Stamp(u64, u32, &'static str);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Row<T> {
    value: T,
    stamp: Stamp,
}

impl<T> Row<T> {
    const fn new(value: T, stamp: Stamp) -> Self {
        Self { value, stamp }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Stock {
    opening: u32,
    fridge: u32,
}

impl Stock {
    const fn new(opening: u32, fridge: u32) -> Self {
        Self { opening, fridge }
    }
}

const OLD_STOCK: Row<Stock> = Row::new(Stock::new(17, 4), Stamp(101, 7, "legacy"));
const ID_STOCK: Row<Stock> = Row::new(Stock::new(42, 6), Stamp(95, 9, "inventory"));
const TARGET_STOCK: Row<Stock> = Row::new(Stock::new(3, 1), Stamp(100, 5, "destination"));
const SAVED_PRODUCT: Row<&str> = Row::new(NEW, Stamp(200, 5, "editor"));

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Godown {
    item: &'static str,
    cases: u32,
    note: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Component {
    item: &'static str,
    pid: Option<ProductId>,
    qty: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SaleLine {
    item: &'static str,
    pid: ProductId,
    qty: u32,
    rate: u32,
    amount: u32,
    cost_at_sale: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PurchaseLine {
    item: &'static str,
    pid: ProductId,
    ordered: u32,
    received: u32,
    rate: u32,
    receipts: [Receipt; 1],
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Receipt {
    date: &'static str,
    qty: u32,
    rate: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Legacy {
    products: BTreeMap<ProductId, Row<&'static str>>,
    stock: BTreeMap<&'static str, Row<Stock>>,
    godown: Row<Godown>,
    crate_template: Row<[Component; 2]>,
    sale: Row<SaleLine>,
    po: Row<PurchaseLine>,
    deleted: Vec<Row<&'static str>>,
}

impl Legacy {
    fn save_existing_rename(&mut self, new_name: &'static str, now: u64) {
        let product = self.products.get_mut(&PID).expect("existing product");
        let old_name = product.value;
        let new_name = new_name.trim();
        product.value = new_name;
        self.rename_product_refs(old_name, new_name);
        let product = self.products.get_mut(&PID).expect("same product ID");
        product.stamp = Stamp(now, product.stamp.1 + 1, "editor");
        self.stock
            .entry(PID.0)
            .or_insert(Row::new(Stock::new(0, 0), Stamp(now, 1, "editor")));
    }

    fn rename_product_refs(&mut self, old_name: &str, new_name: &'static str) {
        let old_name = old_name.trim();
        if old_name.is_empty()
            || old_name == new_name
            || self
                .products
                .iter()
                .any(|(id, row)| *id != PID && row.value == old_name)
        {
            return;
        }
        if let Some(row) = self.stock.remove(old_name) {
            self.stock.entry(new_name).or_insert(row);
        }
        if self.godown.value.item == old_name {
            self.godown.value.item = new_name;
        }
        for component in &mut self.crate_template.value {
            if component.pid.is_none() && component.item == old_name {
                component.item = new_name;
            }
        }
    }

    fn stock_delta(&self, pushed_m: u64) -> BTreeMap<&'static str, Row<Stock>> {
        self.stock
            .iter()
            .filter(|(_, row)| row.stamp.0 > pushed_m)
            .map(|(&key, &row)| (key, row))
            .collect()
    }
}

fn fixture() -> Legacy {
    Legacy {
        products: BTreeMap::from([(PID, Row::new(OLD, Stamp(92, 4, "catalog")))]),
        stock: BTreeMap::from([(OLD, OLD_STOCK), (PID.0, ID_STOCK)]),
        godown: Row::new(
            Godown {
                item: OLD,
                cases: 8,
                note: "back rack",
            },
            Stamp(99, 3, "warehouse"),
        ),
        crate_template: Row::new(
            [
                Component {
                    item: OLD,
                    pid: None,
                    qty: 2,
                },
                Component {
                    item: OLD,
                    pid: Some(PID),
                    qty: 5,
                },
            ],
            Stamp(100, 6, "packing"),
        ),
        sale: Row::new(
            SaleLine {
                item: OLD,
                pid: PID,
                qty: 3,
                rate: 20,
                amount: 60,
                cost_at_sale: 11,
            },
            Stamp(80, 2, "sales"),
        ),
        po: Row::new(
            PurchaseLine {
                item: OLD,
                pid: PID,
                ordered: 12,
                received: 5,
                rate: 13,
                receipts: [Receipt {
                    date: "2025-01-02",
                    qty: 5,
                    rate: 13,
                }],
            },
            Stamp(85, 8, "purchasing"),
        ),
        deleted: Vec::new(),
    }
}

fn cascaded_expected(mut before: Legacy, destination: Row<Stock>) -> Legacy {
    before.products.insert(PID, SAVED_PRODUCT);
    before.stock = BTreeMap::from([(NEW, destination), (PID.0, ID_STOCK)]);
    before.godown.value.item = NEW;
    before.crate_template.value[0].item = NEW;
    before
}

#[test]
fn l09_04_moves_legacy_key_when_destination_is_missing() {
    // Given a fresh fixture with no new-name key.
    let before = fixture();
    let mut after = before.clone();

    // When the existing product is saved under the new name.
    after.save_existing_rename(NEW, 200);

    // Then only the product is stamped; the rekeyed stock keeps its old envelope.
    assert_eq!(
        before.stock_delta(PUSHED_M),
        BTreeMap::from([(OLD, OLD_STOCK)])
    );
    assert_eq!(after, cascaded_expected(before, OLD_STOCK));
    assert_eq!(
        after.stock_delta(PUSHED_M),
        BTreeMap::from([(NEW, OLD_STOCK)])
    );
    assert!(after.stock_delta(101).is_empty());
}

#[test]
fn l09_04_drops_legacy_key_when_destination_is_occupied() {
    // Given a fresh fixture with a distinct destination stock row, not a second product.
    let mut before = fixture();
    before.stock.insert(NEW, TARGET_STOCK);
    let mut after = before.clone();

    // When the existing product is saved under the new name.
    after.save_existing_rename(NEW, 200);

    // Then the destination wins without merging, restamping, or emitting a deletion.
    assert_eq!(
        before.stock_delta(PUSHED_M),
        BTreeMap::from([(OLD, OLD_STOCK)])
    );
    assert_eq!(after, cascaded_expected(before, TARGET_STOCK));
    assert!(after.stock_delta(PUSHED_M).is_empty());
    assert_eq!(after.stock_delta(99), BTreeMap::from([(NEW, TARGET_STOCK)]));
}

#[test]
fn l09_04_skips_cascade_but_saves_product_when_exact_old_name_twin_exists() {
    // Given a fresh fixture with another product using the exact old name.
    let mut before = fixture();
    before
        .products
        .insert(ProductId("p10"), Row::new(OLD, Stamp(93, 2, "catalog")));
    let mut after = before.clone();

    // When p9 is saved under the new name.
    after.save_existing_rename(NEW, 200);

    // Then only p9 changes; the early return is from the cascade, not the save.
    let mut expected = before;
    expected.products.insert(PID, SAVED_PRODUCT);
    assert_eq!(after, expected);
    assert_eq!(
        after.stock_delta(PUSHED_M),
        BTreeMap::from([(OLD, OLD_STOCK)])
    );
}
