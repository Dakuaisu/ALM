//! New Bill commands.
use std::num::NonZeroUsize;

use tauri::State;

use super::{Header, Line, quantity};
use crate::{
    AppState,
    ipc::{IpcDraft, IpcId},
    with,
};

#[tauri::command(async)]
pub fn bill_live(state: State<'_, AppState>, line: String, qty: String) -> Result<String, String> {
    let mut trial = state.lock().map_err(|_| "app state poisoned".to_owned())?;
    trial.bill_live(&line, &qty)
}

#[tauri::command(async)]
pub fn bill_customer_search(state: State<'_, AppState>, q: String) -> Result<String, String> {
    let trial = state.lock().map_err(|_| "app state poisoned".to_owned())?;
    Ok(trial.bill_customer_search(&q))
}

#[tauri::command(async)]
pub fn bill_item_search(state: State<'_, AppState>, q: String) -> Result<String, String> {
    let trial = state.lock().map_err(|_| "app state poisoned".to_owned())?;
    Ok(trial.bill_item_search(&q))
}

#[tauri::command(async)]
pub fn bill_fresh(state: State<'_, AppState>) -> Result<String, String> {
    with(&state, "bill", |trial| {
        trial.fresh_bill(&trial.saved_ids()?)
    })
}

#[tauri::command(async)]
pub fn bill_header(state: State<'_, AppState>, header: Header) -> Result<String, String> {
    with(&state, "bill", |trial| trial.bill_header(header))
}

#[tauri::command(async)]
pub fn bill_add(state: State<'_, AppState>, line: Line) -> Result<String, String> {
    with(&state, "bill", |trial| {
        let (product, quantity) = line.picked()?;
        trial.bill_add(product, quantity)
    })
}

#[tauri::command(async)]
pub fn bill_quantity(
    state: State<'_, AppState>,
    line: usize,
    qty: String,
) -> Result<String, String> {
    let line = NonZeroUsize::new(line).ok_or("Unknown line.")?;
    let qty = quantity(&qty)?;
    with(&state, "bill", |trial| {
        trial.edit_bill(|draft| {
            let previous = draft
                .replace_line_quantity(line, qty)
                .ok_or("Unknown line.")?;
            Ok(previous != qty)
        })
    })
}

#[tauri::command(async)]
pub fn bill_remove(state: State<'_, AppState>, id: IpcId) -> Result<String, String> {
    let line = usize::try_from(id.value())
        .ok()
        .and_then(NonZeroUsize::new)
        .ok_or("Unknown line.")?;
    with(&state, "bill", |trial| {
        trial.edit_bill(|draft| {
            draft.remove_line(line).ok_or("Unknown line.")?;
            Ok(true)
        })
    })
}

#[tauri::command(async)]
pub fn bill_open(state: State<'_, AppState>, id: IpcDraft) -> Result<String, String> {
    with(&state, "bill", |trial| trial.bill_open(id.value()))
}
