//! Applies one intent to a copy of the simulated state.
use alm_core::confirmation::manual_cashbook::{CashBookDeletion, ManualCashBookDraft};
use alm_core::confirmation::owner_billing::{
    BillingDirectory, CrateSaleTerms, ReceiptTerms, SaleTerms, post_saved_sale, priced_draft,
    save_receipt_at, sell_crate,
};
use alm_core::confirmation::{LineEdit, ReceiptMode};
use alm_core::{DraftId, Role};

use super::{State, commands, rejected};
use crate::{AuthorityError, Intent, Outcome, SaleReceipt};

pub(super) fn apply(
    state: &mut State,
    directory: &BillingDirectory,
    intent: &Intent,
    stamp: u64,
) -> Result<Outcome, AuthorityError> {
    let role = Role::Owner;
    let fixture = &mut state.fixture;
    match intent {
        Intent::PostSale(sale) => {
            let id = sale.draft.id();
            if fixture.current_sale(role, id).is_ok() {
                return Err(AuthorityError::AlreadyPosted);
            }
            // Crate rows beside the lines carry their copied name and rate, for a directory
            // customer, as New Bill reviewed them.
            if !sale.draft.crate_rows().is_empty()
                && (sale
                    .draft
                    .crate_rows()
                    .iter()
                    .any(|row| row.name().is_none() || row.rate().is_none())
                    || !matches!(
                        sale.draft.customer(),
                        alm_core::DraftCustomer::Selected { .. }
                    ))
            {
                return Err(rejected("crate row"));
            }
            let priced = priced_draft(fixture, role, &sale.draft).map_err(rejected)?;
            let current: Vec<_> = priced.lines.iter().map(|line| line.unit_price).collect();
            if current != sale.prices {
                return Err(AuthorityError::StaleReview);
            }
            // Atomic in the core: refused, it leaves no effect (see `commit`'s in-place path).
            let receipt = post_saved_sale(
                fixture,
                directory,
                role,
                &sale.draft,
                SaleTerms {
                    date: &sale.date.to_string(),
                    paid: &sale.paid.to_rupee_text(),
                },
            )
            .map_err(rejected)?;
            Ok(Outcome::Sale(SaleReceipt {
                draft: id.value(),
                invoice: receipt.invoice,
                quantity: receipt.quantity,
                total: receipt.total,
            }))
        }
        Intent::SellCrate(c) => {
            let template = fixture
                .crate_template_id(role, c.template)
                .map_err(rejected)?;
            let current = fixture.crate_template(role, template).map_err(rejected)?;
            let components: Vec<_> = current
                .components
                .iter()
                .map(|x| {
                    (
                        x.product.value(),
                        x.quantity.get(),
                        x.unit == alm_core::SaleUnit::Piece,
                    )
                })
                .collect();
            if current.price != c.rate || components != c.components {
                return Err(AuthorityError::StaleReview);
            }
            let customer = alm_core::DraftCustomer::Selected {
                id: alm_core::DraftCustomerId::new(c.customer.0.clone()),
                name: c.customer.1.clone(),
            };
            let receipt = sell_crate(
                fixture,
                directory,
                role,
                &CrateSaleTerms {
                    draft: DraftId::new(c.draft),
                    template,
                    quantity: std::num::NonZeroU32::new(c.quantity)
                        .ok_or(AuthorityError::Invalid)?,
                    customer: &customer,
                    date: c.date,
                    paid: c.paid,
                },
            )
            .map_err(|e| match e {
                alm_core::confirmation::owner_billing::OwnerBillingError::Confirmation(
                    alm_core::confirmation::ConfirmationError::DraftAlreadyPosted,
                ) => AuthorityError::AlreadyPosted,
                other => rejected(other),
            })?;
            Ok(Outcome::Sale(SaleReceipt {
                draft: c.draft,
                invoice: receipt.invoice,
                quantity: receipt.quantity,
                total: receipt.total,
            }))
        }
        Intent::CancelSale { draft } => fixture
            .cancel_sale(role, DraftId::new(*draft))
            .map(|()| Outcome::Done)
            .map_err(rejected),
        Intent::RestoreSale { draft } => fixture
            .undo_cancel_sale(role, DraftId::new(*draft))
            .map(|()| Outcome::Done)
            .map_err(rejected),
        Intent::EditSale { draft, lines } => {
            let edits = lines
                .iter()
                .map(|(quantity, rate)| {
                    Some(LineEdit {
                        quantity: std::num::NonZeroU32::new(*quantity)?,
                        price: alm_core::Money::from_minor_units(*rate),
                    })
                })
                .collect::<Option<Vec<_>>>()
                .ok_or(AuthorityError::Invalid)?;
            let id = DraftId::new(*draft);
            let receipt = fixture
                .edit_posted_lines(role, id, &edits)
                .map_err(rejected)?;
            Ok(Outcome::Sale(SaleReceipt {
                draft: *draft,
                invoice: receipt.invoice,
                quantity: receipt.quantity,
                total: receipt.total,
            }))
        }
        Intent::RecordReceipt {
            customer,
            date,
            amount,
            online,
        } => save_receipt_at(
            fixture,
            directory,
            role,
            customer,
            ReceiptTerms {
                date: &date.to_string(),
                amount: &amount.to_rupee_text(),
                mode: if *online {
                    ReceiptMode::Online
                } else {
                    ReceiptMode::Cash
                },
            },
            stamp,
        )
        .map(|()| Outcome::Done)
        .map_err(rejected),
        Intent::SaveCashDay { id, date, amounts } => {
            let mut draft = match id {
                Some(id) => state.cashbook.find(role, *id).map_err(rejected)?.draft(),
                None => ManualCashBookDraft::new(None, *amounts),
            };
            draft.date = Some(*date);
            draft.amounts = *amounts;
            state.cashbook.save(role, &draft).map_err(rejected)?;
            Ok(Outcome::Done)
        }
        Intent::DeleteCashDay { id } => {
            let row = state.cashbook.find(role, *id).map_err(rejected)?;
            state
                .cashbook
                .delete(role, CashBookDeletion::Confirm(row.id))
                .map_err(rejected)?;
            Ok(Outcome::Done)
        }
        other => commands::apply(state, other),
    }
}
