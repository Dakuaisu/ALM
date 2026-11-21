//! Source-derived replay for known-cost regular bills only; not a production Sales view.

use std::cmp::Reverse;

use crate::confirmation::{
    ConfirmationError, ConfirmationFixture, PreviewPayment, Receipt, ReceiptMode,
};
use crate::{BillDate, DraftCustomer, Role};

type Bounds = (Option<BillDate>, Option<BillDate>);

#[derive(Clone, Copy)]
pub(super) enum Period {
    Today,
    ThisMonth,
    All,
    Custom(Bounds),
}

impl Period {
    pub(super) fn bounds(self, today: BillDate) -> Bounds {
        match self {
            Self::Today => (Some(today), Some(today)),
            Self::ThisMonth => {
                let (year, month, _) = today.parts();
                (Some(BillDate::new(year, month, 1).unwrap()), None)
            }
            Self::All => (None, None),
            Self::Custom(bounds) => bounds,
        }
    }
}

pub(super) struct History<'a> {
    fixture: &'a ConfirmationFixture,
    pub(super) rows: Vec<SaleRow<'a>>,
    receipts: Vec<(usize, &'a PreviewPayment)>,
}

pub(super) struct SaleRow<'a> {
    pub(super) bill: &'a Receipt,
    date: BillDate,
    mode: ReceiptMode,
    pub(super) cancelled: bool,
    pub(super) balance_paise: i64,
}

impl SaleRow<'_> {
    const fn paid_paise(&self) -> i64 {
        match self.bill.received {
            Some(paid) => paid.minor_units(),
            None => 0,
        }
    }

    pub(super) const fn remainder_paise(&self) -> i64 {
        self.bill.total.minor_units() - self.paid_paise()
    }
}

