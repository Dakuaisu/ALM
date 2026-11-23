use alm_store::confirmation::manual_cashbook::ManualCashBook;
use alm_store::confirmation::{
    BalanceMode, BillPreview, MonthlyProfit, Review, SalesHistory, SalesPeriod, SalesProfit,
    StockLots,
};

use super::{
    A, AccountReadback, B, BUYER, ConfirmationFixture, DraftCustomerId, Money, OWNER, Receipt,
    SALE, feb, march_receipt_date, money,
};

#[derive(Debug)]
pub(super) enum Stage {
    Edited,
    Cancelled,
    Undone,
}

#[derive(Debug, Eq, PartialEq)]
pub(super) struct UndoReadback {
    saved_sale: (Review, Receipt),
    posted_count: usize,
    accounts: AccountReadback,
    lots: [StockLots; 2],
    previews: [BillPreview; 2],
    csv: String,
    ledger: [Money; 2],
    history: SalesHistory,
    gross_profit: Option<Money>,
    monthly: Vec<MonthlyProfit>,
}

impl UndoReadback {
    pub(super) fn read(fixture: &ConfirmationFixture, buyer: &DraftCustomerId) -> Self {
        let (review, receipt) = fixture.current_sale(OWNER, SALE).unwrap();
        let today = march_receipt_date();
        let period = SalesPeriod::Custom((Some(feb(21)), Some(today)));
        Self {
            saved_sale: (review.clone(), receipt.clone()),
            posted_count: fixture.posted_count(),
            accounts: AccountReadback::open(fixture, buyer),
            lots: [A, B].map(|product| fixture.open_stock_lots(OWNER, product).unwrap()),
            previews: [BalanceMode::AsOnBillDate, BalanceMode::Latest]
                .map(|mode| fixture.bill_preview(OWNER, SALE, mode).unwrap()),
            csv: fixture.sales_csv(OWNER, SALE).unwrap(),
            ledger: [fixture.ledger(), fixture.received_ledger()],
            history: fixture.sales_history(OWNER, today, period, BUYER).unwrap(),
            gross_profit: fixture.gross_profit(OWNER, SALE).unwrap(),
            monthly: fixture
                .monthly_profit_report(OWNER, &ManualCashBook::default(), today)
                .unwrap(),
        }
    }

    pub(super) fn assert_reporting(&self, stage: Stage) {
        let active = match stage {
            Stage::Edited | Stage::Undone => true,
            Stage::Cancelled => false,
        };
        let (revenue, cost, profit, cash, latest) = if active {
            (41_000, 15_500, 25_500, 10_000, 26_000)
        } else {
            (0, 0, 0, 0, -5_000)
        };
        let bills = active.then_some(SALE);
        let [as_of_bill, latest_preview] = &self.previews;
        assert_eq!(as_of_bill.balance, money(31_000));
        assert_eq!(latest_preview.balance, money(latest));
        assert_eq!(
            [as_of_bill.cancelled, latest_preview.cancelled],
            [!active; 2]
        );
        assert_eq!(self.ledger, [revenue, cash].map(money));
        assert_eq!(self.gross_profit, Some(money(profit)));
        assert_eq!(
            self.history.profit,
            Ok(SalesProfit {
                bill_ids: bills.as_slice().to_vec(),
                revenue: money(revenue),
                cost: money(cost),
                profit: money(profit),
            })
        );

        let february = self
            .monthly
            .iter()
            .find(|month| (month.year, month.month) == (2025, 2))
            .expect("February in report window");
        for month in &self.monthly {
            let is_february = (month.year, month.month) == (2025, 2);
            let (ids, amounts) = if is_february {
                (bills.as_slice(), [revenue, cost, profit, profit])
            } else {
                (&[][..], [0; 4])
            };
            assert_eq!(month.bills, ids, "{stage:?}: monthly bill membership");
            assert_eq!(
                [month.revenue, month.cost, month.gross, month.net],
                amounts.map(money)
            );
        }

        let remaining = self.lots.each_ref().map(|lots| lots.on_hand);
        println!(
            "{stage:?} #{} on {} (paise): sales={}, bill cash={}, later Online={}, bill due={}, latest={}, FIFO remaining={remaining:?}, Feb cost={}, Feb profit={}",
            as_of_bill.invoice,
            as_of_bill.date,
            self.history.sales.minor_units(),
            self.history.bill_paid.minor_units(),
            self.history.receipt_paid.minor_units(),
            as_of_bill.balance.minor_units(),
            latest_preview.balance.minor_units(),
            february.cost.minor_units(),
            february.net.minor_units(),
        );
    }
}
