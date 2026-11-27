//! Purchase Orders screen (legacy VIEWS.purchases): the PO list and the Receive Goods form text.
use super::output::PurchaseOrderStatus;
use super::{
    ConfirmationFixture, PurchaseError, PurchaseOrderId, PurchaseOrderSelection, ReceiveFormRow,
    ReceiveLine,
};
use crate::{BillDate, Money, Role};

impl PurchaseOrderId {
    /// Fixture-local position, for carrying a selection across IPC.
    pub const fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PurchaseOrderRow {
    pub id: PurchaseOrderId,
    /// The selection to view, print or receive this order (number is the 1-based PO number).
    pub selection: PurchaseOrderSelection,
    pub number: String,
    pub supplier: String,
    pub date: BillDate,
    pub status: PurchaseOrderStatus,
    pub ordered: u64,
    pub received: u64,
    pub remaining: u64,
    /// Ordered quantity times line rate, summed (the PO value).
    pub value: Money,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PurchaseOrderList {
    /// Newest order first.
    pub rows: Vec<PurchaseOrderRow>,
    /// Pending or partially received.
    pub open: usize,
    pub suppliers: usize,
}

impl ConfirmationFixture {
    /// Owner-only PO list with quantity progress and value.
    pub fn purchase_order_list(&self, role: Role) -> Result<PurchaseOrderList, PurchaseError> {
        crate::permissions::require(role, crate::permissions::Capability::Purchases)
            .map_err(|_| PurchaseError::AccessDenied)?;
        let mut rows = Vec::new();
        for (index, order) in self.purchases.orders.iter().enumerate() {
            let status = order.status();
            let mut row = PurchaseOrderRow {
                id: order.id,
                selection: PurchaseOrderSelection {
                    role,
                    id: order.id,
                    number: u64::try_from(index + 1).map_err(|_| PurchaseError::ReceiptOverflow)?,
                },
                number: super::output::supplier_html::purchase_order_number(
                    u64::try_from(index + 1).map_err(|_| PurchaseError::ReceiptOverflow)?,
                ),
                supplier: order.supplier_name.clone(),
                date: order.date,
                status,
                ordered: 0,
                received: 0,
                remaining: 0,
                value: Money::from_minor_units(0),
            };
            let mut value = 0_i64;
            for line in &order.items {
                row.ordered += u64::from(line.line.ordered.get());
                row.received += u64::from(line.received);
                row.remaining += u64::from(line.remaining());
                value = line
                    .line
                    .rate
                    .minor_units()
                    .checked_mul(i64::from(line.line.ordered.get()))
                    .and_then(|amount| value.checked_add(amount))
                    .ok_or(PurchaseError::ReceiptOverflow)?;
            }
            row.value = Money::from_minor_units(value);
            rows.push(row);
        }
        rows.reverse();
        Ok(PurchaseOrderList {
            open: rows.iter().filter(|row| row.status.can_receive()).count(),
            suppliers: self.purchases.suppliers.len(),
            rows,
        })
    }
}

impl ReceiveFormRow {
    /// Reads one typed receive row: blank quantity is 0 and a blank rate keeps the PO rate.
    pub fn line_from_text(&self, quantity: &str, rate: &str) -> Result<ReceiveLine, PurchaseError> {
        let quantity = quantity.trim();
        let quantity = if quantity.is_empty() {
            0
        } else if quantity.bytes().all(|b| b.is_ascii_digit()) {
            quantity
                .parse()
                .map_err(|_| PurchaseError::InvalidReceipt)?
        } else {
            return Err(PurchaseError::InvalidReceipt);
        };
        let rate = if rate.trim().is_empty() {
            self.rate
        } else {
            Money::parse_rupees(rate).map_err(|_| PurchaseError::InvalidReceipt)?
        };
        Ok(ReceiveLine { quantity, rate })
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use crate::confirmation::demo_fixture::{fixture, seed_purchasing};
    use crate::confirmation::{
        PurchaseDraft, PurchaseError, PurchaseLine, PurchaseOrderStatus, PurchaseSupplier,
        ReceiveRequest,
    };
    use crate::{BillDate, Money, ProductId, Role};

    #[test]
    fn list_tracks_dated_partial_receipts_and_closure() {
        let mut fixture = fixture().unwrap();
        seed_purchasing(&mut fixture).unwrap();
        let day = |d| BillDate::new(2025, 5, d).unwrap();
        let draft = PurchaseDraft {
            supplier: Some(PurchaseSupplier {
                id: None,
                name: "Synthetic Depot".into(),
            }),
            date: Some(day(1)),
            note: String::new(),
            items: vec![PurchaseLine {
                item: "Synthetic Soda".into(),
                product_id: Some(ProductId::new(2)),
                ordered: NonZeroU32::new(10).unwrap(),
                rate: Money::from_minor_units(3_600),
            }],
        };
        let id = fixture.save_purchase_order(Role::Owner, &draft).unwrap();
        let form = fixture
            .open_purchase_receive_form(Role::Owner, id, day(2))
            .unwrap();
        let row = &form.rows[0];
        assert_eq!(
            row.line_from_text("", ""),
            Ok(crate::confirmation::ReceiveLine {
                quantity: 0,
                rate: Money::from_minor_units(3_600)
            })
        );
        assert_eq!(
            row.line_from_text("x", ""),
            Err(PurchaseError::InvalidReceipt)
        );
        let first = [row.line_from_text("4", "36").unwrap()];
        fixture
            .receive_purchase_order(ReceiveRequest {
                role: Role::Owner,
                id,
                date: Some(day(2)),
                today: day(2),
                lines: &first,
            })
            .unwrap();
        let list = fixture.purchase_order_list(Role::Owner).unwrap();
        let po = &list.rows[0];
        assert_eq!(
            (
                po.number.as_str(),
                po.status,
                po.ordered,
                po.received,
                po.remaining
            ),
            ("PO-001", PurchaseOrderStatus::PartiallyReceived, 10, 4, 6)
        );
        assert_eq!((po.value.minor_units(), list.open), (36_000, 1));
        fixture.close_remaining(Role::Owner, id, true).unwrap();
        let list = fixture.purchase_order_list(Role::Owner).unwrap();
        assert_eq!(
            (list.rows[0].status, list.rows[0].remaining, list.open),
            (PurchaseOrderStatus::Completed, 0, 0)
        );
        assert_eq!(fixture.product_stock(ProductId::new(2)), Some(44));
        // DEC-034: a Manager lists the orders it receives against.
        assert_eq!(
            fixture
                .purchase_order_list(Role::Manager)
                .map(|l| l.rows.len()),
            Ok(list.rows.len())
        );
    }
}
