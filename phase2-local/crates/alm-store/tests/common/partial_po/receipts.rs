use alm_store::confirmation::{
    ConfirmationFixture, PurchaseError, PurchaseOrderSelection, PurchaseOrderStatus,
    PurchaseOrderView, PurchaseReceipt, ReceiveLine, ReceiveRequest, ReceiveResult,
};

use super::super::{ITEM_A, ITEM_B, MANAGER, OWNER, feb, fifo, fifo_value, money, qty, stock};

pub(crate) fn post_partials(
    fixture: &mut ConfirmationFixture,
    selection: PurchaseOrderSelection,
) -> PurchaseOrderView {
    // When Feb 12 is entered first, FIFO must still order the layers by receipt date.
    let [_, final_view] = [
        (12, [(5, 130), (4, 210)], [7, 5], [5, 4], 1_490),
        (11, [(3, 110), (1, 190)], [4, 4], [8, 5], 2_010),
    ]
    .map(|(day, inputs, outstanding, on_hand, value)| {
        let lines = inputs.map(|(quantity, rate)| ReceiveLine {
            quantity,
            rate: money(rate),
        });
        let request = ReceiveRequest {
            role: OWNER,
            id: selection.id,
            date: Some(feb(day)),
            today: feb(12),
            lines: &lines,
        };
        assert_eq!(
            fixture.receive_purchase_order(ReceiveRequest {
                role: MANAGER,
                ..request
            }),
            Err(PurchaseError::AccessDenied)
        );
        assert_eq!(
            fixture.receive_purchase_order(request),
            Ok(ReceiveResult::Received)
        );
        let view = fixture.view_purchase_order(selection, &[]).unwrap();
        assert_eq!(view.status, PurchaseOrderStatus::PartiallyReceived);
        assert!(
            view.lines
                .iter()
                .map(|line| {
                    (
                        line.item.as_str(),
                        line.ordered,
                        line.received,
                        line.cancelled,
                        line.remaining,
                    )
                })
                .eq([
                    (ITEM_A, 12, on_hand[0], 0, outstanding[0]),
                    (ITEM_B, 9, on_hand[1], 0, outstanding[1]),
                ]),
            "Unexpected PO progress: {view:?}"
        );
        let order = fixture
            .purchase_orders(OWNER)
            .unwrap()
            .iter()
            .find(|order| order.id == selection.id)
            .unwrap();
        for (saved, (quantity, rate)) in order.items.iter().zip(inputs) {
            let receipt = PurchaseReceipt {
                date: feb(day),
                quantity: qty(quantity),
                rate: money(rate),
            };
            assert_eq!(saved.receipts().last(), Some(&receipt));
        }
        assert_eq!(stock(fixture), on_hand);
        let fifo_total = fifo_value(&fifo(fixture));
        assert_eq!(fifo_total, value);
        println!(
            "Feb {day} receipt {inputs:?}: {:?}, stock={on_hand:?}, outstanding={outstanding:?}, FIFO={fifo_total}",
            view.status
        );
        view
    });
    final_view
}
