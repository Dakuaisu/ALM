use alm_core::DraftId;
use alm_core::confirmation::{CustomerLedgerKind, OpeningLedgerKind, SalesPeriod, SalesRow};
use serde_json::{Value, json};

use super::OWNER;
use super::fixture::{CATALOG, Fixture, TODAY};
use super::summary::mode_name;

pub(super) const DEVICE: &str = "synthetic-mirror-device";

impl Fixture {
    fn sales(&self) -> Vec<SalesRow> {
        self.app
            .sales_history(OWNER, TODAY, SalesPeriod::All, "")
            .unwrap()
            .rows
    }

    fn sale_record(&self, row: &SalesRow) -> Value {
        let (review, bill) = self.app.current_sale(OWNER, row.bill.draft_id).unwrap();
        json!({
            "id": format!("sale-{}", bill.invoice), "date": row.date.to_string(),
            "invoice": bill.invoice, "totalMinor": bill.total.minor_units(),
            "paidMinor": bill.received.unwrap().minor_units(), "cancelled": row.cancelled,
            "paymentMode": mode_name(review.payment_mode()),
            "_v": 4, "_m": if bill.invoice == 1 { 301 } else { 300 }, "_d": DEVICE,
        })
    }

    pub(super) fn changed_record(&self) -> Value {
        let old = self
            .sales()
            .into_iter()
            .find(|row| row.bill.draft_id == DraftId::new(1))
            .unwrap();
        self.sale_record(&old)
    }

    pub(super) fn snapshot(&self) -> String {
        let sales: Vec<_> = self
            .sales()
            .iter()
            .map(|row| self.sale_record(row))
            .collect();
        let mut payments = Vec::new();
        let mut customers = Vec::new();
        for contact in &self.contacts {
            let ledger = self
                .app
                .customer_ledger_with_opening(OWNER, &contact.id)
                .unwrap();
            let mut opening = 0;
            for row in ledger.entries {
                match row.kind {
                    OpeningLedgerKind::Opening => opening = row.debit.minor_units(),
                    OpeningLedgerKind::Transaction(CustomerLedgerKind::Receipt { mode }) => {
                        payments.push(json!({"customerId": contact.id.value(),
                            "date": row.date.unwrap().to_string(), "amountMinor": row.credit.minor_units(),
                            "mode": mode_name(mode)}));
                    }
                    OpeningLedgerKind::Transaction(CustomerLedgerKind::Bill { .. }) => (),
                }
            }
            customers.push(json!({"id": contact.id.value(), "name": contact.name,
                "mobile": contact.mobile, "openingMinor": opening}));
        }
        let stock = CATALOG.map(|(id, item)| {
            let lots = self.app.open_stock_lots(OWNER, id).unwrap();
            let remaining: Vec<_> = lots
                .rows
                .iter()
                .map(|row| {
                    json!({"packets": row.remaining,
                "unitCostMinor": row.unit_cost.unwrap().minor_units()})
                })
                .collect();
            json!({"item": item, "remainingLots": remaining})
        });
        json!({"sales": sales, "payments": payments, "customers": customers, "stock": stock,
            "meta": {"fixture": "L20-15"}})
        .to_string()
    }
}
