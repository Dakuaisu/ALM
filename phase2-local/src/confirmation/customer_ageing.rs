use std::collections::BTreeSet;

use super::{
    ConfirmationError, ConfirmationFixture, CustomerLedgerKind, Money, OpeningLedgerKind, Role,
};
use crate::{BillDate, DraftCustomer, DraftCustomerId, DraftId};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AgeingBucket {
    D15,
    D30,
    D60,
    Old,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgeingRow {
    pub customer_id: DraftCustomerId,
    pub draft_id: Option<DraftId>,
    pub invoice: Option<u64>,
    pub date: Option<BillDate>,
    pub age: i32,
    pub bucket: AgeingBucket,
    pub due: Money,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgeingReport {
    pub rows: Vec<AgeingRow>,
    pub d15: Money,
    pub d30: Money,
    pub d60: Money,
    pub old: Money,
}

struct Debt {
    draft_id: Option<DraftId>,
    invoice: Option<u64>,
    date: Option<BillDate>,
    amount: i128,
}

fn ordinal(date: BillDate) -> i32 {
    let (year, month, day) = date.parts();
    let year = i32::from(year) - 1;
    let before_year = 365 * year + year / 4 - year / 100 + year / 400;
    let offsets = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    let leap = (year + 1) % 4 == 0 && ((year + 1) % 100 != 0 || (year + 1) % 400 == 0);
    before_year + offsets[usize::from(month - 1)] + i32::from(day) + i32::from(leap && month > 2)
}

impl ConfirmationFixture {
    /// Projects legacy ageing from the opening-aware customer ledger.
    ///
    /// # Errors
    /// Denies Managers and reports ledger errors or amounts outside the money range.
    pub fn customer_ageing(
        &self,
        role: Role,
        today: BillDate,
    ) -> Result<AgeingReport, ConfirmationError> {
        if role != Role::Owner {
            return Err(ConfirmationError::AccessDenied);
        }
        let mut report = AgeingReport {
            rows: Vec::new(),
            d15: Money::from_minor_units(0),
            d30: Money::from_minor_units(0),
            d60: Money::from_minor_units(0),
            old: Money::from_minor_units(0),
        };
        let customers: BTreeSet<_> = self
            .current_sales
            .values()
            .filter_map(|(review, _, _)| match review.draft.customer() {
                DraftCustomer::Selected { id, .. } => Some(id.value().to_owned()),
                DraftCustomer::Typed(_) | DraftCustomer::Unassigned => None,
            })
            .chain(self.registered_customers.iter().cloned())
            .collect();
        for key in customers {
            let customer_id = DraftCustomerId::new(key);
            let ledger = self.customer_ledger_with_opening(role, &customer_id)?;
            let mut debts = Vec::new();
            let mut pool = 0i128;
            for entry in ledger.entries {
                let net =
                    i128::from(entry.debit.minor_units()) - i128::from(entry.credit.minor_units());
                match entry.kind {
                    OpeningLedgerKind::Opening if net > 0 => debts.insert(
                        0,
                        Debt {
                            draft_id: None,
                            invoice: None,
                            date: None,
                            amount: net,
                        },
                    ),
                    OpeningLedgerKind::Opening => pool -= net,
                    OpeningLedgerKind::Transaction(CustomerLedgerKind::Receipt { .. }) => {
                        pool += i128::from(entry.credit.minor_units())
                    }
                    OpeningLedgerKind::Transaction(CustomerLedgerKind::Bill {
                        draft_id,
                        invoice,
                    }) if net > 50 => debts.push(Debt {
                        draft_id: Some(draft_id),
                        invoice: Some(invoice),
                        date: entry.date,
                        amount: net,
                    }),
                    OpeningLedgerKind::Transaction(CustomerLedgerKind::Bill { .. })
                        if net < -50 =>
                    {
                        pool -= net
                    }
                    OpeningLedgerKind::Transaction(CustomerLedgerKind::Bill { .. }) => {}
                }
            }
            for debt in debts {
                let used = pool.min(debt.amount);
                let due = debt.amount - used;
                pool -= used;
                if due <= 50 {
                    continue;
                }
                let age = debt.date.map_or(999, |date| ordinal(today) - ordinal(date));
                let bucket = if age <= 15 {
                    AgeingBucket::D15
                } else if age <= 30 {
                    AgeingBucket::D30
                } else if age <= 60 {
                    AgeingBucket::D60
                } else {
                    AgeingBucket::Old
                };
                let due = i64::try_from(due).map_err(|_| ConfirmationError::Overflow)?;
                let total = match bucket {
                    AgeingBucket::D15 => &mut report.d15,
                    AgeingBucket::D30 => &mut report.d30,
                    AgeingBucket::D60 => &mut report.d60,
                    AgeingBucket::Old => &mut report.old,
                };
                *total = Money::from_minor_units(
                    total
                        .minor_units()
                        .checked_add(due)
                        .ok_or(ConfirmationError::Overflow)?,
                );
                report.rows.push(AgeingRow {
                    customer_id: customer_id.clone(),
                    draft_id: debt.draft_id,
                    invoice: debt.invoice,
                    date: debt.date,
                    age,
                    bucket,
                    due: Money::from_minor_units(due),
                });
            }
        }
        Ok(report)
    }
}
