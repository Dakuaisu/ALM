use super::{
    ConfirmationError, ConfirmationFixture, DraftId, Money, PostedLineItem, ProposedPayment,
    Receipt, Review, Role,
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
        if self
            .lifecycle
            .get(&id)
            .is_some_and(|(cancelled, _)| *cancelled)
        {
            return Err(ConfirmationError::InvalidDraft);
        }
        if *revision != 0
            || original.draft.lines().len() != 1
            || original.checkout.is_some()
            || !original.draft.crate_rows().is_empty()
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
        self.save_edit_inner(
            role,
            edit,
            None::<fn() -> Result<super::CreationKey, ConfirmationError>>,
        )
    }

    /// Saves an edit, using the controlled edit time only when the original key is missing or zero.
    pub fn save_edit_with_creation_key(
        &mut self,
        role: Role,
        edit: &SaleEdit,
        edit_time: super::CreationKey,
    ) -> Result<Receipt, ConfirmationError> {
        self.save_edit_inner(role, edit, Some(|| Ok(edit_time)))
    }

    pub(crate) fn save_edit_with_clock(
        &mut self,
        role: Role,
        edit: &SaleEdit,
        clock: impl FnOnce() -> Result<super::CreationKey, ConfirmationError>,
    ) -> Result<Receipt, ConfirmationError> {
        self.save_edit_inner(role, edit, Some(clock))
    }

    fn save_edit_inner<F>(
        &mut self,
        role: Role,
        edit: &SaleEdit,
        clock: Option<F>,
    ) -> Result<Receipt, ConfirmationError>
    where
        F: FnOnce() -> Result<super::CreationKey, ConfirmationError>,
    {
        if role != Role::Owner {
            return Err(ConfirmationError::AccessDenied);
        }
        let id = edit.original.draft.id();
        if self
            .lifecycle
            .get(&id)
            .is_some_and(|(cancelled, _)| *cancelled)
        {
            return Err(ConfirmationError::StaleReview);
        }
        if self.current_sales.get(&id)
            != Some(&(edit.original.clone(), edit.receipt.clone(), edit.revision))
        {
            return Err(ConfirmationError::StaleReview);
        }
        let mut updated = edit.original.clone();
        let lines = vec![updated.draft.lines()[0], edit.changes.appended_line];
        updated.draft = ProvisionalDraft::new(id, lines)
            .with_customer(updated.draft.customer().clone())
            .with_bill_date(updated.draft.bill_date())
            .with_loading_rate(updated.draft.loading_rate());
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
        let movement = self
            .movement_epoch
            .checked_add(1)
            .ok_or(ConfirmationError::Overflow)?;
        let receipt = Receipt {
            draft_id: id,
            invoice: edit.receipt.invoice,
            quantity: amounts.packets,
            total: amounts.total,
            received: edit.receipt.received,
            due: Some(Money::from_minor_units(due)),
            customer_balance: Some(Money::from_minor_units(balance)),
        };
        let first_post = clock
            .map(|clock| {
                self.first_posts
                    .iter()
                    .position(|(draft, _)| *draft == id)
                    .map(|index| (index, clock))
                    .ok_or(ConfirmationError::InvalidDraft)
            })
            .transpose()?;
        let posted_items = self
            .posted_sale_items
            .get_mut(&id)
            .ok_or(ConfirmationError::StaleReview)?;
        let first_post = match first_post {
            Some((index, clock))
                if self.first_posts[index]
                    .1
                    .is_none_or(|value| value == super::CreationKey::new(0)) =>
            {
                Some((index, clock()?))
            }
            Some(_) | None => None,
        };
        posted_items.push(PostedLineItem::capture(
            &catalog,
            edit.changes.appended_line.product_id(),
        ));
        if let Some((index, time)) = first_post {
            self.first_posts[index].1 = Some(time);
        }
        self.catalog = catalog;
        self.movement_epoch = movement;
        self.lifecycle.insert(id, (false, movement));
        self.ledger = ledger;
        self.customer_balances
            .insert(customer.value().to_owned(), balance);
        self.current_sales.insert(id, (updated, receipt.clone(), 1));
        Ok(receipt)
    }
}

/// One posted line after an in-place edit: its new quantity and rate (`li.qty`, `li.rate`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LineEdit {
    pub quantity: std::num::NonZeroU32,
    pub price: Money,
}

impl LineEdit {
    /// Reads a typed line: a whole quantity above zero and exact non-negative rupees.
    pub fn from_text(quantity: &str, rate: &str) -> Result<Self, ConfirmationError> {
        Ok(Self {
            quantity: quantity
                .trim()
                .parse()
                .map_err(|_| ConfirmationError::InvalidAmount)?,
            price: Money::parse_rupees(rate).map_err(|_| ConfirmationError::InvalidAmount)?,
        })
    }
}

