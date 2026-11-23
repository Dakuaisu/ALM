use alm_core::DraftCustomer;
use alm_core::confirmation::manual_cashbook::{
    CashBookAmounts, ManualCashBook, ManualCashBookDraft, ManualCashBookError, ManualCashBookRow,
};
use alm_core::confirmation::{
    CashBookEntry, CashBookKind, CustomerLedgerKind, SalesDetail, SalesPeriod, SalesReceiptId,
    SalesSource,
};

use super::{
    AccountCheckpoint, BUYER, ConfirmationFixture, DENIED, DraftCustomerId, MANAGER, OWNER,
    ReceiptMode, SALE, feb, march_receipt_date, money,
};

const BILL_PAYMENT: SalesDetail = SalesDetail {
    source: SalesSource::Bill(SALE),
    amount: money(10_000),
    mode: ReceiptMode::Cash,
};
// Sales History exposes a fixture-local slot, not a persisted receipt ID.
const ONLINE_PAYMENT: SalesDetail = SalesDetail {
    source: SalesSource::Receipt(SalesReceiptId(1)),
    amount: money(5_000),
    mode: ReceiptMode::Online,
};
const LEDGER_BILL: CustomerLedgerKind = CustomerLedgerKind::Bill {
    draft_id: SALE,
    invoice: 1,
};
const LEDGER_RECEIPT: CustomerLedgerKind = CustomerLedgerKind::Receipt {
    mode: ReceiptMode::Online,
};
const BOOK_BILL: CashBookKind = CashBookKind::Bill {
    draft_id: SALE,
    invoice: 1,
};

pub(super) fn assert_books(
    fixture: &ConfirmationFixture,
    buyer: &DraftCustomerId,
    stage: AccountCheckpoint,
) {
    use AccountCheckpoint::{BeforePosting, Cancelled, Edited, OnlineReceipt, Posted};

    let (packets, total, due, balance, collected) = match stage {
        Posted => (2, 23_600, 13_600, 13_600, 10_000),
        OnlineReceipt => (2, 23_600, 13_600, 8_600, 15_000),
        Edited => (3, 41_000, 31_000, 26_000, 15_000),
        Cancelled => (3, 41_000, 31_000, -5_000, 5_000),
        BeforePosting => panic!("books checkpoints start at posting"),
    };
    let cancelled = matches!(stage, Cancelled);
    let active = !cancelled;
    let has_receipt = matches!(stage, OnlineReceipt | Edited | Cancelled);
    let [bill_date, receipt_date] = [feb(21), march_receipt_date()];

    let (review, _) = fixture.current_sale(OWNER, SALE).unwrap();
    let DraftCustomer::Selected { id, name } = review.draft().customer() else {
        panic!("the posted bill must use the existing named customer");
    };
    assert_eq!(
        (id, name.as_str(), review.payment_mode()),
        (buyer, BUYER, ReceiptMode::Cash)
    );

    assert_eq!(fixture.customer_ledger(MANAGER, buyer), Err(DENIED));
    let ledger = fixture.customer_ledger(OWNER, buyer).unwrap();
    let expected_ledger = [
        active.then_some((bill_date, LEDGER_BILL, BILL_PAYMENT.amount)),
        has_receipt.then_some((receipt_date, LEDGER_RECEIPT, ONLINE_PAYMENT.amount)),
    ];
    assert_eq!(
        ledger
            .entries
            .into_iter()
            .map(|row| (row.date, row.kind, row.credit))
            .collect::<Vec<_>>(),
        expected_ledger.into_iter().flatten().collect::<Vec<_>>(),
        "{stage:?}: dated ledger sources",
    );
    assert_eq!(ledger.closing, money(balance));

    let period = SalesPeriod::Custom((Some(bill_date), Some(receipt_date)));
    assert_eq!(
        fixture.sales_history(MANAGER, receipt_date, period, BUYER),
        Err(DENIED)
    );
    let history = fixture
        .sales_history(OWNER, receipt_date, period, BUYER)
        .unwrap();
    let [row] = history.rows.as_slice() else {
        panic!("{stage:?}: expected one bill row; got {:?}", history.rows);
    };
    let bill = &row.bill;
    assert_eq!(
        (bill.draft_id, bill.invoice, row.date, row.cancelled),
        (SALE, 1, bill_date, cancelled)
    );
    assert_eq!(
        (bill.quantity, bill.total, bill.received, row.remainder),
        (packets, money(total), Some(BILL_PAYMENT.amount), money(due))
    );
    assert_eq!(row.balance, money(if cancelled { 0 } else { balance }));
    let cash = if active { 10_000 } else { 0 };
    let online = if has_receipt { 5_000 } else { 0 };
    assert_eq!(history.active_packets, if active { packets } else { 0 });
    assert_eq!(history.sales, money(if active { total } else { 0 }));
    let paid = [
        history.bill_paid,
        history.receipt_paid,
        history.total_collected,
    ];
    assert_eq!(paid, [cash, online, collected].map(money));
    let drilldown = history.collected;
    assert_eq!(drilldown.cash, money(cash));
    assert_eq!(drilldown.online, money(online));
    let expected_details: &[SalesDetail] = match stage {
        Posted => &[BILL_PAYMENT],
        OnlineReceipt | Edited => &[ONLINE_PAYMENT, BILL_PAYMENT],
        Cancelled => &[ONLINE_PAYMENT],
        BeforePosting => &[],
    };
    assert_eq!(drilldown.details, expected_details, "{stage:?}: drilldown");
    assert_eq!(drilldown.bill_ids, active.then_some(SALE).as_slice());
    assert_eq!(
        drilldown.receipt_ids,
        has_receipt.then_some(SalesReceiptId(1)).as_slice()
    );

    // Drilldown lacks a date field: exact-day queries bind each source to its document date.
    let bill_day = (bill_date, active.then_some(BILL_PAYMENT), BOOK_BILL);
    let later_payment = has_receipt.then_some(ONLINE_PAYMENT);
    let receipt_day = (receipt_date, later_payment, CashBookKind::Receipt);
    let cash_rupees = if active { 100 } else { 0 };
    let online_rupees = if has_receipt { 50 } else { 0 };
    for ((date, payment, kind), paise, rupees) in [
        (bill_day, [cash, 0], [cash_rupees, 0]),
        (receipt_day, [0, online], [0, online_rupees]),
    ] {
        let dated = fixture
            .sales_history(OWNER, date, SalesPeriod::Today, BUYER)
            .unwrap();
        let details = dated.collected.details;
        assert_eq!(details, payment.as_slice(), "{stage:?}: {date}");

        assert_eq!(fixture.cashbook_day_close(MANAGER, date), Err(DENIED));
        let live = fixture.cashbook_day_close(OWNER, date).unwrap();
        let expected = payment.map(|detail| CashBookEntry {
            kind,
            mode: detail.mode,
            amount: detail.amount,
        });
        assert_eq!(live.entries, expected.as_slice(), "Cash Book: {date}");
        assert_eq!(live.count, expected.as_slice().len());
        assert_eq!([live.cash, live.online], paise.map(money));
        assert_eq!([live.cash_rupees, live.online_rupees], rupees);
    }
}