pub(super) fn sales_history<'a>(
    fixture: &'a ConfirmationFixture,
    bounds: Bounds,
    search: &str,
) -> History<'a> {
    let query = search.to_lowercase();
    let matches = |name: &str| name.to_lowercase().contains(&query);
    let in_range = |date: BillDate| {
        bounds.0.is_none_or(|first| date.parts() >= first.parts())
            && bounds.1.is_none_or(|last| date.parts() <= last.parts())
    };
    let mut rows: Vec<_> = fixture
        .current_sales
        .values()
        .filter_map(|(review, bill, _)| {
            let date = review.draft.bill_date().unwrap();
            let (name, customer_id) = match review.draft.customer() {
                DraftCustomer::Selected { id, name } => (name.as_str(), Some(id)),
                DraftCustomer::Typed(name) => (name.as_str(), None),
                DraftCustomer::Unassigned => ("", None),
            };
            let item_match = || {
                fixture.posted_sale_items[&bill.draft_id]
                    .iter()
                    .any(|item| item.item.as_deref().is_some_and(matches))
            };
            if !in_range(date) || !(matches(name) || item_match()) {
                return None;
            }
            let mut row = SaleRow {
                bill,
                date,
                mode: review.payment_mode(),
                cancelled: fixture.lifecycle[&bill.draft_id].0,
                balance_paise: 0,
            };
            if !row.cancelled {
                row.balance_paise = customer_id.map_or_else(
                    || row.remainder_paise(),
                    |id| fixture.customer_due(id.value()).minor_units(),
                );
            }
            Some(row)
        })
        .collect();
    // The fixture has no creation timestamps. Monotonic invoice assignment captures posting order here.
    rows.sort_by_key(|row| Reverse(row.bill.invoice));
    // Receipts have neither their own stored ID nor a saved name in this fixture. Use append
    // positions as replay IDs and resolve names from the full fixture (stable synthetic names).
    let receipts = fixture
        .preview_payments
        .iter()
        .enumerate()
        .filter_map(|(index, payment)| {
            let party_matches = || {
                fixture
                    .current_sales
                    .values()
                    .any(|(review, _, _)| match review.draft.customer() {
                        DraftCustomer::Selected { id, name } => {
                            id.value() == payment.customer_id && matches(name)
                        }
                        DraftCustomer::Typed(_) | DraftCustomer::Unassigned => false,
                    })
            };
            (in_range(payment.date) && (query.is_empty() || party_matches()))
                .then_some((index + 1, payment))
        })
        .collect();
    History {
        fixture,
        rows,
        receipts,
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Profit {
    pub(super) bill_ids: Vec<u64>,
    pub(super) revenue_paise: i64,
    pub(super) cost_paise: i64,
}

impl Profit {
    pub(super) const fn profit_paise(&self) -> i64 {
        self.revenue_paise - self.cost_paise
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Source {
    Bill(u64),
    Receipt(usize),
}

#[derive(Debug)]
pub(super) struct Collected {
    pub(super) bill_ids: Vec<u64>,
    pub(super) receipt_ids: Vec<usize>,
    pub(super) details: Vec<(Source, i64)>,
    pub(super) cash_paise: i64,
    pub(super) online_paise: i64,
}

impl History<'_> {
    pub(super) fn row_ids(&self) -> Vec<u64> {
        self.rows
            .iter()
            .map(|row| row.bill.draft_id.value())
            .collect()
    }

    fn active(&self) -> impl Iterator<Item = &SaleRow<'_>> {
        self.rows.iter().filter(|row| !row.cancelled)
    }

    pub(super) fn active_packet_count(&self) -> u32 {
        self.active().map(|row| row.bill.quantity).sum()
    }

    pub(super) fn tile_paise(&self) -> [i64; 4] {
        let sales = self.active().map(|row| row.bill.total.minor_units()).sum();
        let paid = self.active().map(SaleRow::paid_paise).sum();
        let received = self
            .receipts
            .iter()
            .map(|(_, pay)| pay.amount.minor_units())
            .sum();
        [sales, paid, received, paid + received]
    }

    pub(super) fn profit(&self, role: Role) -> Result<Profit, ConfirmationError> {
        match role {
            Role::Owner => {}
            Role::Manager => return Err(ConfirmationError::AccessDenied),
        }
        let mut profit = Profit {
            bill_ids: Vec::new(),
            revenue_paise: 0,
            cost_paise: 0,
        };
        for row in self.active() {
            profit.bill_ids.push(row.bill.draft_id.value());
            profit.revenue_paise += row.bill.total.minor_units();
            profit.cost_paise += self
                .fixture
                .catalog
                .frozen_cost(row.bill.draft_id)?
                .ok_or(ConfirmationError::InvalidDraft)?;
        }
        Ok(profit)
    }

    pub(super) fn collected(&self) -> Collected {
        let items = self.active().filter(|row| row.paid_paise() > 0).map(|row| {
            (
                (row.date.parts(), row.bill.invoice, 0),
                Source::Bill(row.bill.draft_id.value()),
                row.paid_paise(),
                row.mode,
            )
        });
        let receipts = self.receipts.iter().map(|&(id, pay)| {
            (
                (pay.date.parts(), pay.after_invoice, id),
                Source::Receipt(id),
                pay.amount.minor_units(),
                pay.mode.unwrap(),
            )
        });
        let mut details: Vec<_> = items.chain(receipts).collect();
        details.sort_by_key(|&(key, _, _, _)| Reverse(key));
        let (mut cash_paise, mut online_paise) = (0, 0);
        let details = details
            .into_iter()
            .map(|(_, source, paise, mode)| {
                match mode {
                    ReceiptMode::Cash => cash_paise += paise,
                    ReceiptMode::Online => online_paise += paise,
                }
                (source, paise)
            })
            .collect();
        Collected {
            bill_ids: self.active().map(|row| row.bill.draft_id.value()).collect(),
            receipt_ids: self.receipts.iter().map(|&(id, _)| id).collect(),
            details,
            cash_paise,
            online_paise,
        }
    }
}

pub(super) const fn round_rupees(paise: i64) -> i64 {
    paise.div_euclid(100) + if paise.rem_euclid(100) >= 50 { 1 } else { 0 }
}

pub(super) fn money_r(paise: i64) -> String {
    // Only fixture-sized amounts: currency prefix and whole rupees, not locale grouping.
    format!("₹{}", round_rupees(paise))
}
