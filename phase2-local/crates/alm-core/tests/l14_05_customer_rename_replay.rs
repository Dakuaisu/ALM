// Static source model only: neither HTML nor any Rust app path is executed.
// ALM_Business_Manager.html:717-734, 776-783, 961-965, 7292-7313, 9451, 9472-9562.
#[path = "l14_05_customer_rename_replay/model.rs"]
mod model;
use model::{Database, Envelope, Record, apply, baseline, changed, rename};

#[test]
fn l14_05_customer_rename_replay() {
    let original = baseline();
    let mut rejected = original.clone();
    assert!(!rename(&mut rejected, "c-a", " shared "));
    assert_eq!(rejected, original);
    let mut captures = Vec::new();
    for (old_m, expected_ids) in [
        (99, vec![("customers", "c-a")]),
        (100, vec![("customers", "c-a")]),
        (
            101,
            vec![
                ("sales", "s-a"),
                ("sales", "s-cancel"),
                ("payments", "p-a"),
                ("customers", "c-a"),
            ],
        ),
    ] {
        let mut sender = original.clone();
        for row in sender.sales.iter_mut().chain(&mut sender.payments) {
            if row.customer_id.as_deref() == Some("c-a") {
                row.stamp.m = old_m;
            }
        }
        if old_m == 101 {
            let before = changed(&sender);
            assert_eq!(
                before.iter().map(|e| e.id.as_str()).collect::<Vec<_>>(),
                ["s-a", "s-cancel", "p-a"]
            );
            for entry in &before {
                match &entry.record {
                    Record::Sales(row) | Record::Payments(row) => {
                        assert_eq!(row.customer_name, "Shared")
                    }
                    Record::Customers(_) => panic!("unexpected customer"),
                }
            }
        }
        assert!(rename(&mut sender, "c-a", "  Renamed A  "));
        let snapshot: Database =
            serde_json::from_slice(&serde_json::to_vec(&sender).unwrap()).unwrap();
        let envelope: Vec<Envelope> =
            serde_json::from_slice(&serde_json::to_vec(&changed(&sender)).unwrap()).unwrap();
        let mut expected = original.clone();
        let customer = expected
            .customers
            .iter_mut()
            .find(|r| r.id == "c-a")
            .unwrap();
        customer.name = "Renamed A".into();
        customer.stamp = serde_json::from_str(r#"{"_m":200,"_v":5,"_d":"sender"}"#).unwrap();
        expected.meta.rev = 8;
        for row in expected.sales.iter_mut().chain(&mut expected.payments) {
            if row.customer_id.as_deref() == Some("c-a") {
                row.customer_name = "Renamed A".into();
                row.stamp.m = old_m;
            }
        }
        assert_eq!(snapshot, expected);
        assert_eq!(
            envelope
                .iter()
                .map(|e| (
                    match &e.record {
                        Record::Sales(_) => "sales",
                        Record::Payments(_) => "payments",
                        Record::Customers(_) => "customers",
                    },
                    e.id.as_str()
                ))
                .collect::<Vec<_>>(),
            expected_ids
        );
        if old_m == 101 {
            let literal: Vec<Envelope> = serde_json::from_str(r#"[
                {"collection":"sales","id":"s-a","json":{"id":"s-a","customerId":"c-a","customerName":"Renamed A","cancelled":false,"_m":101,"_v":4,"_d":"seed"},"v":4,"m":101,"d":"seed"},
                {"collection":"sales","id":"s-cancel","json":{"id":"s-cancel","customerId":"c-a","customerName":"Renamed A","cancelled":true,"_m":101,"_v":4,"_d":"seed"},"v":4,"m":101,"d":"seed"},
                {"collection":"payments","id":"p-a","json":{"id":"p-a","customerId":"c-a","customerName":"Renamed A","_m":101,"_v":4,"_d":"seed"},"v":4,"m":101,"d":"seed"},
                {"collection":"customers","id":"c-a","json":{"id":"c-a","name":"Renamed A","_m":200,"_v":5,"_d":"sender"},"v":5,"m":200,"d":"sender"}
            ]"#).unwrap();
            assert_eq!(envelope, literal);
        }
        println!(
            "L14-05 _m={old_m}: snapshot={}, unsent={}",
            serde_json::to_string(&snapshot).unwrap(),
            serde_json::to_string(&envelope).unwrap()
        );
        captures.push(envelope);
    }
    let mut customer_only = original.clone();
    assert_eq!(apply(&mut customer_only, &captures[0]), [true]);
    let mut customer_expected = original.clone();
    *customer_expected
        .customers
        .iter_mut()
        .find(|r| r.id == "c-a")
        .unwrap() =
        serde_json::from_str(r#"{"id":"c-a","name":"Renamed A","_m":200,"_v":5,"_d":"sender"}"#)
            .unwrap();
    assert_eq!(customer_only, customer_expected);

    let mut equal = original.clone();
    for row in equal
        .sales
        .iter_mut()
        .chain(&mut equal.payments)
        .filter(|r| r.customer_id.as_deref() == Some("c-a"))
    {
        row.stamp.m = 50;
        row.stamp.d = "receiver".into();
    }
    let mut equal_expected = equal.clone();
    *equal_expected
        .customers
        .iter_mut()
        .find(|r| r.id == "c-a")
        .unwrap() = customer_expected
        .customers
        .iter()
        .find(|r| r.id == "c-a")
        .unwrap()
        .clone();
    assert_eq!(apply(&mut equal, &captures[2]), [false, false, false, true]);
    assert_eq!(equal, equal_expected);

    let mut higher = original.clone();
    for row in higher
        .sales
        .iter_mut()
        .chain(&mut higher.payments)
        .filter(|r| r.customer_id.as_deref() == Some("c-a"))
    {
        row.stamp.m = 300;
        row.stamp.v = 3;
        row.stamp.d = "receiver".into();
    }
    let higher_expected: Database = serde_json::from_str(r#"{
        "customers":[{"id":"c-b","name":"Shared","_m":91,"_v":3,"_d":"seed"},{"id":"c-a","name":"Renamed A","_m":200,"_v":5,"_d":"sender"}],
        "sales":[{"id":"s-a","customerId":"c-a","customerName":"Renamed A","cancelled":false,"_m":101,"_v":4,"_d":"seed"},{"id":"s-cancel","customerId":"c-a","customerName":"Renamed A","cancelled":true,"_m":101,"_v":4,"_d":"seed"},{"id":"s-b","customerId":"c-b","customerName":"Shared","_m":80,"_v":2,"_d":"seed"},{"id":"s-none","customerName":"Shared","_m":81,"_v":2,"_d":"seed"}],
        "payments":[{"id":"p-a","customerId":"c-a","customerName":"Renamed A","_m":101,"_v":4,"_d":"seed"},{"id":"p-b","customerId":"c-b","customerName":"Shared","_m":82,"_v":2,"_d":"seed"},{"id":"p-none","customerName":"Shared","_m":83,"_v":2,"_d":"seed"}],
        "meta":{"rev":7,"_m":90,"_v":2,"_d":"seed"}
    }"#).unwrap();
    assert_eq!(apply(&mut higher, &captures[2]), [true, true, true, true]);
    assert_eq!(higher, higher_expected);
    println!(
        "L14-05 receivers: customer-only={customer_only:?}; equal={equal:?}; higher={higher:?}"
    );
}