impl ConfirmationFixture {
    /// The HTML's Edit Bill on a named bill: replaces each existing line's quantity and rate in
    /// place, ordinary lines first and then each crate row (crates and rate; its contents stay),
    /// keeping customer, date, invoice and the amount paid. Stock, lots, the customer's balance,
    /// the ledger and the frozen sale cost follow (each crate component's packets follow its
    /// crate count from or back to its lots, as a line edit); Cash Book (paid) does not change.
    /// A bill of crate rows only keeps its posted loading charge per packet. A second edit is
    /// allowed.
    ///
    /// # Errors
    /// Denies Managers; rejects missing, cancelled or checkout bills, a line count change,
    /// negative rates, a total below the amount paid, and stock shortages, all before mutation.
    pub fn edit_posted_lines(
        &mut self,
        role: Role,
        id: DraftId,
        edits: &[LineEdit],
    ) -> Result<Receipt, ConfirmationError> {
        if role != Role::Owner {
            return Err(ConfirmationError::AccessDenied);
        }
        let (original, receipt, revision) = self
            .current_sales
            .get(&id)
            .cloned()
            .ok_or(ConfirmationError::DraftNotFound)?;
        let ordinary = original.draft.lines().len();
        let crates = original.draft.crate_rows();
        if self
            .lifecycle
            .get(&id)
            .is_some_and(|(cancelled, _)| *cancelled)
            || (original.checkout.is_some() && ordinary != 0)
            || (!crates.is_empty() && !original.is_named_crate_sale())
            || !matches!(original.payment, ProposedPayment::Named(_))
            || edits.len() != ordinary + crates.len()
            || edits.iter().any(|edit| edit.price.minor_units() < 0)
        {
            return Err(ConfirmationError::InvalidDraft);
        }
        let mut updated = original.clone();
        let lines: Vec<DraftLine> = original
            .draft
            .lines()
            .iter()
            .zip(edits)
            .map(|(line, edit)| DraftLine::new(line.product_id(), edit.quantity))
            .collect();
        let mut draft = ProvisionalDraft::new(id, lines.clone())
            .with_customer(original.draft.customer().clone())
            .with_bill_date(original.draft.bill_date())
            .with_loading_rate(original.draft.loading_rate());
        for (row, edit) in crates.iter().zip(&edits[ordinary..]) {
            draft.add_crate_row(
                crate::CrateRow::new(edit.quantity, row.components().to_vec())
                    .with_price(row.name().unwrap_or_default().to_owned(), edit.price),
            );
        }
        updated.draft = draft;
        updated.prices = edits.iter().map(|edit| edit.price).collect();
        let amounts = updated.amounts()?;
        let received = receipt
            .received
            .ok_or(ConfirmationError::InvalidDraft)?
            .minor_units();
        let due = amounts
            .total
            .minor_units()
            .checked_sub(received)
            .filter(|value| *value >= 0)
            .ok_or(ConfirmationError::InvalidAmount)?;
        let old_due = receipt
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
            .checked_sub(receipt.total.minor_units())
            .and_then(|value| value.checked_add(amounts.total.minor_units()))
            .ok_or(ConfirmationError::Overflow)?;
        let before = original.crate_packet_demands()?.unwrap_or_default();
        let after = updated.crate_packet_demands()?.unwrap_or_default();
        let moves: Vec<_> = original
            .draft
            .lines()
            .iter()
            .chain(&before)
            .zip(lines.iter().chain(&after))
            .map(|(old, new)| {
                (
                    old.product_id().value(),
                    old.quantity().get(),
                    new.quantity().get(),
                )
            })
            .collect();
        let catalog = self.catalog.stage_line_edit(id.value(), &moves)?;
        let movement = self
            .movement_epoch
            .checked_add(1)
            .ok_or(ConfirmationError::Overflow)?;
        let revision = revision.checked_add(1).ok_or(ConfirmationError::Overflow)?;
        let receipt = Receipt {
            quantity: amounts.packets,
            total: amounts.total,
            due: Some(Money::from_minor_units(due)),
            customer_balance: Some(Money::from_minor_units(balance)),
            ..receipt
        };
        self.catalog = catalog;
        self.movement_epoch = movement;
        self.lifecycle.insert(id, (false, movement));
        self.ledger = ledger;
        self.customer_balances
            .insert(customer.value().to_owned(), balance);
        self.current_sales
            .insert(id, (updated, receipt.clone(), revision));
        Ok(receipt)
    }

    /// The HTML's Edit Bill on a bill of one crate row (`sell_crate`): its crates and rate.
    ///
    /// # Errors
    /// As [`Self::edit_posted_lines`]; rejects a bill that is not one named crate row.
    pub fn edit_crate_bill(
        &mut self,
        role: Role,
        id: DraftId,
        edit: LineEdit,
    ) -> Result<Receipt, ConfirmationError> {
        if role != Role::Owner {
            return Err(ConfirmationError::AccessDenied);
        }
        let (original, _) = self.current_sale(role, id)?;
        if !original.draft.lines().is_empty()
            || original.draft.crate_rows().len() != 1
            || !original.is_named_crate_sale()
        {
            return Err(ConfirmationError::InvalidDraft);
        }
        self.edit_posted_lines(role, id, &[edit])
    }
}

#[cfg(test)]
#[path = "edit_lines_tests.rs"]
mod edit_lines_tests;
