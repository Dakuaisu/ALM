use super::model::{Item, Line, Product, Sale, report};

fn product(
    id: &'static str,
    name: &'static str,
    opening: f64,
    buy: Option<f64>,
    layers: &[(f64, f64)],
) -> Product {
    Product {
        id,
        name,
        packet_qty: 10.0,
        opening,
        buy,
        layers: layers.to_vec(),
    }
}

fn line(pid: Option<&'static str>, name: &'static str, qty: f64) -> Line {
    Line {
        pid,
        name,
        qty,
        pieces: false,
        packet_qty: None,
    }
}

fn sale(date: &'static str, cancelled: bool, item: Item) -> Sale {
    Sale {
        id: None,
        date,
        cancelled,
        items: vec![item],
    }
}

#[test]
fn cancelled_sales_and_undo_change_dates_stock_and_rows() {
    // Given: one old active bill and one recent cancelled bill for the same stocked product.
    let products = [product("p", "Item", 5.0, Some(4.0), &[(5.0, 4.0)])];
    let mut old = sale(
        "2024-10-03",
        false,
        Item::Line(line(Some("p"), "Item", 1.0)),
    );
    let mut recent = sale("2024-10-05", true, Item::Line(line(Some("p"), "Item", 2.0)));
    old.id = Some("old-bill");
    recent.id = Some("recent-bill");
    let mut sales = vec![old, recent];
    // When: the same seeded bills are replayed before and after undo, and with only the cancelled bill.
    let both = report(&products, &sales);
    let cancelled_only = report(&products, &sales[1..]);
    sales[1].cancelled = false;
    let undone = report(&products, &sales);
    // Then: independently verify stock, last-sold keys and visible rows.
    assert_eq!(both.keys.get("p"), Some(&(1.0, "2024-10-03")));
    assert_eq!(
        sales.iter().map(|s| s.id).collect::<Vec<_>>(),
        vec![Some("old-bill"), Some("recent-bill")]
    );
    assert_eq!(both.candidates["p"], (4.0, Some("2024-10-03")));
    assert_eq!(
        (both.rows[0].on_hand, both.rows[0].last, both.rows[0].raw),
        (4.0, Some("2024-10-03"), 16.0)
    );
    assert_eq!(cancelled_only.keys.get("p"), None);
    assert_eq!(cancelled_only.candidates["p"], (5.0, None));
    assert_eq!(
        (cancelled_only.rows[0].on_hand, cancelled_only.rows[0].last),
        (5.0, None)
    );
    assert_eq!(undone.keys.get("p"), Some(&(3.0, "2024-10-05")));
    assert_eq!(undone.candidates["p"], (2.0, Some("2024-10-05")));
    assert_eq!(
        both.rows.iter().map(|r| r.id).collect::<Vec<_>>(),
        vec!["p"]
    );
    assert_eq!(
        cancelled_only.rows.iter().map(|r| r.id).collect::<Vec<_>>(),
        vec!["p"]
    );
    assert!(undone.rows.is_empty());
    assert_eq!(undone.empty, Some("No dead stock 👍"));
}

