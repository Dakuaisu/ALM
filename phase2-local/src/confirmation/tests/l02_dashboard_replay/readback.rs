use std::num::NonZeroU32;

use crate::confirmation::{
    BalanceMode, CashBookEntry, CashBookKind, ConfirmationFixture, CustomerLedgerKind,
    CustomerReceiptInput, OpeningBalance, OpeningLedgerKind, ReceiptMode, RequestId,
};
use crate::{
    BillDate, DraftCustomer, DraftCustomerId, DraftId, DraftLine, Money, ProductId,
    ProvisionalDraft, Role,
};

use ReceiptMode::{Cash, Online};

const OWNER: Role = Role::Owner;
const PRODUCT: ProductId = ProductId::new(1);
const CANCELLED: u64 = 92;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Source {
    Bill(u64, u64), // draft ID, invoice number
    Receipt(&'static str),
}

pub(super) type Detail = (Source, ReceiptMode, i64);

pub(super) struct Event {
    pub(super) date: BillDate,
    pub(super) created: usize, // deterministic replay stand-in for legacy `created`
    pub(super) detail: Detail,
}

pub(super) struct Sale {
    pub(super) payment: Event,
    pub(super) total: i64,
    pub(super) cancelled: bool,
}

pub(super) struct Readback {
    pub(super) sales: Vec<Sale>,
    pub(super) receipts: Vec<Event>,
}

enum Posting {
    Bill(u64, u32),
    Receipt(&'static str),
}

// Posting order is deliberate: the Feb 1 receipt falls between two active bills,
// while the Jan 31 bill is entered after all Feb 1 events.
const POSTINGS: [(&str, Posting, ReceiptMode, i64); 14] = [
    ("2023-12-31", Posting::Bill(91, 1), Cash, 111),
    ("2024-01-01", Posting::Bill(31, 1), Online, 149),
    ("2024-01-18", Posting::Bill(83, 1), Cash, 159),
    ("2024-01-19", Posting::Bill(19, 2), Cash, 227),
    ("2024-01-25", Posting::Bill(25, 1), Online, 251),
    ("2024-01-25", Posting::Receipt("r-jan25"), Cash, 51),
    ("2024-01-26", Posting::Bill(26, 1), Cash, 349),
    ("2024-01-30", Posting::Receipt("r-jan30"), Online, 151),
    ("2024-02-01", Posting::Bill(81, 1), Cash, 199),
    ("2024-02-01", Posting::Receipt("r-feb01"), Cash, 249),
    ("2024-02-01", Posting::Bill(5, 2), Online, 251),
    ("2024-02-01", Posting::Bill(CANCELLED, 1), Online, 701),
    ("2024-01-31", Posting::Bill(42, 2), Online, 449),
    ("2024-02-02", Posting::Bill(2, 1), Cash, 811),
];

const fn money(paise: i64) -> Money {
    Money::from_minor_units(paise)
}

pub(super) struct Replay {
    pub(super) fixture: ConfirmationFixture,
    buyer: DraftCustomerId,
    target: DraftCustomerId,
}

impl Replay {
    pub(super) fn seed() -> Self {
        let mut fixture = ConfirmationFixture::new(PRODUCT, 30, money(1_201)).unwrap();
        let buyer = DraftCustomerId::new("l02-buyer".into());
        let target = DraftCustomerId::new("receipt-target".into());
        fixture.register_customer(OWNER, &target).unwrap();
        fixture
            .save_customer_opening(
                OWNER,
                &target,
                OpeningBalance {
                    amount: Some(money(1_000)),
                    date: None,
                },
            )
            .unwrap();
        for (date, posting, mode, amount) in POSTINGS {
            let date = date.parse::<BillDate>().unwrap();
            match posting {
                Posting::Bill(draft, units) => {
                    let id = DraftId::new(draft);
                    let draft_row = ProvisionalDraft::new(
                        id,
                        vec![DraftLine::new(PRODUCT, NonZeroU32::new(units).unwrap())],
                    )
                    .with_bill_date(Some(date))
                    .with_customer(DraftCustomer::Selected {
                        id: buyer.clone(),
                        name: "Synthetic Dashboard Buyer".into(),
                    });
                    fixture.register_draft(draft_row).unwrap();
                    let review = fixture
                        .review_named_with_mode(OWNER, id, money(amount), mode)
                        .unwrap();
                    fixture
                        .confirm(OWNER, RequestId::new(draft), &review)
                        .unwrap();
                    if draft == CANCELLED {
                        fixture.cancel_sale(OWNER, id).unwrap();
                    }
                }
                Posting::Receipt(_) => fixture
                    .record_customer_receipt(
                        OWNER,
                        CustomerReceiptInput {
                            customer_id: target.clone(),
                            date,
                            amount: money(amount),
                            mode,
                        },
                    )
                    .unwrap(),
            }
        }
        Self {
            fixture,
            buyer,
            target,
        }
    }

    pub(super) fn read(&self) -> Readback {
        let f = &self.fixture;
        let buyer = f.customer_ledger(OWNER, &self.buyer).unwrap();
        let target = f.customer_ledger_with_opening(OWNER, &self.target).unwrap();
        assert_eq!(target.entries.len(), 4);
        let opening = &target.entries[0];
        assert_eq!(
            (&opening.kind, opening.date, opening.debit, opening.credit),
            (&OpeningLedgerKind::Opening, None, money(1_000), money(0))
        );
        assert_eq!(target.closing, money(549));
        assert_eq!(f.customer_due(self.target.value()), money(549));
        println!(
            "receipt-target opening and closing paise: expected=(1000, 549) observed=({}, {})",
            opening.debit.minor_units(),
            target.closing.minor_units()
        );

        let mut sales = Vec::new();
        let mut receipts = Vec::new();
        for (created, (date, posting, mode, amount)) in POSTINGS.into_iter().enumerate().rev() {
            let date = date.parse::<BillDate>().unwrap();
            match posting {
                Posting::Bill(draft, _) => {
                    let id = DraftId::new(draft);
                    let (review, posted) = f.current_sale(OWNER, id).unwrap();
                    let preview = f
                        .bill_preview(OWNER, id, BalanceMode::AsOnBillDate)
                        .unwrap();
                    assert_eq!(
                        (posted.draft_id, preview.date, preview.cancelled),
                        (id, date, draft == CANCELLED)
                    );
                    let (total, paid) = (preview.total.minor_units(), preview.paid.minor_units());
                    assert!(paid > 0 && paid < total);
                    assert_eq!(preview.invoice, posted.invoice);
                    let source = Source::Bill(posted.draft_id.value(), posted.invoice);
                    if draft == CANCELLED {
                        let observed = (source, total, paid, preview.cancelled);
                        let expected = (Source::Bill(92, 9), 1201, 701, true);
                        println!(
                            "{date} cancelled sale (source, sales, paid, cancelled): expected={expected:?} observed={observed:?}"
                        );
                        assert_eq!(observed, expected);
                    }
                    let kind = CustomerLedgerKind::Bill {
                        draft_id: posted.draft_id,
                        invoice: posted.invoice,
                    };
                    let mut rows = buyer.entries.iter().filter(|row| row.kind == kind);
                    let ledger_fact = rows.next().map(|row| (row.date, row.debit, row.credit));
                    assert_eq!(
                        ledger_fact,
                        (!preview.cancelled).then_some((preview.date, preview.total, preview.paid))
                    );
                    assert!(rows.next().is_none(), "duplicate ledger bill: {source:?}");
                    sales.push(Sale {
                        payment: Event {
                            date: preview.date,
                            created,
                            detail: (source, review.payment_mode(), paid),
                        },
                        total,
                        cancelled: preview.cancelled,
                    });
                }
                Posting::Receipt(id) => {
                    // Cash Book has no receipt identifier. Bind a replay ID only after a
                    // unique match in this customer's ledger by document date, mode and credit.
                    let kind = OpeningLedgerKind::Transaction(CustomerLedgerKind::Receipt { mode });
                    let mut matches = target.entries.iter().filter(|row| {
                        row.kind == kind && row.date == Some(date) && row.credit == money(amount)
                    });
                    let row = matches
                        .next()
                        .expect("dated standalone receipt in target ledger");
                    assert!(matches.next().is_none(), "ambiguous receipt: {id}");
                    assert_eq!(row.debit, money(0));
                    receipts.push(Event {
                        date: row.date.unwrap(),
                        created,
                        detail: (Source::Receipt(id), mode, row.credit.minor_units()),
                    });
                }
            }
        }
        assert_eq!(buyer.entries.len(), 10);
        Readback { sales, receipts }
    }

    pub(super) fn cashbook_paise(&self, date: BillDate, readback: &Readback) -> [i64; 2] {
        let day = self.fixture.cashbook_day_close(OWNER, date).unwrap();
        let events = readback
            .sales
            .iter()
            .filter(|sale| !sale.cancelled)
            .map(|sale| &sale.payment)
            .chain(&readback.receipts)
            .filter(|event| event.date == date);
        let mut unmatched = day.entries;
        for &(source, mode, paise) in events.map(|event| &event.detail) {
            let kind = match source {
                Source::Bill(draft, invoice) => CashBookKind::Bill {
                    draft_id: DraftId::new(draft),
                    invoice,
                },
                Source::Receipt(_) => CashBookKind::Receipt,
            };
            let row = CashBookEntry {
                kind,
                mode,
                amount: money(paise),
            };
            let index = unmatched
                .iter()
                .position(|entry| *entry == row)
                .unwrap_or_else(|| panic!("{date}: Cash Book missing {source:?}: {row:?}"));
            unmatched.swap_remove(index);
        }
        assert!(
            unmatched.is_empty(),
            "{date}: unexpected Cash Book rows: {unmatched:?}"
        );
        [day.cash.minor_units(), day.online.minor_units()]
    }
}
