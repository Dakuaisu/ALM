use serde_json::{Value, json};

use super::model::{Db, Tick};
use super::receipt_rows::{Date, Receipt, ReceiveInput, Rupees};
use super::saved::Saved;

const PO: &str = "po-receipt";
const SUPPLIER: &str = "Synthetic Supplier";
const BEFORE: &str = "synthetic-before";
const AFTER: &str = "synthetic-receiver";
const LOG: &str = "lg-receipt";
const PUSHED: u64 = 100;
const NOW: u64 = 110;
const DATE: &str = "2025-02-12";

fn seed(second: &str) -> Value {
    json!({
        "products": [
            {"id":"1", "item":"Synthetic A", "buyRate":1, "avgCost":1, "sellRate":5,
                "margin":4, "supplierPrices":{(SUPPLIER):1}, "_v":1, "_m":100, "_d":BEFORE},
            {"id":"2", "item":second, "buyRate":2, "avgCost":2, "sellRate":5,
                "margin":3, "supplierPrices":{(SUPPLIER):2}, "_v":1, "_m":100, "_d":BEFORE}
        ],
        "stock": {
            "1":{"opening":2,"fridge":0,"_v":1,"_m":100,"_d":BEFORE},
            "2":{"opening":2,"fridge":0,"_v":1,"_m":100,"_d":BEFORE}
        },
        "purchaseOrders": [{"id":PO, "no":1, "date":"2025-02-11", "supplier":SUPPLIER,
            "supId":"s-receipt", "note":"Synthetic receipt fixture", "created":90, "cancelled":false,
            "items":[{"item":second, "pid":"2", "rate":2, "ordered":2, "received":0, "cancelled":0}],
            "_v":1,"_m":99,"_d":BEFORE}],
        "suppliers": [{"id":"s-receipt","name":SUPPLIER,"_v":1,"_m":99,"_d":BEFORE}],
        "priceSuppliers": [SUPPLIER],
        "meta": {"rev":4,"schema":1,"defaultReorder":5,"_v":1,"_m":99,"_d":BEFORE},
        "gstBills":[], "deleted":[], "log":[]
    })
}

fn expected_after(mut full: Value, duplicate: bool) -> Value {
    let cost_target = usize::from(!duplicate);
    let row = &mut full["products"][cost_target];
    row["buyRate"] = json!(3);
    row["avgCost"] = if duplicate { json!(2) } else { json!(2.5) };
    row["margin"] = json!(2);
    row["supplierPrices"][SUPPLIER] = json!(3);
    let stamped_product = &mut full["products"][1];
    stamped_product["_v"] = json!(2);
    stamped_product["_m"] = json!(NOW);
    stamped_product["_d"] = json!(AFTER);
    let po = &mut full["purchaseOrders"][0];
    po["_v"] = json!(2);
    po["_m"] = json!(NOW);
    po["_d"] = json!(AFTER);
    po["items"][0]["rate"] = json!(3);
    po["items"][0]["received"] = json!(2);
    po["items"][0]["receipts"] = json!([{"date":DATE,"qty":2,"rate":3}]);
    full["meta"]["rev"] = json!(5);
    full["log"] = json!([{"id":LOG,"ts":NOW,"date":DATE,"device":AFTER,"type":"grn",
        "msg":"PO-001 Synthetic Supplier — goods received","_v":1,"_m":NOW,"_d":AFTER}]);
    full
}

#[test]
fn l08_receipt_and_l20_delta_compare_duplicate_exact_name_and_unique_cases() {
    // Given: fixed catalog order, and a saved PO pointing to ID 2 (HTML 950-959, 6735-6755).
    for (case, second) in [("duplicate", "Synthetic A"), ("unique", "Synthetic B")] {
        let duplicate = second == "Synthetic A";
        let before = seed(second);
        let mut db: Db = serde_json::from_value(before.clone()).unwrap();
        let actual_before = serde_json::to_value(&db).unwrap();
        println!("{case} before expected={before} actual={actual_before}");
        assert_eq!(actual_before, before, "{case} pre-receipt full copy");
        assert_eq!(["1", "2"].map(|id| db.receipt_on_hand(id)), [2, 2]);
        assert!(db.changed(PUSHED, AFTER).is_empty());
        let date = Date(DATE.parse().unwrap());

        // When: receive once through the scoped handler (HTML 999-1031, 7024-7053).
        assert!(db.receive_po(
            PO,
            ReceiveInput {
                receipt: Receipt {
                    date,
                    qty: 2,
                    rate: Rupees::from_minor(300)
                },
                tick: Tick { m: NOW, d: AFTER },
                log_id: LOG,
                today: date,
            }
        ));

        // Then: compare full state and strict pushedM at the common-clock boundary (HTML 9451, 9472-9505).
        let expected = expected_after(before, duplicate);
        let actual = serde_json::to_value(&db).unwrap();
        println!("{case} after expected={expected} actual={actual}");
        assert_eq!(actual, expected, "{case} in-memory full copy");
        assert_eq!(["1", "2"].map(|id| db.receipt_on_hand(id)), [2, 4]);
        assert_eq!(
            ["Synthetic A", second].map(|name| db.receipt_on_hand_for_name(name)),
            if duplicate { [2, 2] } else { [2, 4] },
        );
        let envelope = |collection, id, version, body: &Value| json!({"collection":collection,"id":id,"v":version,"m":NOW,"d":AFTER,"json":body});
        let expected_delta = json!([
            envelope("purchaseOrders", PO, 2, &expected["purchaseOrders"][0]),
            envelope("products", "2", 2, &expected["products"][1]),
            envelope("log", LOG, 1, &expected["log"][0]),
        ]);
        let delta = db.changed(PUSHED, AFTER);
        let actual_delta = serde_json::to_value(&delta).unwrap();
        println!("{case} delta expected={expected_delta} actual={actual_delta}");
        assert_eq!(actual_delta, expected_delta);
        assert_eq!(db.changed(NOW - 1, AFTER), delta);
        assert!(db.changed(NOW, AFTER).is_empty());

        let saved = Saved::new();
        saved.write("receipt-full.json", &db);
        drop(db);
        let reopened: Db = saved.read("receipt-full.json");
        let reopened_json: Value = saved.read("receipt-full.json");
        println!("{case} reopen expected={expected} actual={reopened_json}");
        assert_eq!(reopened_json, expected);
        assert_eq!(serde_json::to_value(&reopened).unwrap(), expected);
        assert_eq!(reopened.changed(PUSHED, AFTER), delta);
        assert!(reopened.changed(NOW, AFTER).is_empty());
        assert_eq!(["1", "2"].map(|id| reopened.receipt_on_hand(id)), [2, 4]);
    }
}
