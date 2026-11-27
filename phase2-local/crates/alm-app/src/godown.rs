use crate::ipc::State;

use crate::{AppState, Trial, with};

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
    trial.require(alm_core::permissions::Capability::SaveGodown)?;
    trial.send(&alm_sync::Intent::SaveGodown {
        item: row.item.clone(),
        cases: row.cases.clone(),
        note: row.note.clone(),
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
        trial.require(alm_core::permissions::Capability::Godown)?;
        trial.godown_query = terms.q;
        Ok(())
    })
}
