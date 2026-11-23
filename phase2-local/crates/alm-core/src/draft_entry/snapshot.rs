use super::{ActiveBillForm, ActiveDraft, CustomerId, CustomerPrefill};
use crate::{DraftCustomer, DraftCustomerId, ProvisionalDraft};

impl<'a> ActiveBillForm<'a> {
    pub fn reopen(draft: &'a ProvisionalDraft) -> Self {
        let (customer, customer_name) = match draft.customer() {
            DraftCustomer::Unassigned => (None, ""),
            DraftCustomer::Typed(name) => (None, name.as_str()),
            DraftCustomer::Selected { id, name } => (
                Some(CustomerPrefill::new(CustomerId::new(id.value()), name)),
                name.as_str(),
            ),
        };
        Self {
            draft: ActiveDraft::Existing(draft),
            customer,
            customer_name,
        }
    }

    pub fn snapshot(&self) -> ProvisionalDraft {
        let customer = match self.customer {
            Some(customer) => DraftCustomer::Selected {
                id: DraftCustomerId::new(customer.id().value().to_owned()),
                name: customer.name().to_owned(),
            },
            None if self.customer_name.is_empty() => DraftCustomer::Unassigned,
            None => DraftCustomer::Typed(self.customer_name.to_owned()),
        };
        self.draft().clone().with_customer(customer)
    }
}
