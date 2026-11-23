use alm_core::confirmation::owner_billing::{OwnerBillingError, SaleTerms};
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

#[tauri::command(async)]
pub fn sale_prepare(state: State<'_, AppState>, terms: Terms) -> Result<String, String> {
    with(&state, "review", |trial| {
        let role = trial.require_owner()?;
        let saved = trial
            .current
            .as_ref()
            .and_then(|(draft, saved)| saved.then_some(draft));
        trial
            .posting
            .prepare(
                &mut trial.authority,
                &trial.directory,
                role,
                saved,
                SaleTerms {
                    date: &terms.date,
                    paid: &terms.paid,
                },
            )
            .map_err(posting_error)
    })
}

#[tauri::command(async)]
pub fn sale_confirm(state: State<'_, AppState>, id: IpcId) -> Result<String, String> {
    let id = id.value();
    with(&state, "sales", |trial| {
        let role = trial.require_owner()?;
        let receipt = trial
            .posting
            .confirm(&mut trial.authority, role, id)
            .map_err(posting_error)?;
        trial.message = format!(
            "Bill #{} confirmed in the local simulation.",
            receipt.invoice
        );
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
        let role = trial.require_owner()?;
        trial
            .authority
            .cancel_sale(role, alm_core::DraftId::new(id))
            .map_err(debug)
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
            .authority
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
        let receipt = self
            .authority
            .edit_posted_lines(role, id, &edits)
            .map_err(|error| match error {
                alm_core::confirmation::ConfirmationError::InvalidAmount => {
                    "The new total is below the amount already paid.".to_owned()
                }
                other => debug(other),
            })?;
        self.sale_edit = None;
        self.message = format!("Bill #{} saved ✓", receipt.invoice);
        Ok(())
    }
}

#[tauri::command(async)]
pub fn sale_edit_open(state: State<'_, AppState>, id: IpcId) -> Result<String, String> {
    let id = alm_core::DraftId::new(id.value());
    with(&state, "sales", |trial| {
        let role = trial.require_owner()?;
        trial.authority.current_sale(role, id).map_err(debug)?;
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
        let role = trial.require_owner()?;
        trial
            .authority
            .undo_cancel_sale(role, alm_core::DraftId::new(id))
            .map_err(debug)
    })
}
