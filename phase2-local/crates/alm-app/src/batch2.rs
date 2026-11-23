//! Batch 2 integrated check, following milestone_b_replay / common/partial_po and the L10 crate
//! FIFO replay on a zero-opening fixture: a quote change through a plan and PO (no stock), two
//! dated partial receipts at different rates, closure of the remainder, a crate dispatched from
//! those lots, a count adjustment, and the Stock / Godown / Reorder screens, with no costs for
//! the Manager anywhere on the path.
use std::collections::HashMap;
use std::num::NonZeroU32;

use alm_core::confirmation::demo_fixture::seed_purchasing;
use alm_core::confirmation::{
    ConfirmationFixture, CrateTemplate, PhysicalCountInput, ProductTerms, PurchaseOrderStatus,
    RequestId, StockCountReason,
};
use alm_core::{
    BillDate, CrateComponent, DraftCustomer, DraftCustomerId, DraftId, Money, ProductId,
    ProvisionalDraft, Role, SaleUnit,
};

use crate::Trial;
use crate::screen_tests::Root;

const LIME: ProductId = ProductId::new(1);
const SODA: ProductId = ProductId::new(2);
const OWNER: Role = Role::Owner;

const fn money(paise: i64) -> Money {
    Money::from_minor_units(paise)
}

fn may(day: u8) -> BillDate {
    BillDate::new(2025, 5, day).unwrap()
}

fn zero_opening() -> ConfirmationFixture {
    let mut f = ConfirmationFixture::new(LIME, 0, money(12_050)).unwrap();
    f.add_product(SODA, 0, money(6_250)).unwrap();
    for (id, item, level, price) in [
        (LIME, "Synthetic Lime", 4, 12_050),
        (SODA, "Synthetic Soda", 3, 6_250),
    ] {
        let mut form = f.open_product_entry(OWNER, id).unwrap();
        form.item = item.into();
        form.reorder_level = Some(level);
        f.save_product_entry(OWNER, &form).unwrap();
        let pack = NonZeroU32::MIN;
        f.set_checkout_product(
            id,
            ProductTerms {
                pack,
                piece_price: money(price),
                case_price: money(price),
            },
        )
        .unwrap();
    }
    seed_purchasing(&mut f).unwrap();
    f
}

type Lots = (u32, Vec<(Option<BillDate>, u32, i64)>, Option<i64>);

fn lots(trial: &Trial, id: ProductId) -> Lots {
    let lots = trial.authority.open_stock_lots(OWNER, id).unwrap();
    (
        lots.on_hand,
        lots.rows
            .iter()
            .map(|lot| {
                (
                    lot.date,
                    lot.remaining.get(),
                    lot.unit_cost.unwrap().minor_units(),
                )
            })
            .collect(),
        lots.total_value.map(Money::minor_units),
    )
}

fn map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