const MANUAL_DENIED: ManualCashBookError = ManualCashBookError::AccessDenied;
const CLOSE_TEMPLATE: CashBookAmounts = CashBookAmounts {
    cash: money(0),
    paytm: money(0),
    phonepe: money(0),
    goods: money(375),
    misc: money(0),
    salary: money(0),
    debt: money(0),
};

pub(super) struct SavedCashBookClose {
    book: ManualCashBook,
    rows: [ManualCashBookRow; 2],
}

impl SavedCashBookClose {
    pub(super) fn save(fixture: &ConfirmationFixture) -> Self {
        let mut book = ManualCashBook::default();
        let inputs = [(feb(21), [10_000, 0]), (march_receipt_date(), [0, 5_000])];
        let rows = inputs.map(|(date, [cash, paytm])| {
            let mut draft = ManualCashBookDraft::new(Some(date), CLOSE_TEMPLATE);
            let manager_copy = draft.use_system_figures(fixture, MANAGER);
            assert_eq!(manager_copy, Err(MANUAL_DENIED));
            assert_eq!(draft.amounts, CLOSE_TEMPLATE);
            draft.use_system_figures(fixture, OWNER).unwrap();
            let expected = CashBookAmounts {
                cash: money(cash),
                paytm: money(paytm),
                ..CLOSE_TEMPLATE
            };
            assert_eq!(draft.amounts, expected);
            assert_eq!(book.save(MANAGER, &draft), Err(MANUAL_DENIED));
            assert_eq!(book.month(OWNER, date).unwrap().rows, 0);
            let row = book.save(OWNER, &draft).unwrap();
            assert_eq!((row.date, row.amounts), (date, expected));
            let compare = row.draft().compare(fixture, OWNER).unwrap();
            assert_eq!(compare.cash_difference_rupees, 0);
            assert_eq!(compare.online_difference_rupees, 0);
            row
        });
        assert_ne!(rows[0].id, rows[1].id);
        Self { book, rows }
    }

    pub(super) fn assert_after_cancel(&self, fixture: &ConfirmationFixture) {
        for (row, [live_online, cash_delta]) in self.rows.into_iter().zip([[0, 100], [5_000, 0]]) {
            assert_eq!(self.book.row(MANAGER, row.id), Err(MANUAL_DENIED));
            let saved = self.book.row(OWNER, row.id).unwrap().unwrap();
            assert_eq!(saved, row, "cancellation must not rewrite a saved close");
            let draft = saved.draft();
            assert_eq!(draft.compare(fixture, MANAGER), Err(MANUAL_DENIED));
            let compare = draft.compare(fixture, OWNER).unwrap();
            assert_eq!(compare.system.cash, money(0));
            assert_eq!(compare.system.online, money(live_online));
            assert_eq!(compare.cash_difference_rupees, cash_delta);
            assert_eq!(compare.online_difference_rupees, 0);
        }
    }

    pub(super) fn assert_after_undo(&self, fixture: &ConfirmationFixture) {
        let expected = [
            (feb(21), [10_000, 0], [100, 0]),
            (march_receipt_date(), [0, 5_000], [0, 50]),
        ];
        for (baseline, (date, paise, rupees)) in self.rows.into_iter().zip(expected) {
            assert_eq!(self.book.row(MANAGER, baseline.id), Err(MANUAL_DENIED));
            let saved = self.book.row(OWNER, baseline.id).unwrap().unwrap();
            assert_eq!(saved.date, date);
            assert_eq!(saved, baseline, "undo must not rewrite the {date} close");

            let draft = saved.draft();
            assert_eq!(draft.compare(fixture, MANAGER), Err(MANUAL_DENIED));
            let compare = draft.compare(fixture, OWNER).unwrap();
            let live = compare.system;
            assert_eq!([live.cash, live.online], paise.map(money), "{date}");
            assert_eq!([live.cash_rupees, live.online_rupees], rupees, "{date}");
            assert_eq!(compare.cash_difference_rupees, 0, "{date}");
            assert_eq!(compare.online_difference_rupees, 0, "{date}");
        }
    }
}
