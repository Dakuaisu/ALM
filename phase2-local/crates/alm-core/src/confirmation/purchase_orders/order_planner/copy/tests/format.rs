use super::{BELOW, NEW_LOW, OWNER, copy_payload, fixture, product, set_quote};

const LARGE_QUOTE: i64 = 168_884_986_026_394_800;

#[test]
fn l13_03_copy_cart_quote_uses_frozen_fmt_n() {
    let mut fixture = fixture();
    set_quote(&mut fixture, BELOW, LARGE_QUOTE);
    let mut plan = fixture.open_order_planner(OWNER).unwrap();
    assert!(plan.add_item(BELOW, Some(1)));

    let payload = copy_payload(&fixture, &plan);

    assert_eq!(
        payload.lines().nth(2),
        Some("Synthetic Lime & Soda — 1 pkt (Synthetic Orchard ₹1,68,88,49,86,02,63,948.2)")
    );
}

#[test]
fn l13_03_copy_catalog_quote_uses_frozen_fmt_n() {
    let mut fixture = fixture();
    set_quote(&mut fixture, BELOW, LARGE_QUOTE);
    let plan = fixture.open_order_planner(OWNER).unwrap();

    let payload = copy_payload(&fixture, &plan);

    assert_eq!(
        payload.lines().nth(3),
        Some("Synthetic Lime & Soda — 13 pkt (Synthetic Orchard ₹1,68,88,49,86,02,63,948.2)")
    );
}

#[test]
fn l13_03_copy_quantity_uses_frozen_fmt_n_at_signed_packet_limits() {
    let mut fixture = fixture();
    product(&mut fixture, (NEW_LOW, "Synthetic No Cost", 0, 0, 0));
    for (quantity, visible) in [
        (i64::MAX, "92,23,37,20,36,85,47,76,000"),
        (i64::MIN, "-92,23,37,20,36,85,47,76,000"),
    ] {
        let mut plan = fixture.open_order_planner(OWNER).unwrap();
        assert!(plan.add_item(NEW_LOW, Some(quantity)));

        let payload = copy_payload(&fixture, &plan);

        assert_eq!(
            payload.lines().nth(2),
            Some(format!("Synthetic No Cost — {visible} pkt").as_str())
        );
    }
}
