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
