use alm_core::ProductId;
use tauri::State;

use crate::{AppState, ipc::IpcId, refusal, with};

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

#[derive(serde::Deserialize)]
pub struct Count {
    pub product: String,
    pub on_hand: String,
    pub counted: String,
    #[serde(default)]
    pub fridge: String,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub note: String,
}

impl crate::Trial {
    /// The Stock "physical count" form: sent with the on-hand quantity it showed.
    pub(crate) fn stock_count(&mut self, count: &Count) -> Result<(), String> {
        self.require_owner()?;
        let whole = |text: &str| text.trim().parse::<u32>();
        let product = count
            .product
            .parse::<u64>()
            .map_err(|_| "Unknown product.".to_owned())?;
        let on_hand = whole(&count.on_hand).map_err(|_| "Unknown stock.".to_owned())?;
        if count.counted.trim().is_empty() {
            return Err("Enter the counted quantity".into());
        }
        let counted = whole(&count.counted)
            .map_err(|_| "Enter the actual count in whole packets.".to_owned())?;
        let fridge = if count.fridge.trim().is_empty() {
            None
        } else {
            Some(
                whole(&count.fridge)
                    .map_err(|_| "Fridge count must be whole packets.".to_owned())?,
            )
        };
        let reason = (!count.reason.is_empty()).then(|| count.reason.clone());
        let note = (!count.note.trim().is_empty()).then(|| count.note.trim().to_owned());
        let date = self.today()?;
        self.send(&alm_sync::Intent::CountStock {
            product,
            counted,
            fridge,
            date,
            reason,
            note,
            on_hand,
        })?;
        self.stock.adjust = None;
        self.message = "Stock adjusted ✓".into();
        Ok(())
    }
}

#[tauri::command(async)]
pub fn stock_count(state: State<'_, AppState>, terms: Count) -> Result<String, String> {
    with(&state, "stock", |trial| trial.stock_count(&terms))
}

#[tauri::command(async)]
pub fn stock_adjust(state: State<'_, AppState>, id: IpcId) -> Result<String, String> {
    let id = ProductId::new(id.value());
    with(&state, "stock", |trial| {
        let role = trial.require_owner()?;
        trial
            .posted
            .open_physical_count(role, id)
            .map_err(refusal)?;
        trial.stock.adjust = Some(id);
        Ok(())
    })
}

#[tauri::command(async)]
pub fn stock_adjust_close(state: State<'_, AppState>) -> Result<String, String> {
    with(&state, "stock", |trial| {
        trial.stock.adjust = None;
        Ok(())
    })
}

#[tauri::command(async)]
pub fn stock_lots_close(state: State<'_, AppState>) -> Result<String, String> {
    with(&state, "stock", |trial| {
        trial.stock.lots = None;
        Ok(())
    })
}

#[tauri::command(async)]
pub fn stock_lots(state: State<'_, AppState>, id: IpcId) -> Result<String, String> {
    let id = ProductId::new(id.value());
    with(&state, "stock", |trial| {
        let role = trial.require_owner()?;
        trial.posted.open_stock_lots(role, id).map_err(refusal)?;
        trial.stock.lots = Some(id);
        Ok(())
    })
}
