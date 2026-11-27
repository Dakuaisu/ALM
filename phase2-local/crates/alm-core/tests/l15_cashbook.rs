use std::num::NonZeroU32;

use alm_core::confirmation::manual_cashbook::{
    CashBookAmounts, CashBookDeletion, ManualCashBook, ManualCashBookDraft, ManualCashBookError,
};
use alm_core::confirmation::{
    CashBookDay, CashBookEntry, CashBookKind, ConfirmationFixture, CustomerLedger,
    CustomerLedgerEntry, CustomerLedgerKind, CustomerReceiptInput, Receipt, ReceiptMode, RequestId,
    Review,
};
use alm_core::{
    BillDate, DraftCustomer, DraftCustomerId, DraftId, DraftLine, Money, ProductId,
    ProvisionalDraft, Role,
};

const OWNER: Role = Role::Owner;
const SALE: DraftId = DraftId::new(1);
const PRODUCT: ProductId = ProductId::new(1);

const fn money(units: i64) -> Money {
    Money::from_minor_units(units)
}

fn seed_accounts(
    [feb, mar]: [BillDate; 2],
    buyer: &DraftCustomerId,
) -> (ConfirmationFixture, Review, Receipt) {
    let mut fixture = ConfirmationFixture::new(PRODUCT, 10, money(11_800)).unwrap();
    let bill = ProvisionalDraft::new(
        SALE,
        vec![DraftLine::new(PRODUCT, NonZeroU32::new(2).unwrap())],
    )
    .with_bill_date(Some(feb))
    .with_customer(DraftCustomer::Selected {
        id: buyer.clone(),
        name: "Synthetic L15 Buyer".into(),
    });
    fixture.register_draft(bill).unwrap();
    let review = fixture.review_named(OWNER, SALE, money(10_051)).unwrap();
    let posted = fixture.confirm(OWNER, RequestId::new(1), &review).unwrap();
    for (mode, amount) in [(ReceiptMode::Online, 5_050), (ReceiptMode::Cash, 2_149)] {
        fixture
            .record_customer_receipt(
                OWNER,
                CustomerReceiptInput {
                    customer_id: buyer.clone(),
                    date: mar,
                    mode,
                    amount: money(amount),
                },
            )
            .unwrap();
    }
    (fixture, review, posted)
}

fn system_days(fixture: &ConfirmationFixture, [feb, mar]: [BillDate; 2]) -> [CashBookDay; 2] {
    let bill = CashBookEntry {
        kind: CashBookKind::Bill {
            draft_id: SALE,
            invoice: 1,
        },
        mode: ReceiptMode::Cash,
        amount: money(10_051),
    };
    let receipt = |mode, amount| CashBookEntry {
        kind: CashBookKind::Receipt,
        mode,
        amount: money(amount),
    };
    let actual = [feb, mar].map(|date| {
        let day = fixture.cashbook_day_close(OWNER, date).unwrap();
        println!("{date} system: {day:?} (Money is minor units)");
        day
    });
    assert_eq!(actual[0].entries, [bill]);
    assert_eq!(
        actual[1].entries,
        [
            receipt(ReceiptMode::Online, 5_050),
            receipt(ReceiptMode::Cash, 2_149)
        ]
    );
    assert_eq!(
        actual.each_ref().map(|day| (
            day.count,
            day.cash.minor_units(),
            day.online.minor_units(),
            day.cash_rupees,
            day.online_rupees,
        )),
        [(1, 10_051, 0, 101, 0), (2, 2_149, 5_050, 21, 51)]
    );
    actual
}

fn expected_ledger([feb, mar]: [BillDate; 2]) -> CustomerLedger {
    let receipt = |mode, credit, balance| CustomerLedgerEntry {
        date: mar,
        kind: CustomerLedgerKind::Receipt { mode },
        debit: money(0),
        credit: money(credit),
        balance: money(balance),
    };
    CustomerLedger {
        entries: vec![
            CustomerLedgerEntry {
                date: feb,
                kind: CustomerLedgerKind::Bill {
                    draft_id: SALE,
                    invoice: 1,
                },
                debit: money(23_600),
                credit: money(10_051),
                balance: money(13_549),
            },
            receipt(ReceiptMode::Online, 5_050, 8_499),
            receipt(ReceiptMode::Cash, 2_149, 6_350),
        ],
        closing: money(6_350),
    }
}

fn amounts(values: [i64; 7]) -> CashBookAmounts {
    let [cash, paytm, phonepe, goods, misc, salary, debt] = values.map(money);
    CashBookAmounts {
        cash,
        paytm,
        phonepe,
        goods,
        misc,
        salary,
        debt,
    }
}

fn assert_month(book: &ManualCashBook, date: BillDate, (rows, values): (usize, [i64; 7])) {
    let m = book.month(OWNER, date).unwrap();
    assert_eq!(m.rows, rows);
    assert_eq!(
        [
            m.goods, m.misc, m.salary, m.debt, m.income, m.expenses, m.net
        ],
        values.map(money)
    );
    println!("{date} month: {m:?} (Money is minor units)");
}

