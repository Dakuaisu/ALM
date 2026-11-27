use crate::ipc::State;
use alm_core::Role;
use alm_core::confirmation::owner_billing::{
    BillingDirectory, OwnerBillingError, ReceiptTerms, save_receipt,
};
use alm_core::confirmation::{ConfirmationError, ConfirmationFixture, ReceiptMode};

use crate::{AppState, refusal, with};

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
        OwnerBillingError::Date(error) => refusal(error),
        OwnerBillingError::Customer => "Choose a customer.".into(),
        other => refusal(other),
    })
}

impl crate::Trial {
    /// Validates the receipt on a copy of the authority's state, then sends it as an intent.
    pub(crate) fn receipt_save(&mut self, terms: Terms) -> Result<(), String> {
        let role = self.require(alm_core::permissions::Capability::RecordReceipt)?;
        let mut scratch = self.posted.fixture.clone();
        let intent = alm_sync::Intent::RecordReceipt {
            customer: terms.customer.clone(),
            date: terms.date.parse().map_err(refusal)?,
            amount: alm_core::Money::parse_rupees(&terms.amount)
                .map_err(|_| "Enter a positive amount with up to two decimal places.".to_owned())?,
            online: matches!(terms.mode, Mode::Online),
        };
        save(&mut scratch, &self.directory, role, terms)?;
        self.send(&intent)?;
        self.message = format!("Standalone payment recorded {}.", self.authority_name());
        Ok(())
    }
}

#[tauri::command(async)]
pub fn receipt_save(state: State<'_, AppState>, terms: Terms) -> Result<String, String> {
    with(&state, "receipt", |trial| trial.receipt_save(terms))
}
