//! L08-05 static source-derived coverage, not a legacy runtime observation.
//! ALM_Business_Manager.html: delete 6342-6354, stock 1065-1148,
//! inbound register 10866-10881, stored receipt costs and quotes 999-1031.

use super::*;

const OPENING: u32 = 10;
const ITEM: &str = "Synthetic Orange";
const CONTROL_SUPPLIER: &str = "Synthetic Control";
const TARGET_SUPPLIER: &str = "Synthetic Target";
const OPENING_COST: Money = Money::from_minor_units(1_000);
const TARGET_RATE: Money = Money::from_minor_units(1_390);

#[derive(Clone, Debug, Eq, PartialEq)]
struct SourceLedger(PurchaseBook);

struct DeleteDialog {
    id: PurchaseOrderId,
    warning: String,
}

enum DeleteResponse {
    Decline,
    Confirm(Role),
}

impl SourceLedger {
    fn from_fixture(fixture: &ConfirmationFixture) -> Self {
        Self(fixture.purchases.clone())
    }

    fn open(&self, id: PurchaseOrderId) -> DeleteDialog {
        let selected = self.0.orders.iter().find(|order| order.id == id).unwrap();
        let received: u32 = selected.items.iter().map(|line| line.received).sum();
        let warning = if received > 0 {
            format!(
                "⚠ {received} case(s) were received on this PO. Deleting it removes those receipts from the records, \
                 so stock goes back DOWN by {received} case(s) — as if the goods were never received. \
                 Only delete a PO that was entered by mistake."
            )
        } else {
            "Removes this purchase order. Nothing was received on it, so stock is not affected."
                .into()
        };
        DeleteDialog { id, warning }
    }

    fn resolve(
        &mut self,
        dialog: DeleteDialog,
        response: DeleteResponse,
    ) -> Result<(), PurchaseError> {
        match response {
            DeleteResponse::Decline => Ok(()),
            DeleteResponse::Confirm(role) => {
                owner(role)?;
                self.0.orders.retain(|order| order.id != dialog.id);
                Ok(())
            }
        }
    }

    fn product_lines(&self) -> impl Iterator<Item = &SavedPurchaseLine> {
        self.0
            .orders
            .iter()
            .flat_map(|order| &order.items)
            .filter(|line| line.line.product_id == Some(PRODUCT))
    }

    fn on_hand(&self) -> u32 {
        OPENING + self.product_lines().map(|line| line.received).sum::<u32>()
    }

    fn inbound_on(&self, date: BillDate) -> u32 {
        self.product_lines()
            .flat_map(SavedPurchaseLine::receipts)
            .filter(|receipt| receipt.date == date)
            .map(|receipt| receipt.quantity.get())
            .sum()
    }
}

fn jan(day: u8) -> BillDate {
    BillDate::new(2025, 1, day).unwrap()
}

fn opening_fixture() -> ConfirmationFixture {
    let mut fixture = ConfirmationFixture::with_ordered_lots(
        PRODUCT,
        &[(OPENING, OPENING_COST)],
        Money::from_minor_units(1_500),
    )
    .unwrap();
    fixture
        .register_purchase_product(PurchaseProduct {
            id: PRODUCT,
            item: ITEM.into(),
            buy_rate: OPENING_COST,
        })
        .unwrap();
    fixture
}

fn save(fixture: &mut ConfirmationFixture, supplier: &str, date: BillDate) -> PurchaseOrderId {
    fixture
        .save_purchase_order(
            OWNER,
            &PurchaseDraft {
                supplier: Some(PurchaseSupplier {
                    id: None,
                    name: supplier.into(),
                }),
                date: Some(date),
                note: supplier.into(),
                items: vec![PurchaseLine {
                    item: ITEM.into(),
                    product_id: Some(PRODUCT),
                    ordered: NonZeroU32::new(5).unwrap(),
                    rate: OPENING_COST,
                }],
            },
        )
        .unwrap()
}

fn received_fixture() -> (ConfirmationFixture, PurchaseOrderId, PurchaseOrderId) {
    let mut fixture = opening_fixture();
    let target = save(&mut fixture, TARGET_SUPPLIER, jan(3));
    let control = save(&mut fixture, CONTROL_SUPPLIER, jan(1));
    for (id, day, quantity, rate) in [(control, 2, 1, OPENING_COST), (target, 4, 2, TARGET_RATE)] {
        assert_eq!(
            fixture.receive_purchase_order(ReceiveRequest {
                role: OWNER,
                id,
                date: Some(jan(day)),
                today: jan(31),
                lines: &[ReceiveLine { quantity, rate }],
            }),
            Ok(ReceiveResult::Received)
        );
    }
    (fixture, target, control)
}

