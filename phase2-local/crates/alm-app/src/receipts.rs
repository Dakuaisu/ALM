use alm_core::Role;
use alm_core::confirmation::owner_billing::{
    BillingDirectory, OwnerBillingError, ReceiptTerms, save_receipt,
};
use alm_core::confirmation::{ConfirmationError, ConfirmationFixture, ReceiptMode};
use tauri::State;

use crate::{AppState, debug, with};

#[derive(serde::Deserialize)]
pub enum Mode {
    Cash,
    Online,
}

#[derive(serde::Deserialize)]
pub struct Terms {
    pub customer: String,
    pub date: String,
    pub amount: String,
    pub mode: Mode,
}

pub fn save(
    fixture: &mut ConfirmationFixture,
    directory: &BillingDirectory,
    role: Role,
    terms: Terms,
) -> Result<(), String> {
    let mode = match terms.mode {
        Mode::Cash => ReceiptMode::Cash,
        Mode::Online => ReceiptMode::Online,
    };
    save_receipt(
        fixture,
        directory,
        role,
        &terms.customer,
        ReceiptTerms {
            date: &terms.date,
            amount: &terms.amount,
            mode,
        },
    )
    .map_err(|error| match error {
        OwnerBillingError::Confirmation(
            ConfirmationError::InvalidAmount | ConfirmationError::Overflow,
        ) => "Enter a positive amount with up to two decimal places.".into(),
        OwnerBillingError::Date(error) => debug(error),
        OwnerBillingError::Customer => "Choose a customer.".into(),
        other => debug(other),
    })
}

#[tauri::command(async)]
pub fn receipt_save(state: State<'_, AppState>, terms: Terms) -> Result<String, String> {
    with(&state, "receipt", |trial| {
        let role = trial.require_owner()?;
        save(&mut trial.authority, &trial.directory, role, terms)?;
        trial.message = "Standalone payment recorded in the local simulation.".into();
        Ok(())
    })
}
