use super::{
    ConfirmationError, ConfirmationFixture, DraftId, Money, ProposedPayment, Receipt, Review, Role,
};
use crate::{DraftCustomer, DraftLine, ProvisionalDraft};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EditChanges {
    pub retained_price: Money,
    pub appended_line: DraftLine,
    pub appended_price: Money,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SaleEdit {
    original: Review,
    receipt: Receipt,
    revision: u64,
    changes: EditChanges,
}

impl ConfirmationFixture {
    pub fn current_sale(
        &self,
        role: Role,
        id: DraftId,
    ) -> Result<(&Review, &Receipt), ConfirmationError> {
        if role != Role::Owner {
            return Err(ConfirmationError::AccessDenied);
        }
        self.current_sales
            .get(&id)
            .map(|(review, receipt, _)| (review, receipt))
            .ok_or(ConfirmationError::DraftNotFound)
    }

    pub fn begin_edit(
        &self,
        role: Role,
        id: DraftId,
        changes: EditChanges,
    ) -> Result<SaleEdit, ConfirmationError> {
        if role != Role::Owner {
            return Err(ConfirmationError::AccessDenied);
        }
        let (original, receipt, revision) = self
            .current_sales
            .get(&id)
            .ok_or(ConfirmationError::DraftNotFound)?;
        if *revision != 0
            || original.draft.lines().len() != 1
            || original.checkout.is_some()
            || original.draft.crate_row().is_some()
            || !matches!(original.payment, ProposedPayment::Named(_))
            || receipt.received.is_none_or(|amount| {
                amount.minor_units() <= 0 || amount.minor_units() >= receipt.total.minor_units()
            })
            || original.draft.lines()[0].product_id() == changes.appended_line.product_id()
            || changes.retained_price.minor_units() < 0
            || changes.appended_price.minor_units() < 0
        {
            return Err(ConfirmationError::InvalidDraft);
        }
        Ok(SaleEdit {
            original: original.clone(),
            receipt: receipt.clone(),
            revision: *revision,
            changes,
        })
    }

    pub fn save_edit(&mut self, role: Role, edit: &SaleEdit) -> Result<Receipt, ConfirmationError> {
        if role != Role::Owner {
            return Err(ConfirmationError::AccessDenied);
        }
        let id = edit.original.draft.id();
        if self.current_sales.get(&id)
            != Some(&(edit.original.clone(), edit.receipt.clone(), edit.revision))
        {
            return Err(ConfirmationError::StaleReview);
        }
        let mut updated = edit.original.clone();
        let lines = vec![updated.draft.lines()[0], edit.changes.appended_line];
        updated.draft = ProvisionalDraft::new(id, lines)
            .with_customer(updated.draft.customer().clone())
            .with_bill_date(updated.draft.bill_date());
        updated.prices = vec![edit.changes.retained_price, edit.changes.appended_price];
        let amounts = updated.amounts()?;
        let received = edit
            .receipt
            .received
            .ok_or(ConfirmationError::InvalidDraft)?
            .minor_units();
        let due = amounts
            .total
            .minor_units()
            .checked_sub(received)
            .filter(|value| *value >= 0)
            .ok_or(ConfirmationError::InvalidAmount)?;
        let old_due = edit
            .receipt
            .due
            .ok_or(ConfirmationError::InvalidDraft)?
            .minor_units();
        let DraftCustomer::Selected { id: customer, .. } = updated.draft.customer() else {
            return Err(ConfirmationError::InvalidDraft);
        };
        let balance = self
            .customer_balances
            .get(customer.value())
            .copied()
            .ok_or(ConfirmationError::InvalidDraft)?
            .checked_sub(old_due)
            .and_then(|value| value.checked_add(due))
            .ok_or(ConfirmationError::Overflow)?;
        let ledger = self
            .ledger
            .checked_sub(edit.receipt.total.minor_units())
            .and_then(|value| value.checked_add(amounts.total.minor_units()))
            .ok_or(ConfirmationError::Overflow)?;
        let mut appended = updated.clone();
        appended.draft = ProvisionalDraft::new(id, vec![edit.changes.appended_line]);
        appended.prices = vec![edit.changes.appended_price];
        let catalog = self.catalog.stage_append(&appended)?;
        let receipt = Receipt {
            draft_id: id,
            invoice: edit.receipt.invoice,
            quantity: amounts.packets,
            total: amounts.total,
            received: edit.receipt.received,
            due: Some(Money::from_minor_units(due)),
            customer_balance: Some(Money::from_minor_units(balance)),
        };
        self.catalog = catalog;
        self.ledger = ledger;
        self.customer_balances
            .insert(customer.value().to_owned(), balance);
        self.current_sales.insert(id, (updated, receipt.clone(), 1));
        Ok(receipt)
    }
}
