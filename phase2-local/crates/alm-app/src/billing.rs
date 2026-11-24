use alm_core::Money;
use alm_core::confirmation::owner_billing::{self, OwnerBillingError, SaleTerms};
use alm_sync::{Intent, Outcome, SaleIntent};
use tauri::State;

use crate::ipc::IpcId;
use crate::{AppState, debug, views, with};

#[derive(serde::Deserialize)]
pub struct Terms {
    pub date: String,
    pub paid: String,
}

fn posting_error(error: OwnerBillingError) -> String {
    match error {
        OwnerBillingError::Customer => {
            "Select a connected customer on New Bill and save the draft.".into()
        }
        OwnerBillingError::NoSavedDraft => "Save or reopen the Manager draft first.".into(),
        OwnerBillingError::NoReview => "Review the sale before confirming.".into(),
        OwnerBillingError::ReviewLimit => "Review limit reached.".into(),
        OwnerBillingError::Confirmation(error) => debug(error),
        OwnerBillingError::Date(error) => debug(error),
    }
}

impl crate::Trial {
    /// Reviews the saved draft on a copy of the authority's state (nothing is registered or
    /// posted) and keeps the reviewed terms as the intent `sale_confirm` sends.
    pub(crate) fn sale_prepare(&mut self, terms: &Terms) -> Result<(), String> {
        let role = self.require_owner()?;
        self.reviewed = None;
        let saved = self
            .current
            .as_ref()
            .and_then(|(draft, saved)| saved.then_some(draft));
        let mut scratch = self.posted.fixture.clone();
        self.posting
            .prepare(
                &mut scratch,
                &self.directory,
                role,
                saved,
                SaleTerms {
                    date: &terms.date,
                    paid: &terms.paid,
                },
            )
            .map_err(posting_error)?;
        let draft = self
            .posting
            .pending(role)
            .ok_or("Review the sale before confirming.")?
            .review()
            .draft()
            .clone();
        let prices = owner_billing::priced_draft(&scratch, role, &draft)
            .map_err(debug)?
            .lines
            .iter()
            .map(|line| line.unit_price)
            .collect();
        self.reviewed = Some(SaleIntent {
            date: draft.bill_date().ok_or("Enter a valid bill date.")?,
            paid: Money::parse_rupees(&terms.paid).map_err(debug)?,
            draft,
            prices,
        });
        Ok(())
    }

    /// Sends the reviewed sale through the outbox; `serial` must be the review on screen.
    pub(crate) fn sale_confirm(&mut self, serial: u64) -> Result<u64, String> {
        let role = self.require_owner()?;
        if self.posting.pending(role).map(|p| p.serial()) != Some(serial) {
            return Err(posting_error(OwnerBillingError::Confirmation(
                alm_core::confirmation::ConfirmationError::StaleReview,
            )));
        }
        let sale = self
            .reviewed
            .clone()
            .ok_or("Review the sale before confirming.")?;
        let outcome = self.send(&Intent::PostSale(sale))?;
        self.posting.invalidate();
        self.reviewed = None;
        match outcome {
            Outcome::Sale(receipt) => Ok(receipt.invoice),
            Outcome::Done | Outcome::GstNumber(_) | Outcome::Created(_) => {
                Err("Unexpected outcome.".into())
            }
        }
    }
}

#[tauri::command(async)]
pub fn sale_prepare(state: State<'_, AppState>, terms: Terms) -> Result<String, String> {
    with(&state, "review", |trial| trial.sale_prepare(&terms))
}

#[tauri::command(async)]
pub fn sale_confirm(state: State<'_, AppState>, id: IpcId) -> Result<String, String> {
    let id = id.value();
    with(&state, "sales", |trial| {
        let invoice = trial.sale_confirm(id)?;
        trial.message = format!("Bill #{invoice} confirmed {}.", trial.authority_name());
        Ok(())
    })
}

#[tauri::command(async)]
pub fn sales_filter(
    state: State<'_, AppState>,
    terms: views::SalesFilter,
) -> Result<String, String> {
    let filter = terms.checked().map_err(debug)?;
    with(&state, "sales", |trial| {
        trial.require_owner()?;
        trial.sales_filter = filter;
        Ok(())
    })
}

#[tauri::command(async)]
pub fn sale_cancel(state: State<'_, AppState>, id: IpcId) -> Result<String, String> {
    let id = id.value();
    with(&state, "sales", |trial| {
        trial.send(&Intent::CancelSale { draft: id }).map(drop)
    })
}

impl crate::Trial {
    pub(crate) fn sale_edit_save(
        &mut self,
        mut terms: std::collections::HashMap<String, String>,
    ) -> Result<(), String> {
        let role = self.require_owner()?;
        let id = terms
            .remove("id")
            .and_then(|text| text.parse().ok())
            .map(alm_core::DraftId::new)
            .ok_or("Unknown bill.")?;
        let lines = self
            .posted
            .current_sale(role, id)
            .map_err(debug)?
            .0
            .draft()
            .lines()
            .len();
        let edits = (0..lines)
            .map(|i| {
                let qty = terms.remove(&format!("qty_{i}")).unwrap_or_default();
                let rate = terms.remove(&format!("rate_{i}")).unwrap_or_default();
                alm_core::confirmation::LineEdit::from_text(&qty, &rate)
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| {
                "Each line needs a whole quantity above zero and a rate in rupees.".to_owned()
            })?;
        let lines = edits
            .iter()
            .map(|edit| (edit.quantity.get(), edit.price.minor_units()))
            .collect();
        let outcome = self
            .send(&Intent::EditSale {
                draft: id.value(),
                lines,
            })
            .map_err(|error| {
                if error.contains("InvalidAmount") {
                    "The new total is below the amount already paid.".to_owned()
                } else {
                    error
                }
            })?;
        self.sale_edit = None;
        if let Outcome::Sale(receipt) = outcome {
            self.message = format!("Bill #{} saved ✓", receipt.invoice);
        }
        Ok(())
    }
}

#[tauri::command(async)]
pub fn sale_edit_open(state: State<'_, AppState>, id: IpcId) -> Result<String, String> {
    let id = alm_core::DraftId::new(id.value());
    with(&state, "sales", |trial| {
        let role = trial.require_owner()?;
        trial.posted.current_sale(role, id).map_err(debug)?;
        trial.sale_edit = Some(id);
        Ok(())
    })
}

#[tauri::command(async)]
pub fn sale_edit_close(state: State<'_, AppState>) -> Result<String, String> {
    with(&state, "sales", |trial| {
        trial.sale_edit = None;
        Ok(())
    })
}

#[tauri::command(async)]
pub fn sale_edit_save(
    state: State<'_, AppState>,
    terms: std::collections::HashMap<String, String>,
) -> Result<String, String> {
    with(&state, "sales", |trial| trial.sale_edit_save(terms))
}

#[tauri::command(async)]
pub fn sale_restore(state: State<'_, AppState>, id: IpcId) -> Result<String, String> {
    let id = id.value();
    with(&state, "sales", |trial| {
        trial.send(&Intent::RestoreSale { draft: id }).map(drop)
    })
}