#[test]
fn l15_01_l15_02_manual_close_across_month_end() {
    // Given: a named part-paid bill and two later standalone receipts.
    let feb = "2025-02-28".parse::<BillDate>().unwrap();
    let mar = "2025-03-01".parse::<BillDate>().unwrap();
    let dates = [feb, mar];
    let buyer = DraftCustomerId::new("l15-buyer".into());
    let (fixture, review, posted) = seed_accounts(dates, &buyer);
    let system = system_days(&fixture, dates);
    assert_eq!(
        (posted.total, posted.received, posted.due),
        (money(23_600), Some(money(10_051)), Some(money(13_549)))
    );
    let ledger = expected_ledger(dates);
    let assert_accounts_unchanged = || {
        assert_eq!(fixture.current_sale(OWNER, SALE), Ok((&review, &posted)));
        assert_eq!(fixture.customer_ledger(OWNER, &buyer), Ok(ledger.clone()));
        let day_readback = dates.map(|date| fixture.cashbook_day_close(OWNER, date).unwrap());
        assert_eq!(day_readback, system);
        assert_eq!(fixture.customer_due(buyer.value()), money(6_350));
        assert_eq!(fixture.posted_count(), 1);
        assert_eq!(fixture.ledger(), money(23_600));
        assert_eq!(fixture.received_ledger(), money(10_051));
    };
    assert_accounts_unchanged();

    // When: Owner compares dated drafts and applies the system figures to the March draft.
    let mut book = ManualCashBook::default();
    let [feb_draft, mar_draft, survivor_draft] = [
        (feb, [9_000, 800, 700, 1_201, 2_302, 3_403, 4_504]),
        (mar, [8_800, 900, 1_600, 4_001, 5_002, 6_003, 7_004]),
        (mar, [3_100, 4_200, 5_300, 101, 202, 303, 404]),
    ]
    .map(|(date, values)| ManualCashBookDraft::new(Some(date), amounts(values)));
    let blank = ManualCashBookDraft::new(None, mar_draft.amounts);
    assert_eq!(
        book.save(OWNER, &blank).unwrap_err(),
        ManualCashBookError::MissingDate
    );
    let denied = ManualCashBookError::AccessDenied;
    // DEC-034: a Manager closes a new day (on a copy; the authority keeps it to today).
    assert!(book.clone().save(Role::Manager, &feb_draft).is_ok());
    assert_month(&book, feb, (0, [0; 7]));
    assert_month(&book, mar, (0, [0; 7]));
    let feb_row = book.save(OWNER, &feb_draft).unwrap();
    let mar_row = book.save(OWNER, &mar_draft).unwrap();
    let survivor = book.save(OWNER, &survivor_draft).unwrap();
    let mut draft = mar_row.draft();
    let rows = [(feb_row, (-11, 15)), (mar_row, (67, -26))];
    for ((row, differences), expected_day) in rows.into_iter().zip(&system) {
        let comparison = row.draft().compare(&fixture, OWNER).unwrap();
        assert_eq!(&comparison.system, expected_day);
        let actual = (
            comparison.cash_difference_rupees,
            comparison.online_difference_rupees,
        );
        assert_eq!(actual, differences);
        println!(
            "{} manual={:?}; differences Cash/Online={actual:?} whole rupees",
            row.date, row.amounts
        );
    }
    // DEC-034: a Manager copies the system figures into its draft as the Owner does.
    let mut manager = draft;
    manager.use_system_figures(&fixture, Role::Manager).unwrap();
    assert_eq!(draft, mar_row.draft());
    draft.use_system_figures(&fixture, OWNER).unwrap();
    assert_eq!(manager, draft);
    assert_eq!(book.month(Role::Manager, mar), Err(denied.clone()));
    let copied = amounts([2_100, 5_100, 0, 4_001, 5_002, 6_003, 7_004]);
    assert_eq!(draft.amounts, copied);
    assert_eq!(book.row(OWNER, mar_row.id), Ok(Some(mar_row)));
    let march_before = (2, [4_102, 5_204, 6_306, 7_408, 23_900, 15_612, 8_288]);
    assert_month(&book, mar, march_before);
    println!("{mar} copied draft (not saved yet): {:?}", draft.amounts);
    assert_accounts_unchanged();

    // Then: only Save persists the draft; only confirming Delete removes the selected row.
    let saved = book.save(OWNER, &draft).unwrap();
    assert_eq!(saved.id, mar_row.id);
    assert_eq!(saved.date, mar);
    assert_eq!(saved.amounts, copied);
    assert_eq!(book.row(OWNER, saved.id), Ok(Some(saved)));
    let february = (1, [1_201, 2_302, 3_403, 4_504, 10_500, 6_906, 3_594]);
    let march_after = (2, [4_102, 5_204, 6_306, 7_408, 19_800, 15_612, 4_188]);
    assert_month(&book, feb, february);
    assert_month(&book, mar, march_after);
    assert_accounts_unchanged();
    let declined = book.delete(OWNER, CashBookDeletion::Decline(saved.id));
    assert_eq!(declined, Ok(None));
    assert_eq!(book.row(OWNER, saved.id), Ok(Some(saved)));
    assert_accounts_unchanged();
    let deleted = book.delete(OWNER, CashBookDeletion::Confirm(saved.id));
    assert_eq!(deleted, Ok(Some(saved)));
    assert_eq!(book.row(OWNER, saved.id), Ok(None));
    assert_eq!(book.row(OWNER, survivor.id), Ok(Some(survivor)));
    assert_eq!(book.row(OWNER, feb_row.id), Ok(Some(feb_row)));
    assert_month(&book, mar, (1, [101, 202, 303, 404, 12_600, 606, 11_994]));
    assert_accounts_unchanged();
    println!("Unchanged customer ledger: {ledger:?}");
    println!("Online bill payment unsupported; March Online is a standalone receipt.");
}
