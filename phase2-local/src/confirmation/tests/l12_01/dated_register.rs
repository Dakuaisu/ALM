use super::{BillDate, ConfirmationFixture, OWNER, PRODUCT, SALE, date};
use std::cmp::Ordering;

#[derive(Debug, Eq, PartialEq)]
pub(super) struct RegisterRow {
    pub(super) date: BillDate,
    pub(super) opening: i64,
    pub(super) received: i64,
    pub(super) dispatched: i64,
    pub(super) closing: i64,
}

fn split_at(day: BillDate, movements: impl Iterator<Item = (BillDate, i64)>) -> (i64, i64) {
    movements.fold((0, 0), |(before, on), (date, quantity)| {
        match date.parts().cmp(&day.parts()) {
            Ordering::Less => (before + quantity, on),
            Ordering::Equal => (before, on + quantity),
            Ordering::Greater => (before, on),
        }
    })
}

// A one-product L12-01 comparison, not a register API or an L12-02 parity test.
pub(super) fn register(fixture: &ConfirmationFixture) -> [RegisterRow; 3] {
    let orders = fixture.purchase_orders(OWNER).unwrap();
    let count = fixture.open_physical_count(OWNER, PRODUCT).unwrap();
    let (sale, _, _) = &fixture.current_sales[&SALE];
    let sale_day = sale.draft.bill_date().unwrap();
    [10, 11, 12].map(|day| {
        let date = date(day);
        let receipts = orders
            .iter()
            .flat_map(|order| &order.items)
            .filter(|line| line.line.product_id == Some(PRODUCT))
            .flat_map(|line| line.receipts())
            .map(|receipt| (receipt.date, i64::from(receipt.quantity.get())));
        let dispatches = sale
            .draft
            .lines()
            .iter()
            .filter(|line| line.product_id() == PRODUCT)
            .map(|line| (sale_day, i64::from(line.quantity().get())));
        let adjustments = count.movements.iter().map(|row| (row.date, row.delta));
        let (prior_receipts, received) = split_at(date, receipts);
        let (prior_dispatches, dispatched) = split_at(date, dispatches);
        let (prior_adjustments, adjustment) = split_at(date, adjustments);
        // The fixture starts with zero billing stock. Same-day net corrections are signed.
        let opening = prior_receipts - prior_dispatches + prior_adjustments;
        let received = received + adjustment.max(0);
        let dispatched = dispatched - adjustment.min(0);
        RegisterRow {
            date,
            opening,
            received,
            dispatched,
            closing: opening + received - dispatched,
        }
    })
}