#[test]
fn l08_05_opening_uses_only_the_selected_orders_cumulative_receipts() {
    let (fixture, target, _) = received_fixture();
    let snapshot = fixture.clone();
    let model = SourceLedger::from_fixture(&fixture);
    let before = model.clone();

    let prompt = model.open(target);

    assert_eq!(
        prompt.warning,
        concat!(
            "⚠ 2 case(s) were received on this PO. Deleting it removes those receipts from the ",
            "records, so stock goes back DOWN by 2 case(s) — as if the goods were never received. ",
            "Only delete a PO that was entered by mistake."
        )
    );
    assert_eq!(model, before);
    assert_eq!(fixture, snapshot);
}

#[test]
fn l08_05_decline_and_manager_confirmation_preserve_complete_state() {
    let (fixture, target, _) = received_fixture();
    let snapshot = fixture.clone();
    let before = SourceLedger::from_fixture(&fixture);

    for (response, expected) in [
        (DeleteResponse::Decline, Ok(())),
        (
            DeleteResponse::Confirm(Role::Manager),
            Err(PurchaseError::AccessDenied),
        ),
    ] {
        let mut model = before.clone();
        let dialog = model.open(target);
        assert_eq!(model.resolve(dialog, response), expected);
        assert_eq!(model, before);
        assert_eq!(fixture, snapshot);
    }
}

#[test]
fn l08_05_owner_confirmation_drops_only_target_receipts_from_derived_views() {
    let (fixture, target, control) = received_fixture();
    let snapshot = fixture.clone();
    let before = SourceLedger::from_fixture(&fixture);
    let mut after = before.clone();
    let [target_order, control_order] = before.0.orders.as_slice() else {
        panic!("expected target and control orders")
    };
    assert_eq!((target_order.id, target_order.date), (target, jan(3)));
    assert_eq!((control_order.id, control_order.date), (control, jan(1)));
    assert_eq!(target_order.items[0].received, 2);
    assert_eq!(
        target_order.items[0].receipts(),
        &[PurchaseReceipt {
            date: jan(4),
            quantity: NonZeroU32::new(2).unwrap(),
            rate: TARGET_RATE,
        }]
    );
    assert_eq!(control_order.items[0].received, 1);
    assert_eq!(
        control_order.items[0].receipts(),
        &[PurchaseReceipt {
            date: jan(2),
            quantity: NonZeroU32::MIN,
            rate: OPENING_COST,
        }]
    );
    assert_eq!(fixture.product_stock(PRODUCT), Some(13));
    let average = Money::from_minor_units(1_060);
    assert_eq!(
        fixture.purchase_cost(OWNER, PRODUCT, TARGET_SUPPLIER),
        Ok(Some((average, TARGET_RATE, Some(TARGET_RATE))))
    );
    let days = [1, 2, 3, 4].map(jan);
    assert_eq!(before.on_hand(), 13);
    assert_eq!(days.map(|day| before.inbound_on(day)), [0, 1, 0, 2]);

    let dialog = after.open(target);
    let result = after.resolve(dialog, DeleteResponse::Confirm(OWNER));

    assert_eq!(result, Ok(()));
    let surviving_orders = after.0.orders.as_slice();
    assert_eq!(surviving_orders, std::slice::from_ref(control_order));
    assert_eq!(after.on_hand(), 11);
    assert_eq!(days.map(|day| after.inbound_on(day)), [0, 1, 0, 0]);
    assert_eq!(after.0.products, before.0.products);
    assert_eq!(after.0.costs, before.0.costs);
    assert_eq!(after.0.suppliers, before.0.suppliers);
    assert_eq!(fixture, snapshot);
}

#[test]
fn l08_05_owner_confirmation_of_unreceived_order_has_no_stock_or_movement_effect() {
    let mut fixture = opening_fixture();
    let pending = save(&mut fixture, TARGET_SUPPLIER, jan(5));
    let snapshot = fixture.clone();
    let mut model = SourceLedger::from_fixture(&fixture);
    assert_eq!((model.on_hand(), model.inbound_on(jan(5))), (10, 0));

    let dialog = model.open(pending);
    assert_eq!(
        dialog.warning,
        "Removes this purchase order. Nothing was received on it, so stock is not affected."
    );
    let result = model.resolve(dialog, DeleteResponse::Confirm(OWNER));

    assert_eq!(result, Ok(()));
    assert!(model.0.orders.is_empty());
    assert_eq!((model.on_hand(), model.inbound_on(jan(5))), (10, 0));
    assert_eq!(fixture, snapshot);
}
