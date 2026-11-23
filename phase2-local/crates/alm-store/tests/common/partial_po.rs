use alm_store::ProductId;
use alm_store::confirmation::{
    ConfirmationFixture, PurchaseBusiness, PurchaseOrderSelection, PurchaseOrderStatus,
    PurchaseProductDisplay,
};

use super::{A, B, ITEM_B, OWNER, money};

#[path = "partial_po/plan.rs"]
mod plan;
#[path = "partial_po/receipts.rs"]
mod receipts;

pub struct PartialPurchaseOrderSetup {
    pub fixture: ConfirmationFixture,
    pub selection: PurchaseOrderSelection,
    pub business: PurchaseBusiness<'static>,
    pub products: [PurchaseProductDisplay<'static>; 3],
}

/// Replays the shared Milestone B planner, note, and backdated receipt setup.
pub fn partial_purchase_order_setup() -> PartialPurchaseOrderSetup {
    let mut fixture = ConfirmationFixture::new(A, 0, money(500)).unwrap();
    fixture.add_product(B, 0, money(500)).unwrap();
    let selection = plan::save_planned_order(&mut fixture);
    let mut note = fixture
        .open_purchase_order_note(OWNER, selection.id)
        .unwrap();
    note.note = "Synthetic note & handling".into();
    fixture.save_purchase_order_note(OWNER, &note).unwrap();
    receipts::post_partials(&mut fixture, selection);
    PartialPurchaseOrderSetup {
        fixture,
        selection,
        business: PurchaseBusiness {
            name: "Synthetic & Co.",
            address: "Fixture <Lane>",
            mobile: "00000 00000",
        },
        products: [
            PurchaseProductDisplay {
                id: ProductId::new(99),
                item: "Synthetic A",
                pack_size: "11",
                uom: "jars",
                mrp: Some(money(7_175)),
            },
            PurchaseProductDisplay {
                id: A,
                item: "Renamed Synthetic A",
                pack_size: "12",
                uom: "bottles",
                mrp: Some(money(6_750)),
            },
            PurchaseProductDisplay {
                id: ProductId::new(98),
                item: ITEM_B,
                pack_size: "6",
                uom: "cans",
                mrp: Some(money(8_950)),
            },
        ],
    }
}

#[test]
fn partial_purchase_order_setup_stops_before_closing_remaining() {
    let setup = partial_purchase_order_setup();
    let view = setup
        .fixture
        .view_purchase_order(setup.selection, &setup.products)
        .unwrap();
    assert_eq!(view.status, PurchaseOrderStatus::PartiallyReceived);
    assert_eq!(
        view.lines
            .iter()
            .map(|line| (line.ordered, line.received, line.cancelled, line.remaining))
            .collect::<Vec<_>>(),
        [(12, 8, 0, 4), (9, 5, 0, 4)]
    );
    assert_eq!(
        setup.fixture.purchase_orders(OWNER).unwrap()[0].note,
        "Synthetic note & handling"
    );
    assert_eq!(setup.products[1].mrp, Some(money(6_750)));
    assert_eq!(setup.products[2].mrp, Some(money(8_950)));
    assert_eq!(setup.business.name, "Synthetic & Co.");
}
