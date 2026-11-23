use alm_store::confirmation::order_planner::{
    CopyPlanFeedback, CopyPlanRejected, OrderPlannerError,
};
use alm_store::confirmation::{ConfirmationFixture, ReorderError};
use alm_store::{BillDate, Money};

use super::{A, B, ITEM_A, ITEM_B, MANAGER, OWNER};

pub(super) fn exercise(fixture: &ConfirmationFixture) {
    let today = BillDate::new(2025, 2, 21).unwrap();
    let mut plan = fixture.open_order_planner(OWNER).unwrap();
    assert_eq!(
        plan.rows(true)
            .map(|line| (line.product, line.item.as_str(), line.on_hand))
            .collect::<Vec<_>>(),
        [(A, ITEM_A, 4), (B, ITEM_B, 2)]
    );
    assert_eq!(
        fixture.open_order_planner(MANAGER),
        Err(OrderPlannerError::AccessDenied)
    );
    assert_eq!(
        fixture.open_reorder_planner(MANAGER),
        Err(ReorderError::AccessDenied)
    );
    assert_eq!(
        fixture.copy_reorder_list(MANAGER, today, &mut |_| {
            panic!("Manager reorder list invoked the copy callback")
        }),
        Err(ReorderError::AccessDenied)
    );

    let mut fallback = Vec::new();
    let mut capture = |text: &str| {
        fallback.push(text.to_owned());
        Ok(())
    };
    assert_eq!(
        fixture.copy_order_plan(OWNER, &plan, today, Some(&mut capture)),
        Ok(CopyPlanFeedback::Copied)
    );
    assert_eq!(
        fallback,
        [concat!(
            "*ALM Order Plan* (21-Feb-2025)\n\n",
            "Synthetic A — 11 pkt (Synthetic Supplier ₹60)\n",
            "Synthetic B — 13 pkt (Synthetic Supplier ₹25)\n",
        )]
    );
    assert!(plan.add_item(A, Some(2)));
    assert!(plan.add_item(B, Some(1)));
    let before = plan.clone();
    let mut forbidden = |_: &str| panic!("Manager order plan invoked the copy callback");
    assert_eq!(
        fixture.copy_order_plan(MANAGER, &plan, today, Some(&mut forbidden)),
        Err(OrderPlannerError::AccessDenied)
    );

    let mut cart = Vec::new();
    let mut reject = |text: &str| {
        cart.push(text.to_owned());
        Err(CopyPlanRejected)
    };
    assert_eq!(
        fixture.copy_order_plan(OWNER, &plan, today, Some(&mut reject)),
        Ok(CopyPlanFeedback::Rejected)
    );
    assert_eq!(
        cart,
        [concat!(
            "*ALM Order Plan* (21-Feb-2025)\n\n",
            "Synthetic A — 2 pkt (Synthetic Supplier ₹60)\n",
            "Synthetic B — 1 pkt (Synthetic Supplier ₹25)\n",
            "\nTotal ₹145",
        )]
    );
    assert_eq!(
        fixture.copy_order_plan(OWNER, &plan, today, None),
        Ok(CopyPlanFeedback::Unavailable)
    );
    assert_eq!(plan, before);

    let mut list = Vec::new();
    fixture
        .copy_reorder_list(OWNER, today, &mut |text| list.push(text))
        .unwrap();
    assert_eq!(list, ["*ALM Reorder List* (21-Feb-2025)\n\n"]);

    // C registers purchase metadata, not product-entry reorder thresholds.
    let mut control = ConfirmationFixture::new(A, 4, Money::from_minor_units(11_800)).unwrap();
    control
        .add_product(B, 2, Money::from_minor_units(5_600))
        .unwrap();
    for (id, item) in [(A, ITEM_A), (B, ITEM_B)] {
        let mut entry = control.open_product_entry(OWNER, id).unwrap();
        entry.item = item.into();
        entry.reorder_level = Some(5);
        control.save_product_entry(OWNER, &entry).unwrap();
    }
    list.clear();
    control
        .copy_reorder_list(OWNER, today, &mut |text| list.push(text))
        .unwrap();
    assert_eq!(
        list,
        [concat!(
            "*ALM Reorder List* (21-Feb-2025)\n\n",
            "Synthetic B — 13 pkt\n",
            "Synthetic A — 11 pkt\n",
        )]
    );
}