#[test]
fn quote_plan_po_partial_receipts_crate_dispatch_and_count_reconcile() {
    let root = Root::new("batch2");
    let mut trial = Trial::open(&root.0, Some(may(1))).unwrap();
    trial.authority = zero_opening();
    let empty = (0, vec![], Some(0));

    // Quote change: Wholesale drops Soda to ₹35, overtaking Depot's ₹36; no stock moves.
    let wholesale = trial.authority.purchase_suppliers(OWNER).unwrap()[0].clone();
    assert_eq!(wholesale.name, "Synthetic Wholesale");
    let before = trial
        .authority
        .compare_supplier_prices(OWNER, Some(SODA))
        .unwrap()
        .rows
        .remove(0);
    assert_eq!(before.best.unwrap().1, money(3_600));
    trial
        .supplier_quotes(map(&[
            ("product", "2"),
            (&format!("s{}", wholesale.id.index()), "35"),
        ]))
        .unwrap();
    let after = trial
        .authority
        .compare_supplier_prices(OWNER, Some(SODA))
        .unwrap()
        .rows
        .remove(0);
    assert_eq!(after.best, Some((wholesale.id, money(3_500))));
    assert_eq!(
        (lots(&trial, LIME), lots(&trial, SODA)),
        (empty.clone(), empty.clone())
    );

    // Plan both items from the same best supplier and save a PO; still no stock.
    let line = |product: &str, qty: &str| crate::order::Line {
        product: product.into(),
        qty: qty.into(),
    };
    trial.order_add(&line("1", "6")).unwrap();
    trial.order_add(&line("2", "10")).unwrap();
    assert_eq!(
        trial.order.planner.as_ref().unwrap().cart_total().unwrap(),
        money(6 * 6_900 + 10 * 3_500)
    );
    trial.order_checkout().unwrap();
    assert_eq!(
        trial
            .order
            .draft
            .as_ref()
            .unwrap()
            .supplier
            .as_ref()
            .unwrap()
            .name,
        wholesale.name
    );
    trial
        .order_po(&crate::order::Po {
            supplier: wholesale.name.clone(),
            date: "2025-05-01".into(),
            note: String::new(),
            paste: String::new(),
            intent: "save".into(),
        })
        .unwrap();
    assert_eq!(
        (lots(&trial, LIME), lots(&trial, SODA)),
        (empty.clone(), empty)
    );
    let reorder: Vec<_> = trial
        .authority
        .open_reorder_planner(OWNER)
        .unwrap()
        .iter()
        .map(|r| r.product)
        .collect();
    assert_eq!(reorder, [LIME, SODA]);

    // Two dated partial receipts at different rates, then close the unreceived remainder.
    trial
        .po_receive_save(map(&[
            ("po", "0"),
            ("date", "2025-05-01"),
            ("qty_0", "4"),
            ("rate_0", "69"),
            ("qty_1", "3"),
            ("rate_1", "35"),
        ]))
        .unwrap();
    trial
        .po_receive_save(map(&[
            ("po", "0"),
            ("date", "2025-05-03"),
            ("qty_0", "0"),
            ("rate_0", ""),
            ("qty_1", "4"),
            ("rate_1", "37.50"),
        ]))
        .unwrap();
    let id = trial.purchase_id(0).unwrap();
    let open = trial
        .authority
        .purchase_order_list(OWNER)
        .unwrap()
        .rows
        .remove(0);
    assert_eq!(
        (open.status, open.received, open.remaining),
        (PurchaseOrderStatus::PartiallyReceived, 11, 5)
    );
    trial.authority.close_remaining(OWNER, id, true).unwrap();
    let closed = trial
        .authority
        .purchase_order_list(OWNER)
        .unwrap()
        .rows
        .remove(0);
    assert_eq!(
        (
            closed.status,
            closed.ordered,
            closed.received,
            closed.remaining
        ),
        (PurchaseOrderStatus::Completed, 16, 11, 0)
    );
    assert_eq!(
        lots(&trial, LIME),
        (4, vec![(Some(may(1)), 4, 6_900)], Some(27_600))
    );
    assert_eq!(
        lots(&trial, SODA),
        (
            7,
            vec![(Some(may(1)), 3, 3_500), (Some(may(3)), 4, 3_750)],
            Some(25_500)
        )
    );

    // Crate dispatch in the Owner simulation: 2 crates of (2 cs Soda + 1 cs Lime) on 05-04.
    let template = trial
        .authority
        .save_crate_template(
            OWNER,
            CrateTemplate {
                name: "Synthetic Party Crate".into(),
                components: vec![
                    CrateComponent::new(SODA, NonZeroU32::new(2).unwrap(), SaleUnit::Case),
                    CrateComponent::new(LIME, NonZeroU32::MIN, SaleUnit::Case),
                ],
                price: money(30_000),
            },
        )
        .unwrap();
    let mut draft = ProvisionalDraft::new(DraftId::new(1), vec![])
        .with_bill_date(Some(may(4)))
        .with_customer(DraftCustomer::Selected {
            id: DraftCustomerId::new("batch2-buyer".into()),
            name: "Synthetic Crate Buyer".into(),
        });
    trial
        .authority
        .load_crate_template(OWNER, template, NonZeroU32::new(2).unwrap(), &mut draft)
        .unwrap();
    trial.authority.register_draft(draft).unwrap();
    assert!(
        trial
            .authority
            .review_checkout(Role::Manager, DraftId::new(1), &[], money(60_000))
            .is_err()
    );
    let review = trial
        .authority
        .review_checkout(OWNER, DraftId::new(1), &[], money(60_000))
        .unwrap();
    let receipt = trial
        .authority
        .confirm(OWNER, RequestId::new(1), &review)
        .unwrap();
    assert_eq!((receipt.quantity, receipt.total), (6, money(60_000)));
    assert_eq!(
        lots(&trial, SODA),
        (3, vec![(Some(may(3)), 3, 3_750)], Some(11_250))
    );
    assert_eq!(
        lots(&trial, LIME),
        (2, vec![(Some(may(1)), 2, 6_900)], Some(13_800))
    );
    let register = trial
        .authority
        .open_daily_movement_register(OWNER, may(4))
        .unwrap();
    let moved: Vec<_> = register
        .iter()
        .map(|r| (r.product, r.opening, r.received, r.dispatched, r.closing))
        .collect();
    assert_eq!(moved, [(LIME, 4, 0, 2, 2), (SODA, 7, 0, 4, 3)]);

    // Count adjustment: one Soda packet short on 05-05.
    trial
        .authority
        .submit_physical_count(
            OWNER,
            PhysicalCountInput {
                product: SODA,
                counted: "2",
                fridge: None,
                date: may(5),
                reason: Some(StockCountReason::CountCorrection),
                note: Some("one packet short"),
            },
        )
        .unwrap();
    let soda = lots(&trial, SODA);
    assert_eq!((soda.0, soda.2), (2, Some(7_500)));
    let reorder: Vec<_> = trial
        .authority
        .open_reorder_planner(OWNER)
        .unwrap()
        .iter()
        .map(|r| (r.product, r.on_hand))
        .collect();
    assert_eq!(reorder, [(LIME, 2), (SODA, 2)]);

    // Manual godown counts stay separate from the derived movements.
    crate::godown::save(
        &mut trial,
        &crate::godown::Row {
            item: "Synthetic Soda".into(),
            cases: "9".into(),
            note: String::new(),
        },
    )
    .unwrap();
    assert_eq!(trial.authority.product_stock(SODA), Some(2));
    let stock = trial.render("stock").unwrap();
    assert!(
        stock.contains(r#"<div class="label">Stock Value</div><div class="val">₹213</div>"#),
        "{stock}"
    );
    assert!(stock.contains(r#"<td><b>Synthetic Soda</b><div class="muted" style="font-size:11px"></div></td><td class="num" style="font-weight:800">2</td><td class="num">3</td><td><span class="tag low">LOW</span>"#));
    let godown = trial.render("godown").unwrap();
    assert!(godown.contains("1 items · 9 cases"));
    let purchases = trial.render("purchases").unwrap();
    assert!(
        purchases
            .contains(r#"<span class="tag ok">Completed</span></td><td class="num">11 / 16</td>"#)
    );

    // The Manager sees no cost, buy rate or FIFO value on any batch 2 screen.
    trial.owner = false;
    for tab in [
        "products",
        "suppliers",
        "order",
        "purchases",
        "stock",
        "godown",
        "crates",
        "reorder",
        "ratelist",
    ] {
        let html = trial.render(tab).unwrap();
        assert!(html.contains("role=\"alert\""), "{tab}");
        for hidden in ["₹", "69", "37.50", "Synthetic Wholesale"] {
            assert!(!html.contains(hidden), "{tab} shows {hidden}");
        }
    }
}
