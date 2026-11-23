use alm_core::confirmation::OpeningBalance;
use alm_core::{DraftCustomerId, Money};
use tauri::State;

use crate::{AppState, debug, with};

#[derive(serde::Deserialize)]
pub struct Filter {
    q: String,
    mode: String,
}

#[derive(serde::Deserialize)]
pub struct Opening {
    pub customer: String,
    pub amount: String,
    pub date: String,
}

#[tauri::command(async)]
pub fn customers_filter(state: State<'_, AppState>, terms: Filter) -> Result<String, String> {
    with(&state, "customers", |trial| {
        trial.require_owner()?;
        let view = &mut trial.customers;
        match terms.mode.as_str() {
            "due" => (view.due_only, view.query) = (true, String::new()),
            "all" => (view.due_only, view.query) = (false, String::new()),
            _ => (view.due_only, view.query) = (false, terms.q),
        }
        Ok(())
    })
}

#[tauri::command(async)]
pub fn customer_open(state: State<'_, AppState>, id: String) -> Result<String, String> {
    with(&state, "customers", |trial| {
        trial.require_owner()?;
        trial
            .directory
            .customer(&trial.authority, &id)
            .map_err(debug)?;
        trial.customers.open = Some(id);
        Ok(())
    })
}

pub fn save_opening(trial: &mut crate::Trial, terms: Opening) -> Result<(), String> {
    let role = trial.require_owner()?;
    let customer = trial
        .directory
        .customer(&trial.authority, &terms.customer)
        .map_err(debug)?;
    let amount =
        if terms.amount.trim().is_empty() {
            None
        } else {
            Some(Money::parse_rupees(&terms.amount).map_err(|_| {
                "Enter a non-negative amount with up to two decimal places.".to_owned()
            })?)
        };
    let date = if terms.date.is_empty() {
        None
    } else {
        Some(terms.date.parse().map_err(debug)?)
    };
    trial
        .authority
        .save_customer_opening(
            role,
            &DraftCustomerId::new(customer.id.into()),
            OpeningBalance { amount, date },
        )
        .map_err(debug)?;
    trial.customers.open = Some(customer.id.into());
    trial.message = format!(
        "Opening balance saved for {} (local simulation).",
        customer.name
    );
    Ok(())
}

#[tauri::command(async)]
pub fn customer_opening(state: State<'_, AppState>, terms: Opening) -> Result<String, String> {
    with(&state, "customers", |trial| save_opening(trial, terms))
}
