use alm_core::confirmation::{ManualGodownError, ManualGodownInput};
use tauri::State;

use crate::{AppState, Trial, debug, with};

#[derive(serde::Deserialize)]
pub struct Row {
    pub item: String,
    pub cases: String,
    pub note: String,
}

#[derive(serde::Deserialize)]
pub struct Filter {
    q: String,
}

pub fn save(trial: &mut Trial, row: &Row) -> Result<(), String> {
    let role = trial.require_owner()?;
    trial
        .authority
        .save_manual_godown(
            role,
            ManualGodownInput {
                item: &row.item,
                cases: &row.cases,
                note: &row.note,
            },
        )
        .map_err(|error| match error {
            ManualGodownError::ItemNameRequired => "Type an item name".to_owned(),
            ManualGodownError::InvalidCases => "Enter quantity (whole cases)".to_owned(),
            other => debug(other),
        })?;
    trial.message = format!("{} saved ✓", row.item.trim());
    Ok(())
}

#[tauri::command(async)]
pub fn godown_save(state: State<'_, AppState>, terms: Row) -> Result<String, String> {
    with(&state, "godown", |trial| save(trial, &terms))
}

#[tauri::command(async)]
pub fn godown_filter(state: State<'_, AppState>, terms: Filter) -> Result<String, String> {
    with(&state, "godown", |trial| {
        trial.require_owner()?;
        trial.godown_query = terms.q;
        Ok(())
    })
}