#[test]
fn raw_value_stable_cap_fifo_and_empty_are_distinct() {
    // Given: twelve high values, rounded ties with opposite FIFO ordering, and two zero ties.
    let mut products: Vec<_> = [
        "h01", "h02", "h03", "h04", "h05", "h06", "h07", "h08", "h09", "h10", "h11", "h12",
    ]
    .into_iter()
    .map(|id| product(id, id, 1.0, Some(20.0), &[(1.0, 1.0)]))
    .collect();
    products.extend([
        product(
            "fraction",
            "fraction",
            2.5,
            Some(4.25),
            &[(1.0, 20.0), (1.5, 30.0)],
        ),
        product("higher", "higher", 1.0, Some(10.75), &[(1.0, 2.0)]),
        product("missing", "missing", 1.0, None, &[(1.0, 7.0)]),
        product("zero", "zero", 1.0, Some(0.0), &[(1.0, 9.0)]),
    ]);
    // When: report both input orders; equal values retain their input order across the cap.
    let forward = report(&products, &[]);
    products.reverse();
    let reverse = report(&products, &[]);
    let empty_products = [
        product("no-stock", "no-stock", 0.0, Some(10.0), &[]),
        product("boundary", "boundary", 2.0, Some(10.0), &[]),
    ];
    let empty = report(
        &empty_products,
        &[sale(
            "2024-10-04",
            false,
            Item::Line(line(Some("boundary"), "boundary", 1.0)),
        )],
    );
    // Then: assert literal order, unrounded values, display and independent seeded FIFO sums.
    assert_eq!(
        forward.rows.iter().map(|r| r.id).collect::<Vec<_>>(),
        vec![
            "h01", "h02", "h03", "h04", "h05", "h06", "h07", "h08", "h09", "h10", "h11", "h12",
            "higher", "fraction", "missing"
        ]
    );
    assert_eq!(
        reverse.rows.iter().map(|r| r.id).collect::<Vec<_>>(),
        vec![
            "h12", "h11", "h10", "h09", "h08", "h07", "h06", "h05", "h04", "h03", "h02", "h01",
            "higher", "fraction", "zero"
        ]
    );
    assert_eq!(
        (
            forward.rows[12].raw,
            forward.rows[12].displayed.as_str(),
            forward.rows[12].fifo
        ),
        (10.75, "₹11", 2.0)
    );
    assert_eq!(
        (
            forward.rows[13].on_hand,
            forward.rows[13].raw,
            forward.rows[13].displayed.as_str(),
            forward.rows[13].fifo
        ),
        (2.5, 10.625, "₹11", 65.0)
    );
    assert_eq!(
        (
            forward.rows[14].raw,
            forward.rows[14].displayed.as_str(),
            forward.rows[14].fifo
        ),
        (0.0, "₹0", 7.0)
    );
    assert_eq!(
        (
            reverse.rows[14].raw,
            reverse.rows[14].displayed.as_str(),
            reverse.rows[14].fifo
        ),
        (0.0, "₹0", 9.0)
    );
    assert_eq!(empty.candidates["no-stock"], (0.0, None));
    assert_eq!(empty.candidates["boundary"], (1.0, Some("2024-10-04")));
    assert_eq!(empty.keys.get("boundary"), Some(&(1.0, "2024-10-04")));
    assert!(empty.rows.is_empty());
    assert_eq!(empty.empty, Some("No dead stock 👍"));
}

#[test]
fn identity_uses_stored_keys_even_when_packet_lookup_falls_back() {
    // Given: unique names only; old name survives on stable ID and crate component.
    let products = [
        product("renamed", "New name", 3.0, Some(1.0), &[]),
        product("current", "Current name", 3.0, Some(1.0), &[]),
        product("control", "Control", 3.0, Some(1.0), &[]),
    ];
    let mut stale = line(Some("stale"), "Current name", 5.0);
    stale.pieces = true;
    let controls = vec![
        sale("2024-10-05", false, Item::Line(stale)),
        sale(
            "2024-10-05",
            false,
            Item::Line(line(None, "Unmatched", 1.0)),
        ),
        sale("2024-10-05", false, Item::Line(line(None, "Control", 1.0))),
    ];
    // When: replay independent catalogs with no match, a direct match, and a crate match.
    let baseline = report(&products, &controls);
    let direct = Item::Line(line(Some("renamed"), "Old name", 1.0));
    let crate_item = Item::Crate {
        qty: 2.0,
        components: vec![line(Some("renamed"), "Old name", 0.5)],
    };
    assert_eq!(baseline.candidates["renamed"], (3.0, None));
    assert_eq!(
        baseline.rows.iter().map(|r| r.id).collect::<Vec<_>>(),
        vec!["renamed", "current"]
    );
    for (variant, item) in [("direct", direct), ("crate", crate_item)] {
        let mut sales = controls.clone();
        sales.push(sale("2024-10-05", false, item));
        let result = report(&products.clone(), &sales);
        println!(
            "model identity variant={variant} keys={:?} candidates={:?}",
            result.keys, result.candidates
        );
        // Then: each variant alone matches the same ID; stale and unmatched keys remain separate.
        assert_eq!(result.keys.get("renamed"), Some(&(1.0, "2024-10-05")));
        assert_eq!(result.candidates["renamed"], (2.0, Some("2024-10-05")));
        assert_eq!(result.keys.get("stale"), Some(&(0.5, "2024-10-05")));
        assert_eq!(result.keys.get("n:Unmatched"), Some(&(1.0, "2024-10-05")));
        assert_eq!(result.keys.get("control"), Some(&(1.0, "2024-10-05")));
        assert_eq!(result.keys.get("current"), None);
        assert_eq!(result.candidates["current"], (3.0, None));
        assert_eq!(
            result
                .rows
                .iter()
                .map(|r| (r.id, r.on_hand, r.last))
                .collect::<Vec<_>>(),
            vec![("current", 3.0, None)]
        );
    }
}
