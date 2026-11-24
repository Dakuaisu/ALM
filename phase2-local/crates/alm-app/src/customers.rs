use alm_core::confirmation::OpeningBalance;
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
            .customer(&trial.posted, &id)
            .map_err(debug)?;
        trial.customers.open = Some(id);
        Ok(())
    })
}

#[tauri::command(async)]
pub fn customer_close(state: State<'_, AppState>) -> Result<String, String> {
    with(&state, "customers", |trial| {
        trial.customers.open = None;
        Ok(())
    })
}

pub fn save_opening(trial: &mut crate::Trial, terms: Opening) -> Result<(), String> {
    trial.require_owner()?;
    let customer = trial
        .directory
        .customer(&trial.posted, &terms.customer)
        .map_err(debug)?;
    let amount = OpeningBalance::parse_amount(&terms.amount).map_err(|_| {
        "Enter an amount with up to two decimal places (negative for an advance).".to_owned()
    })?;
    let date = if terms.date.is_empty() {
        None
    } else {
        Some(terms.date.parse().map_err(debug)?)
    };
    let (id, name) = (customer.id.to_owned(), customer.name.to_owned());
    trial.send(&alm_sync::Intent::SaveOpening {
        customer: id.clone(),
        amount,
        date,
    })?;
    trial.customers.open = Some(id);
    trial.message = format!("Opening balance saved for {name}.");
    Ok(())
}

#[tauri::command(async)]
pub fn customer_opening(state: State<'_, AppState>, terms: Opening) -> Result<String, String> {
    with(&state, "customers", |trial| save_opening(trial, terms))
}
