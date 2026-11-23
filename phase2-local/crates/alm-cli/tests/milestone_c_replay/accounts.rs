use alm_core::confirmation::{
    CashBookDay, CashBookEntry, CashBookKind, ConfirmationError, ConfirmationFixture,
    CustomerLedger, CustomerLedgerEntry, CustomerLedgerKind, ReceiptMode,
};
use alm_core::{BillDate, DraftCustomerId, Money};

use super::{DENIED, MANAGER, OWNER, SALE, feb, money};

pub(super) fn march_receipt_date() -> BillDate {
    BillDate::new(2025, 3, 1).expect("valid March receipt date")
}

#[derive(Debug)]
pub(super) enum AccountCheckpoint {
    BeforePosting,
    Posted,
    OnlineReceipt,
    Edited,
    Cancelled,
}

#[derive(Debug, Eq, PartialEq)]
pub(super) struct AccountReadback {
    ledger: Result<CustomerLedger, ConfirmationError>,
    cashbook: [CashBookDay; 2],
    balance: Money,
}

impl AccountReadback {
    pub(super) fn open(fixture: &ConfirmationFixture, buyer: &DraftCustomerId) -> Self {
        Self {
            ledger: fixture.customer_ledger(OWNER, buyer),
            cashbook: [feb(21), march_receipt_date()]
                .map(|date| fixture.cashbook_day_close(OWNER, date).unwrap()),
            balance: fixture.customer_due(buyer.value()),
        }
    }
}

pub(super) fn assert_accounts(
    fixture: &ConfirmationFixture,
    buyer: &DraftCustomerId,
    checkpoint: AccountCheckpoint,
) -> AccountReadback {
    assert_eq!(fixture.customer_ledger(MANAGER, buyer), Err(DENIED));
    for date in [feb(21), march_receipt_date()] {
        assert_eq!(fixture.cashbook_day_close(MANAGER, date), Err(DENIED));
    }

    let (ledger, cashbook, balance) = match checkpoint {
        AccountCheckpoint::BeforePosting => (
            Err(ConfirmationError::InvalidDraft),
            [EMPTY_DAY, EMPTY_DAY],
            0,
        ),
        AccountCheckpoint::Posted => (
            Ok(CustomerLedger {
                entries: vec![bill_entry(23_600, 13_600)],
                closing: money(13_600),
            }),
            [bill_day(), EMPTY_DAY],
            13_600,
        ),
        AccountCheckpoint::OnlineReceipt => (
            Ok(CustomerLedger {
                entries: vec![bill_entry(23_600, 13_600), online_entry(8_600)],
                closing: money(8_600),
            }),
            [bill_day(), receipt_day()],
            8_600,
        ),
        AccountCheckpoint::Edited => (
            Ok(CustomerLedger {
                entries: vec![bill_entry(41_000, 31_000), online_entry(26_000)],
                closing: money(26_000),
            }),
            [bill_day(), receipt_day()],
            26_000,
        ),
        AccountCheckpoint::Cancelled => (
            Ok(CustomerLedger {
                entries: vec![online_entry(-5_000)],
                closing: money(-5_000),
            }),
            [EMPTY_DAY, receipt_day()],
            -5_000,
        ),
    };
    let actual = AccountReadback::open(fixture, buyer);
    assert_eq!(actual.ledger, ledger, "{checkpoint:?}: customer ledger");
    assert_eq!(actual.balance, money(balance), "{checkpoint:?}: balance");
    println!(
        "accounts {checkpoint:?}: balance={}, ledger={:?}",
        actual.balance.minor_units(),
        actual.ledger
    );
    for ((date, day), expected) in [feb(21), march_receipt_date()]
        .into_iter()
        .zip(&actual.cashbook)
        .zip(&cashbook)
    {
        assert_eq!(day, expected, "{checkpoint:?}: Cash Book {date}");
        println!("Cash Book {checkpoint:?} {date}: {day:?}");
    }
    actual
}

fn bill_entry(debit: i64, balance: i64) -> CustomerLedgerEntry {
    CustomerLedgerEntry {
        date: feb(21),
        kind: CustomerLedgerKind::Bill {
            draft_id: SALE,
            invoice: 1,
        },
        debit: money(debit),
        credit: money(10_000),
        balance: money(balance),
    }
}

fn online_entry(balance: i64) -> CustomerLedgerEntry {
    CustomerLedgerEntry {
        date: march_receipt_date(),
        kind: CustomerLedgerKind::Receipt {
            mode: ReceiptMode::Online,
        },
        debit: money(0),
        credit: money(5_000),
        balance: money(balance),
    }
}

const EMPTY_DAY: CashBookDay = CashBookDay {
    entries: Vec::new(),
    cash: money(0),
    online: money(0),
    cash_rupees: 0,
    online_rupees: 0,
    count: 0,
};

fn bill_day() -> CashBookDay {
    CashBookDay {
        entries: vec![CashBookEntry {
            kind: CashBookKind::Bill {
                draft_id: SALE,
                invoice: 1,
            },
            mode: ReceiptMode::Cash,
            amount: money(10_000),
        }],
        cash: money(10_000),
        online: money(0),
        cash_rupees: 100,
        online_rupees: 0,
        count: 1,
    }
}

fn receipt_day() -> CashBookDay {
    CashBookDay {
        entries: vec![CashBookEntry {
            kind: CashBookKind::Receipt,
            mode: ReceiptMode::Online,
            amount: money(5_000),
        }],
        cash: money(0),
        online: money(5_000),
        cash_rupees: 0,
        online_rupees: 50,
        count: 1,
    }
}
