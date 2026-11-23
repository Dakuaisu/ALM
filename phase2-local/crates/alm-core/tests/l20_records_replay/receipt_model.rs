use super::model::{Db, Row, Stamp};
use super::receipt_rows::{ActionKind, GoodsLog, ReceiveInput, Rupees};

const LOCAL_LOG_CAP: usize = 4_000;
const LOCAL_LOG_KEEP: usize = 3_000;

impl Db {
    pub(super) fn receive_po(&mut self, order_id: &str, input: ReceiveInput<'_>) -> bool {
        let index = self
            .purchase_orders
            .iter()
            .position(|row| row.id == order_id)
            .expect("existing synthetic PO");
        let order = &self.purchase_orders[index].value.data;
        assert!(!order.cancelled);
        let [line] = order.items.as_slice() else {
            panic!("focused one-line order")
        };
        let remaining = line
            .ordered
            .saturating_sub(line.received)
            .saturating_sub(line.cancelled);
        let mut receipt = input.receipt;
        receipt.qty = receipt.qty.min(remaining);
        if receipt.qty == 0 {
            return false;
        }
        assert!(receipt.rate.minor() > 0, "focused positive-rate receipt");
        let stamped = self
            .products
            .iter()
            .position(|row| row.id == line.pid)
            .or_else(|| self.first_exact(&line.item))
            .expect("existing synthetic product");
        let name = &self.products[stamped].value.data.item;
        let cost_target = self.first_exact(name).expect("cost lookup by exact name");
        let on_hand = self.receipt_on_hand_for_name(name).max(0);
        let supplier = order.supplier.trim().to_owned();

        let priced = &mut self.products[cost_target].value.data;
        let costs = priced.costs.as_mut().expect("synthetic product pricing");
        let prior = if costs.avg_cost.minor() > 0 {
            costs.avg_cost
        } else {
            costs.buy_rate
        };
        let before = i128::from(on_hand);
        let incoming = i128::from(receipt.qty);
        let count = before + incoming;
        let combined =
            before * i128::from(prior.minor()) + incoming * i128::from(receipt.rate.minor());
        costs.avg_cost = Rupees::from_minor(i64::try_from((combined + count / 2) / count).unwrap());
        costs.buy_rate = receipt.rate;
        costs.margin = Rupees::from_minor(
            costs
                .sell_rate
                .minor()
                .checked_sub(receipt.rate.minor())
                .unwrap(),
        );
        assert!(!supplier.is_empty() && priced.supplier_prices.contains_key(&supplier));
        priced.supplier_prices.insert(supplier, Some(receipt.rate));
        self.products[stamped].value.stamp.write(None, input.tick);

        let order = &mut self.purchase_orders[index].value;
        let line = &mut order.data.items[0];
        line.rate = receipt.rate;
        line.received += receipt.qty;
        line.receipts.push(receipt);
        order.stamp.write(None, input.tick);

        let log = GoodsLog {
            ts: input.tick.m,
            date: input.today,
            device: input.tick.d.into(),
            kind: ActionKind::Grn,
            msg: format!(
                "PO-{:03} {} — goods received",
                order.data.no, order.data.supplier
            ),
        };
        let mut stamp = Stamp::default();
        stamp.write(None, input.tick);
        self.log.push(Row::new(input.log_id, log, stamp));
        if self.log.len() > LOCAL_LOG_CAP {
            self.log.drain(..self.log.len() - LOCAL_LOG_KEEP);
        }
        self.meta.data.rev += 1;
        true
    }

    fn first_exact(&self, name: &str) -> Option<usize> {
        self.products
            .iter()
            .position(|row| row.value.data.item == name)
    }

    pub(super) fn receipt_on_hand_for_name(&self, name: &str) -> i64 {
        let product = self
            .first_exact(name)
            .expect("existing synthetic stock name");
        self.receipt_on_hand(&self.products[product].id)
    }

    pub(super) fn receipt_on_hand(&self, id: &str) -> i64 {
        self.stock
            .get(id)
            .expect("seeded opening stock")
            .data
            .opening
            + self
                .purchase_orders
                .iter()
                .flat_map(|row| &row.value.data.items)
                .filter(|line| line.pid == id)
                .map(|line| i64::from(line.received))
                .sum::<i64>()
    }
}
