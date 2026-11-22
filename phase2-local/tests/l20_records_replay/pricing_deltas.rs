use std::collections::BTreeMap;

use super::super::model::{Db, DeleteTick, Envelope, Origin, Record};
use super::super::saved::Saved;
use super::{PRODUCT, RECEIVER, RETIRED, RETIRED_ID, SENDER, product, supplier, tick};

const NEW: &str = "New Price Supplier";
const QUOTE_ONLY: &str = "Quote Only";

#[test]
fn l20_08_deltas_carry_quotes_and_contacts_but_not_raw_columns_or_retention_effect() {
    // Given: PHASE1_CASES L20-08 / HTML 7750-7758, 7910, 7941-7948, 9472-9562, 10466-10479.
    let old_price = product(&[(RETIRED, Some(510))]);
    let mut sender = Db {
        price_suppliers: vec![QUOTE_ONLY.into()],
        suppliers: vec![supplier(RETIRED_ID, RETIRED)],
        products: vec![old_price.clone()],
        ..Db::default()
    };
    let mut receiver = Db {
        price_suppliers: vec!["Receiver Raw".into()],
        suppliers: vec![
            supplier("s-old", "Existing Contact"),
            supplier(RETIRED_ID, RETIRED),
        ],
        products: vec![old_price],
        ..Db::default()
    };
    assert_eq!(sender.columns(), [QUOTE_ONLY, RETIRED]);
    sender.set_quote(PRODUCT, RETIRED, Some(630), tick(20));
    sender.set_quote(PRODUCT, QUOTE_ONLY, Some(740), tick(20));
    assert!(sender.add_pricing_supplier("s-new", NEW, tick(30)));

    // When: the divergent receiver applies ordinary deltas, then a separately collected contact tombstone.
    let outgoing = sender.changed(10, SENDER);
    let saved = Saved::new();
    saved.write("delta.json", &outgoing);
    let script: Vec<Envelope> = saved.read("delta.json");
    assert_eq!(script, outgoing);
    let [price, contact] = script.as_slice() else {
        panic!("expected only product and supplier envelopes")
    };
    assert!(matches!(price.record, Record::Products(_)));
    assert!(matches!(contact.record, Record::Suppliers(_)));
    assert_eq!(
        (&*price.id, price.v, price.m, &*price.d),
        (PRODUCT, 5, 20, SENDER)
    );
    assert_eq!(
        (&*contact.id, contact.v, contact.m, &*contact.d),
        ("s-new", 1, 30, SENDER)
    );
    assert!(receiver.apply(price, RECEIVER, Origin::Pull));
    assert_eq!(
        receiver.columns(),
        ["Receiver Raw", "Existing Contact", RETIRED]
    );
    assert!(receiver.apply(contact, RECEIVER, Origin::Pull));
    assert_eq!(
        receiver.columns(),
        ["Receiver Raw", NEW, "Existing Contact", RETIRED]
    );
    sender.delete_contact(
        RETIRED_ID,
        DeleteTick {
            ts: 39,
            write: tick(40),
        },
    );
    let deletes = sender.changed(30, SENDER);
    let [delete] = deletes.as_slice() else {
        panic!("only the tombstone is newly eligible")
    };
    assert!(matches!(delete.record, Record::Deleted(_)));
    assert!(receiver.apply(delete, RECEIVER, Origin::Pull));
    assert!(!receiver.apply(delete, RECEIVER, Origin::Pull));

    // Then: branch-specific raw and displayed columns differ while the product quote maps agree.
    let quotes = BTreeMap::from([(RETIRED.into(), Some(630)), (QUOTE_ONLY.into(), Some(740))]);
    for (file, db, raw, display) in [
        (
            "sender.json",
            sender,
            &[QUOTE_ONLY, NEW, RETIRED][..],
            &[QUOTE_ONLY, NEW, RETIRED][..],
        ),
        (
            "receiver.json",
            receiver,
            &["Receiver Raw"][..],
            &["Receiver Raw", NEW, "Existing Contact"][..],
        ),
    ] {
        saved.write(file, &db);
        let readback = saved.read::<Db>(file);
        assert_eq!(readback, db);
        assert_eq!(readback.price_suppliers, raw);
        assert_eq!(readback.columns(), display);
        assert_eq!(readback.products[0].value.data.supplier_prices, quotes);
        assert_eq!(readback.deleted[0].value.data.rid, RETIRED_ID);
    }
}
