// Source-derived test model, not execution of the frozen HTML.
// Amounts are integer paise in the deliberately bounded synthetic fixture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Customer {
    pub id: &'static str,
    pub name: &'static str,
    pub opening: i64,
    pub created: Option<i64>,
    pub created_date_utc: &'static str,
    pub version: u64,
    pub modified: i64,
    pub device: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Bill {
    pub id: &'static str,
    pub customer_id: &'static str,
    pub customer_name: &'static str,
    pub date: &'static str,
    pub created: i64,
    pub total: i64,
    pub paid: i64,
    pub cancelled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Payment {
    pub id: &'static str,
    pub customer_id: &'static str,
    pub customer_name: &'static str,
    pub date: &'static str,
    pub created: i64,
    pub amount: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Records {
    pub customers: Vec<Customer>,
    pub bills: Vec<Bill>,
    pub payments: Vec<Payment>,
}

#[derive(Clone, Copy, Debug)]
pub(super) enum OpeningInput {
    Amount(i64),
    Blank,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum DeleteGuard {
    Debt(i64),
    Advance(i64),
    OfferConfirmation,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct LedgerRow {
    pub date: &'static str,
    pub kind: &'static str,
    pub debit: i64,
    pub credit: i64,
    pub running: i64,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct AgeRow {
    pub date: Option<&'static str>,
    pub due: i64,
    pub age: i64,
    pub bucket: &'static str,
}

impl Records {
    pub fn save_opening(&mut self, input: OpeningInput) -> bool {
        let target = &self.customers[0];
        if self
            .customers
            .iter()
            .any(|c| c.id != target.id && c.name.trim().eq_ignore_ascii_case(target.name.trim()))
        {
            return false;
        }
        let target = &mut self.customers[0];
        target.opening = match input {
            OpeningInput::Amount(amount) => amount,
            OpeningInput::Blank => 0,
        };
        target.version += 1;
        target.modified = 1_740_441_600_000;
        target.device = "synthetic-device";
        // syncCustomerName touches all matching rows, including cancelled bills.
        for bill in self.bills.iter_mut().filter(|b| b.customer_id == target.id) {
            bill.customer_name = target.name;
        }
        for payment in self
            .payments
            .iter_mut()
            .filter(|p| p.customer_id == target.id)
        {
            payment.customer_name = target.name;
        }
        true
    }

    pub fn exact(&self, id: &str) -> i64 {
        let customer = self.customers.iter().find(|c| c.id == id).unwrap();
        let bills: i64 = self
            .bills
            .iter()
            .filter(|b| b.customer_id == id && !b.cancelled)
            .map(|b| b.total - b.paid)
            .sum();
        let payments: i64 = self
            .payments
            .iter()
            .filter(|p| p.customer_id == id)
            .map(|p| p.amount)
            .sum();
        customer.opening + bills - payments
    }

    pub fn rounded(&self, id: &str) -> i64 {
        (self.exact(id) + 50).div_euclid(100)
    }

    pub fn delete_guard(&self, id: &str) -> DeleteGuard {
        let balance = self.rounded(id);
        match balance.cmp(&0) {
            std::cmp::Ordering::Greater => DeleteGuard::Debt(balance),
            std::cmp::Ordering::Less => DeleteGuard::Advance(-balance),
            std::cmp::Ordering::Equal => DeleteGuard::OfferConfirmation,
        }
    }

    pub fn ledger(&self, id: &str) -> Vec<LedgerRow> {
        let customer = self.customers.iter().find(|c| c.id == id).unwrap();
        let date = if customer.created.is_some() {
            customer.created_date_utc
        } else {
            ""
        };
        let mut entries = vec![(date, 0, "opening", customer.opening, 0)];
        entries.extend(
            self.bills
                .iter()
                .filter(|b| b.customer_id == id && !b.cancelled)
                .map(|b| {
                    (
                        b.date,
                        if b.created == 0 { 1 } else { b.created },
                        "bill",
                        b.total,
                        b.paid,
                    )
                }),
        );
        entries.extend(
            self.payments
                .iter()
                .filter(|p| p.customer_id == id)
                .map(|p| {
                    (
                        p.date,
                        if p.created == 0 { 1 } else { p.created },
                        "payment",
                        0,
                        p.amount,
                    )
                }),
        );
        entries.sort_by_key(|entry| (entry.0, entry.1));
        let mut running = 0;
        entries
            .into_iter()
            .map(|(date, _, kind, debit, credit)| {
                running += debit - credit;
                LedgerRow {
                    date,
                    kind,
                    debit,
                    credit,
                    running,
                }
            })
            .collect()
    }

    // This fixture uses only February 2025 bill dates and a fixed February 25 today.
    pub fn ageing(&self, id: &str) -> Vec<AgeRow> {
        let customer = self.customers.iter().find(|c| c.id == id).unwrap();
        let mut items = Vec::new();
        if customer.opening > 0 {
            items.push((None, customer.opening));
        }
        let mut pool: i64 = self
            .payments
            .iter()
            .filter(|p| p.customer_id == id)
            .map(|p| p.amount)
            .sum();
        pool += (-customer.opening).max(0);
        let mut bills: Vec<_> = self
            .bills
            .iter()
            .filter(|b| b.customer_id == id && !b.cancelled)
            .collect();
        bills.sort_by_key(|b| (b.date, b.created));
        for bill in bills {
            let due = bill.total - bill.paid;
            if due > 50 {
                items.push((Some(bill.date), due));
            }
            if due < -50 {
                pool += -due;
            }
        }
        items
            .into_iter()
            .filter_map(|(date, due)| {
                let used = pool.min(due);
                pool -= used;
                let due = due - used;
                if due <= 50 {
                    return None;
                }
                let age = date.map_or(999, |d| 25 - d[8..10].parse::<i64>().unwrap());
                let bucket = if age <= 15 {
                    "d15"
                } else if age <= 30 {
                    "d30"
                } else if age <= 60 {
                    "d60"
                } else {
                    "old"
                };
                Some(AgeRow {
                    date,
                    due,
                    age,
                    bucket,
                })
            })
            .collect()
    }
}
