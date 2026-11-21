use alm_phase2_local::confirmation::{CustomerReceiptInput, ReceiptMode};
use alm_phase2_local::{DraftCustomerId, Money};

use super::receipt_date;

pub(super) struct Customer {
    pub(super) id: DraftCustomerId,
    pub(super) name: &'static str,
}

#[derive(Debug)]
pub(super) struct PickCustomer;

impl PickCustomer {
    pub(super) const fn toast(self) -> &'static str {
        "Pick a customer"
    }
}

// Only focused-input paths, with valid amount/date supplied at Save. The
// fixture uses plain ASCII names; stable sorting preserves source order on ties.
#[derive(Default)]
pub(super) struct ReceivePayment<'a> {
    pub(super) text: &'a str,
    pub(super) suggestions: Vec<&'a Customer>,
    pub(super) highlighted: Option<usize>,
    chosen: Option<&'a Customer>,
}

impl<'a> ReceivePayment<'a> {
    pub(super) fn input(&mut self, text: &'a str, customers: &'a [Customer]) {
        self.text = text;
        self.highlighted = None;
        let trimmed = text.trim();
        let query = trimmed.to_lowercase();
        self.suggestions = customers
            .iter()
            .filter(|customer| customer.name.to_lowercase().contains(&query))
            .collect();
        self.suggestions.sort_by_key(|customer| customer.name);
        self.suggestions.truncate(40);

        let mut exact = customers.iter().filter(|customer| customer.name == trimmed);
        self.chosen = match (exact.next(), exact.next()) {
            (Some(customer), None) => Some(customer),
            (None, _) | (Some(_), Some(_)) => None,
        };
    }

    pub(super) fn arrow_down(&mut self) {
        if let Some(last) = self.suggestions.len().checked_sub(1) {
            self.highlighted = Some(self.highlighted.map_or(0, |index| (index + 1).min(last)));
        }
    }

    pub(super) fn enter(&mut self) {
        if let Some(index) = self.highlighted {
            self.pick(index);
        }
    }

    pub(super) fn pick(&mut self, index: usize) {
        if let Some(&customer) = self.suggestions.get(index) {
            self.text = customer.name;
            self.suggestions.clear();
            self.chosen = Some(customer); // Setting the value does not fire input.
        }
    }

    pub(super) fn save(
        &self,
        amount: Money,
        mode: ReceiptMode,
    ) -> Result<CustomerReceiptInput, PickCustomer> {
        let customer = self.chosen.ok_or(PickCustomer)?;
        Ok(CustomerReceiptInput {
            customer_id: customer.id.clone(),
            date: receipt_date(),
            amount,
            mode,
        })
    }
}
