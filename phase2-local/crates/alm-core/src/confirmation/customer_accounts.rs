use super::{
    ConfirmationError, ConfirmationFixture, DraftId, Money, PreviewPayment, ProposedPayment, Role,
};
use super::{OpeningLedger, OpeningLedgerEntry, OpeningLedgerKind};
use crate::{BillDate, DraftCustomer, DraftCustomerId};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReceiptMode {
    Cash,
    Online,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CustomerReceiptInput {
    pub customer_id: DraftCustomerId,
    pub date: BillDate,
    pub amount: Money,
    pub mode: ReceiptMode,
}

/// Saved selected label and caller-supplied creation order for a receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CustomerReceiptMetadata {
    pub selected_name: String,
    pub creation_key: super::CreationKey,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CustomerLedgerKind {
    Bill { draft_id: DraftId, invoice: u64 },
    Receipt { mode: ReceiptMode },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CustomerLedgerEntry {
    pub date: BillDate,
    pub kind: CustomerLedgerKind,
    pub debit: Money,
    pub credit: Money,
    pub balance: Money,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CustomerLedger {
    pub entries: Vec<CustomerLedgerEntry>,
    pub closing: Money,
}

impl ConfirmationFixture {
    pub(super) fn bill_creation_key(&self, id: DraftId) -> Option<super::CreationKey> {
        self.first_posts
            .iter()
            .find(|(draft, _)| *draft == id)
            .and_then(|(_, key)| *key)
    }

    pub fn record_customer_receipt(
        &mut self,
        role: Role,
        input: CustomerReceiptInput,
    ) -> Result<(), ConfirmationError> {
        self.save_customer_receipt(role, input, None)
    }

    /// Saves a receipt with its selected customer label and creation key.
    pub fn record_customer_receipt_with_metadata(
        &mut self,
        role: Role,
        input: CustomerReceiptInput,
        metadata: CustomerReceiptMetadata,
    ) -> Result<(), ConfirmationError> {
        self.save_customer_receipt(role, input, Some(metadata))
    }

    fn save_customer_receipt(
        &mut self,
        role: Role,
        input: CustomerReceiptInput,
        metadata: Option<CustomerReceiptMetadata>,
    ) -> Result<(), ConfirmationError> {
        if role != Role::Owner {
            return Err(ConfirmationError::AccessDenied);
        }
        if metadata
            .as_ref()
            .is_some_and(|value| value.selected_name.trim().is_empty())
        {
            return Err(ConfirmationError::MissingCustomerName);
        }
        if input.amount.minor_units() == 0 {
            return Err(ConfirmationError::InvalidAmount);
        }
        let key = input.customer_id.value();
        if !self.registered_customers.contains(key)
            && !self.current_sales.values().any(|(review, _, _)| {
                matches!(review.draft.customer(), DraftCustomer::Selected { id, .. } if id == &input.customer_id)
                    && matches!(review.payment, ProposedPayment::Named(_))
                    && review.checkout.is_none()
                    && review.draft.crate_row().is_none()
            })
        {
            return Err(ConfirmationError::InvalidDraft);
        }
        let balance = self
            .customer_balances
            .get(key)
            .copied()
            .unwrap_or_default()
            .checked_sub(input.amount.minor_units())
            .ok_or(ConfirmationError::Overflow)?;
        self.preview_payments.push(PreviewPayment {
            customer_id: key.to_owned(),
            amount: input.amount,
            date: input.date,
            after_invoice: self.next_invoice - 1,
            mode: Some(input.mode),
            metadata,
        });
        self.customer_balances.insert(key.to_owned(), balance);
        Ok(())
    }

    pub fn customer_ledger(
        &self,
        role: Role,
        customer_id: &DraftCustomerId,
    ) -> Result<CustomerLedger, ConfirmationError> {
        let ledger = self.customer_ledger_with_opening(role, customer_id)?;
        let entries = ledger
            .entries
            .into_iter()
            .filter_map(|entry| match entry.kind {
                OpeningLedgerKind::Opening => None,
                OpeningLedgerKind::Transaction(kind) => {
                    entry.date.map(|date| CustomerLedgerEntry {
                        date,
                        kind,
                        debit: entry.debit,
                        credit: entry.credit,
                        balance: entry.balance,
                    })
                }
            })
            .collect();
        Ok(CustomerLedger {
            entries,
            closing: ledger.closing,
        })
    }

    pub fn customer_ledger_with_opening(
        &self,
        role: Role,
        customer_id: &DraftCustomerId,
    ) -> Result<OpeningLedger, ConfirmationError> {
        if role != Role::Owner {
            return Err(ConfirmationError::AccessDenied);
        }
        let mut rows = Vec::new();
        let mut known = self.registered_customers.contains(customer_id.value());
        for (id, (review, receipt, _)) in &self.current_sales {
            if !matches!(review.draft.customer(), DraftCustomer::Selected { id, .. } if id == customer_id)
            {
                continue;
            }
            known = true;
            if self
                .lifecycle
                .get(id)
                .ok_or(ConfirmationError::InvalidDraft)?
                .0
            {
                continue;
            }
            if !matches!(review.payment, ProposedPayment::Named(_))
                || review.checkout.is_some()
                || review.draft.crate_row().is_some()
            {
                return Err(ConfirmationError::InvalidDraft);
            }
            let date = review
                .draft
                .bill_date()
                .ok_or(ConfirmationError::InvalidDraft)?;
            let paid = receipt.received.ok_or(ConfirmationError::InvalidDraft)?;
            rows.push((
                (
                    Some(date.parts()),
                    1u8,
                    self.bill_creation_key(*id),
                    receipt.invoice,
                    0usize,
                ),
                OpeningLedgerEntry {
                    date: Some(date),
                    kind: OpeningLedgerKind::Transaction(CustomerLedgerKind::Bill {
                        draft_id: *id,
                        invoice: receipt.invoice,
                    }),
                    debit: receipt.total,
                    credit: paid,
                    balance: Money::from_minor_units(0),
                },
            ));
        }
        if !known {
            return Err(ConfirmationError::InvalidDraft);
        }
        for (index, payment) in self.preview_payments.iter().enumerate() {
            if payment.customer_id != customer_id.value() {
                continue;
            }
            let mode = payment.mode.ok_or(ConfirmationError::InvalidDraft)?;
            rows.push((
                (
                    Some(payment.date.parts()),
                    1u8,
                    payment
                        .metadata
                        .as_ref()
                        .map(|metadata| metadata.creation_key),
                    payment.after_invoice,
                    index + 1,
                ),
                OpeningLedgerEntry {
                    date: Some(payment.date),
                    kind: OpeningLedgerKind::Transaction(CustomerLedgerKind::Receipt { mode }),
                    debit: Money::from_minor_units(0),
                    credit: payment.amount,
                    balance: Money::from_minor_units(0),
                },
            ));
        }
        if let Some(opening) = self.customer_openings.get(customer_id.value()) {
            rows.push((
                (opening.date.map(|date| date.parts()), 0, None, 0, 0),
                OpeningLedgerEntry {
                    date: opening.date,
                    kind: OpeningLedgerKind::Opening,
                    debit: opening.amount,
                    credit: Money::from_minor_units(0),
                    balance: Money::from_minor_units(0),
                },
            ));
        }
        let keyed = rows.iter().any(|((_, _, key, _, _), _)| key.is_some());
        if keyed {
            // The source's stable tie order is opening, then newest bills,
            // then newest receipts. Missing and zero keys sort as one only here.
            rows.sort_by_key(|((date, kind, key, anchor, index), _)| {
                let effective = key
                    .filter(|key| *key != super::CreationKey::new(0))
                    .unwrap_or(super::CreationKey::new(1));
                (
                    *date,
                    *kind,
                    effective,
                    u8::from(*index != 0),
                    std::cmp::Reverse(if *index == 0 { *anchor } else { 0 }),
                    std::cmp::Reverse(*index),
                )
            });
        } else {
            rows.sort_by_key(|((date, kind, _, anchor, index), _)| (*date, *kind, *anchor, *index));
        }
        let mut balance = 0i64;
        let mut entries = Vec::with_capacity(rows.len());
        for (_, mut entry) in rows {
            balance = balance
                .checked_add(entry.debit.minor_units())
                .and_then(|v| v.checked_sub(entry.credit.minor_units()))
                .ok_or(ConfirmationError::Overflow)?;
            entry.balance = Money::from_minor_units(balance);
            entries.push(entry);
        }
        Ok(OpeningLedger {
            entries,
            closing: Money::from_minor_units(balance),
        })
    }
}
