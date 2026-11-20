//! L09-02/L09-03 historical references (docs/PHASE1_CASES.md:617).
//! Disposable in-memory model; the legacy HTML and the application are not run.

use std::collections::BTreeMap;

const TARGET: ProductId = ProductId("p9");
const CONTROL: ProductId = ProductId("p10");
const SAME_NAME: ProductId = ProductId("p8");
const ITEM: &str = "Synthetic Orange 250ml";
const CONTROL_ITEM: &str = "Synthetic Lime 250ml";
const NOW: u64 = 200;
const DEVICE: &str = "delete-fixture";
const LINKED_RECEIPT: Receipt = Receipt {
    date: "2025-01-02",
    qty: 5,
    rate: Money(14),
};
const NAME_ONLY_RECEIPT: Receipt = Receipt {
    date: "2025-01-03",
    qty: 2,
    rate: Money(17),
};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ProductId(&'static str);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Money(u32);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct LineRef {
    pid: Option<ProductId>,
    item: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct BillLine {
    product: LineRef,
    qty: u32,
    mrp: Money,
    rate: Money,
    amount: Money,
    cost_at_sale: Money,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Receipt {
    date: &'static str,
    qty: u32,
    rate: Money,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PoLine {
    product: LineRef,
    qty: u32,
    received: u32,
    rate: Money,
    receipts: [Receipt; 1],
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

#[derive(Clone, Debug, Eq, PartialEq)]
struct Tombstone {
    id: String,
    coll: &'static str,
    rid: ProductId,
    ts: u64,
    modified: u64,
    version: u32,
    device: &'static str,
}

enum Role {
    Manager,
    Owner,
}

enum Decision {
    Decline,
    Confirm,
}

#[derive(Debug, Eq, PartialEq)]
enum DeleteOutcome {
    Declined,
    AccessDenied,
    Deleted,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Legacy {
    products: BTreeMap<ProductId, &'static str>,
    bill: [BillLine; 2],
    po: [PoLine; 2],
    stock: BTreeMap<ProductId, Stock>,
    deleted: Vec<Tombstone>,
}

impl Legacy {
    fn delete_product(&mut self, pid: ProductId, role: Role, decision: Decision) -> DeleteOutcome {
        // Manager denial is the local fixture policy, not a check in the legacy dialog (7069–7088).
        // The confirmed callback removes the master and adds a fresh tombstone (7769–7779, 974–998).
        match (role, decision) {
            (Role::Owner | Role::Manager, Decision::Decline) => DeleteOutcome::Declined,
            (Role::Manager, Decision::Confirm) => DeleteOutcome::AccessDenied,
            (Role::Owner, Decision::Confirm) => {
                self.products.remove(&pid);
                self.deleted.push(Tombstone {
                    id: format!("products:{}", pid.0),
                    coll: "products",
                    rid: pid,
                    ts: NOW,
                    modified: NOW,
                    version: 1,
                    device: DEVICE,
                });
                DeleteOutcome::Deleted
            }
        }
    }

    fn prod_by_id(&self, pid: ProductId) -> Option<&'static str> {
        self.products.get(&pid).copied()
    }

    fn prod_by_item(&self, item: &str) -> Option<ProductId> {
        self.products
            .iter()
            .find_map(|(&id, &name)| (name == item).then_some(id))
    }

    // Lookup falls back to the exact stored name if the ID has no master (950–959).
    fn prod_of_line(&self, line: LineRef) -> Option<ProductId> {
        line.pid
            .filter(|&id| self.prod_by_id(id).is_some())
            .or_else(|| self.prod_by_item(line.item))
    }

    // Packet lines and PO receipts keep a stored PID even when lookup finds another master
    // (1040–1074, 1198–1214). These fixture lines are ordinary packets and dated, rated receipts.
    fn history_key(&self, line: LineRef) -> String {
        line.pid
            .or_else(|| self.prod_by_item(line.item))
            .map_or_else(|| format!("n:{}", line.item), |pid| pid.0.to_owned())
    }

    fn history_at(&self, key: &str) -> (u32, u32, Vec<Receipt>) {
        let sold = self
            .bill
            .iter()
            .filter_map(|line| (self.history_key(line.product) == key).then_some(line.qty))
            .sum();
        let mut received = 0;
        let mut lots = Vec::new();
        for line in &self.po {
            if self.history_key(line.product) == key {
                received += line.received;
                lots.extend(line.receipts);
            }
        }
        (sold, received, lots)
    }
}

fn fixture() -> Legacy {
    let linked = LineRef {
        pid: Some(TARGET),
        item: ITEM,
    };
    let name_only = LineRef {
        pid: None,
        ..linked
    };
    let bill = BillLine {
        product: linked,
        qty: 3,
        mrp: Money(25),
        rate: Money(20),
        amount: Money(60),
        cost_at_sale: Money(11),
    };
    Legacy {
        products: BTreeMap::from([(TARGET, ITEM), (CONTROL, CONTROL_ITEM)]),
        bill: [
            bill,
            BillLine {
                product: name_only,
                qty: 2,
                amount: Money(40),
                ..bill
            },
        ],
        po: [
            PoLine {
                product: linked,
                qty: 12,
                received: 5,
                rate: Money(13),
                receipts: [LINKED_RECEIPT],
            },
            PoLine {
                product: name_only,
                qty: 4,
                received: 2,
                rate: Money(16),
                receipts: [NAME_ONLY_RECEIPT],
            },
        ],
        stock: BTreeMap::from([(TARGET, Stock::new(42, 6)), (CONTROL, Stock::new(17, 4))]),
        deleted: Vec::new(),
    }
}

#[test]
fn decline_preserves_the_whole_fixture() {
    // Given a fresh fixture with a linked product.
    let before = fixture();
    let mut after = before.clone();

    // When the Owner declines the delete confirmation.
    let outcome = after.delete_product(TARGET, Role::Owner, Decision::Decline);

    // Then no modeled record, stock entry, or tombstone changes.
    assert_eq!(outcome, DeleteOutcome::Declined);
    assert_eq!(after, before);
}

#[test]
fn manager_confirmation_preserves_the_whole_fixture() {
    // Given a fresh fixture with a linked product.
    let before = fixture();
    let mut after = before.clone();

    // When a Manager attempts to confirm deletion.
    let outcome = after.delete_product(TARGET, Role::Manager, Decision::Confirm);

    // Then access is denied with no change to the fixture.
    assert_eq!(outcome, DeleteOutcome::AccessDenied);
    assert_eq!(after, before);
}

#[test]
fn owner_confirmation_removes_only_the_master_without_a_name_match() {
    // Given an existing product, its history, and an unrelated control.
    let before = fixture();
    let mut after = before.clone();

    // When the Owner confirms deletion.
    let outcome = after.delete_product(TARGET, Role::Owner, Decision::Confirm);

    // Then only the master list and tombstones change; all stored history and stock remain.
    assert_eq!(outcome, DeleteOutcome::Deleted);
    assert_eq!(
        after,
        Legacy {
            products: BTreeMap::from([(CONTROL, CONTROL_ITEM)]),
            deleted: vec![Tombstone {
                id: "products:p9".to_owned(),
                coll: "products",
                rid: TARGET,
                ts: 200,
                modified: 200,
                version: 1,
                device: "delete-fixture",
            }],
            ..before
        },
    );
    assert_eq!(after.prod_by_id(TARGET), None);
    assert_eq!(after.prod_by_id(CONTROL), Some(CONTROL_ITEM));
    for (bill, po) in after.bill.iter().zip(&after.po) {
        assert_eq!(after.prod_of_line(bill.product), None);
        assert_eq!(after.prod_of_line(po.product), None);
    }
    assert_history_keys(&after, "n:Synthetic Orange 250ml");
}

#[test]
fn exact_name_fallback_resolves_a_different_id_without_rekeying_history() {
    // Given a separate fresh copy with a distinct ID using the exact same name.
    let mut after = fixture();
    after.products.insert(SAME_NAME, ITEM);
    assert_eq!(after.prod_of_line(after.bill[0].product), Some(TARGET));
    assert_eq!(after.prod_of_line(after.po[0].product), Some(TARGET));

    // When the Owner confirms deletion of the original ID.
    let outcome = after.delete_product(TARGET, Role::Owner, Decision::Confirm);

    // Then both line types resolve by name, but only name-only history aggregates at p8.
    assert_eq!(outcome, DeleteOutcome::Deleted);
    assert_eq!(after.prod_by_id(TARGET), None);
    for (bill, po) in after.bill.iter().zip(&after.po) {
        assert_eq!(after.prod_of_line(bill.product), Some(SAME_NAME));
        assert_eq!(after.prod_of_line(po.product), Some(SAME_NAME));
    }
    assert_history_keys(&after, "p8");
    assert!(!after.stock.contains_key(&SAME_NAME));
}

fn assert_history_keys(legacy: &Legacy, name_key: &str) {
    assert_eq!(legacy.history_at("p9"), (3, 5, vec![LINKED_RECEIPT]));
    assert_eq!(legacy.history_at(name_key), (2, 2, vec![NAME_ONLY_RECEIPT]));
}
