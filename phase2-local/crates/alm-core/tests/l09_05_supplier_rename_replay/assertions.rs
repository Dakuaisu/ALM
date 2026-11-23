use super::*;

fn decoded<T: Serialize + for<'de> Deserialize<'de>>(value: &T) -> T {
    serde_json::from_slice(&serde_json::to_vec(value).unwrap()).unwrap()
}

fn expected_snapshot(m: u64) -> Database {
    let mut expected: Database = serde_json::from_value(json!({
        "suppliers":[
            {"id":"s2","name":"Target","mobile":"2","note":"other","_m":91,"_v":1,"_d":"seed"},
            {"id":"s1","name":"Target","mobile":"1","note":"note","_m":200,"_v":3,"_d":"sender"}
        ],
        "purchases":[
            {"id":"p1","supId":"other","supplier":"Target","_m":99,"_v":3,"_d":"seed"},
            {"id":"p2","supId":"s1","supplier":"old","_m":99,"_v":3,"_d":"seed"}
        ],
        "purchaseOrders":[
            {"id":"o1","supId":"other","supplier":"Target","_m":99,"_v":3,"_d":"seed"},
            {"id":"o2","supId":"s1","supplier":"old","_m":99,"_v":3,"_d":"seed"}
        ],
        "products":[
            {"id":"missing-target","supplierPrices":{"Target":11},"_m":99,"_v":3,"_d":"seed"},
            {"id":"null-target","supplierPrices":{"Target":12},"_m":99,"_v":3,"_d":"seed"},
            {"id":"existing-target","supplierPrices":{"Target":44},"_m":99,"_v":3,"_d":"seed"},
            {"id":"zero-target","supplierPrices":{"Target":0},"_m":99,"_v":3,"_d":"seed"},
            {"id":"null-old","supplierPrices":{"Old":null,"Target":55},"_m":99,"_v":3,"_d":"seed"}
        ],
        "priceSuppliers":["Target","Other","other"],
        "meta":{"rev":8,"_m":90,"_v":2,"_d":"seed"}
    }))
    .unwrap();
    expected.purchases[0].stamp.m = m;
    expected.purchase_orders[0].stamp.m = m;
    expected.products[0].stamp.m = m;
    expected
}

fn expected_payload(full: bool) -> Vec<Envelope> {
    let supplier = json!({"collection":"suppliers","id":"s1","json":{"id":"s1","name":"Target","mobile":"1","note":"note","_m":200,"_v":3,"_d":"sender"},"v":3,"m":200,"d":"sender"});
    let mut rows = if full {
        vec![
            json!({"collection":"purchaseOrders","id":"o1","json":{"id":"o1","supId":"other","supplier":"Target","_m":101,"_v":3,"_d":"seed"},"v":3,"m":101,"d":"seed"}),
            json!({"collection":"purchases","id":"p1","json":{"id":"p1","supId":"other","supplier":"Target","_m":101,"_v":3,"_d":"seed"},"v":3,"m":101,"d":"seed"}),
            json!({"collection":"products","id":"missing-target","json":{"id":"missing-target","supplierPrices":{"Target":11},"_m":101,"_v":3,"_d":"seed"},"v":3,"m":101,"d":"seed"}),
        ]
    } else {
        vec![]
    };
    rows.push(supplier);
    serde_json::from_value(json!(rows)).unwrap()
}

fn receiver_seed(m: u64, v: u64) -> Database {
    let mut receiver = baseline();
    for stamp in [
        &mut receiver.purchases[0].stamp,
        &mut receiver.purchase_orders[0].stamp,
        &mut receiver.products[0].stamp,
    ] {
        stamp.m = m;
        stamp.v = v;
    }
    receiver
}

fn receiver_batch(payload: &[Envelope], full: bool, (m, v): (u64, u64)) {
    let mut receiver = receiver_seed(m, v);
    let decisions: Vec<bool> = payload
        .iter()
        .map(|entry| apply(&mut receiver, entry))
        .collect();
    let expected_decisions = if full {
        vec![v == 2, v == 2, v == 2, true]
    } else {
        vec![true]
    };
    assert_eq!(decisions, expected_decisions);
    let mut expected = receiver_seed(m, v);
    expected.suppliers[1] = serde_json::from_value(
        json!({"id":"s1","name":"Target","mobile":"1","note":"note","_m":200,"_v":3,"_d":"sender"}),
    )
    .unwrap();
    if full && v == 2 {
        expected.purchases[0] = serde_json::from_value(
            json!({"id":"p1","supId":"other","supplier":"Target","_m":101,"_v":3,"_d":"seed"}),
        )
        .unwrap();
        expected.purchase_orders[0] = serde_json::from_value(
            json!({"id":"o1","supId":"other","supplier":"Target","_m":101,"_v":3,"_d":"seed"}),
        )
        .unwrap();
        expected.products[0] = serde_json::from_value(json!({"id":"missing-target","supplierPrices":{"Target":11},"_m":101,"_v":3,"_d":"seed"})).unwrap();
    }
    assert_eq!(receiver, expected);
    assert_eq!(
        receiver.price_suppliers,
        ["Target", "Old", "Other", "Old", "other", "Other"]
    );
    println!(
        "SOURCE MODEL receiver full={full} local_m={m} local_v={v} decisions={decisions:?} rows={receiver:?}"
    );
}

#[test]
fn source_model_save_snapshot_payload_and_receiver() {
    // Given: the matching supplier is not the first row and the target already exists.
    let seed = baseline();
    let mut blank = seed.clone();
    assert!(!save(&mut blank, "s1", "  "));
    assert_eq!(blank, seed);

    for m in [99, 100, 101] {
        let mut sender = seed.clone();
        sender.purchases[0].stamp.m = m;
        sender.purchase_orders[0].stamp.m = m;
        sender.products[0].stamp.m = m;
        if m == 101 {
            let before: Vec<Envelope> = decoded(&changed(&sender));
            let expected: Vec<Envelope> = serde_json::from_value(json!([
                {"collection":"purchaseOrders","id":"o1","json":{"id":"o1","supId":"other","supplier":"Old","_m":101,"_v":3,"_d":"seed"},"v":3,"m":101,"d":"seed"},
                {"collection":"purchases","id":"p1","json":{"id":"p1","supId":"other","supplier":"Old","_m":101,"_v":3,"_d":"seed"},"v":3,"m":101,"d":"seed"},
                {"collection":"products","id":"missing-target","json":{"id":"missing-target","supplierPrices":{"Old":11},"_m":101,"_v":3,"_d":"seed"},"v":3,"m":101,"d":"seed"}
            ])).unwrap();
            assert_eq!(before, expected);
            println!("SOURCE MODEL pre-save payload={before:?}");
        }
        // When: Save runs and each representation is decoded independently.
        assert!(save(&mut sender, "s1", " Target "));
        let snapshot: Database = decoded(&sender);
        let payload: Vec<Envelope> = decoded(&changed(&sender));
        // Then: literal snapshot and payload expectations retain the original stamps.
        assert_eq!(snapshot, expected_snapshot(m));
        assert_eq!(payload, expected_payload(m == 101));
        println!(
            "SOURCE MODEL watermark=100 dependent_m={m} snapshot={snapshot:?} payload={payload:?}"
        );
        if m == 99 || m == 101 {
            receiver_batch(&payload, m == 101, (50, 3));
            receiver_batch(&payload, m == 101, (300, 2));
        }
    }
    println!("SOURCE MODEL only; no legacy application or network execution");
}
