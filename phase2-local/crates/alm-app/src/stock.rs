use alm_core::ProductId;
use tauri::State;

use crate::{AppState, debug, ipc::IpcId, with};

#[derive(serde::Deserialize)]
pub struct Filter {
    q: String,
    low: String,
}

#[tauri::command(async)]
pub fn stock_filter(state: State<'_, AppState>, terms: Filter) -> Result<String, String> {
    with(&state, "stock", |trial| {
        trial.require_owner()?;
        trial.stock.query = terms.q;
        if terms.low == "toggle" {
            trial.stock.low_only = !trial.stock.low_only;
        }
        Ok(())
    })
}

#[tauri::command(async)]
pub fn stock_lots(state: State<'_, AppState>, id: IpcId) -> Result<String, String> {
    let id = ProductId::new(id.value());
    with(&state, "stock", |trial| {
        let role = trial.require_owner()?;
        trial.authority.open_stock_lots(role, id).map_err(debug)?;
        trial.stock.lots = Some(id);
        Ok(())
    })
}
