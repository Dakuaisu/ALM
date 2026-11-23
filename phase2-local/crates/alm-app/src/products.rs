use alm_core::ProductId;
use alm_core::confirmation::ProductText;
use tauri::State;

use crate::{AppState, Trial, debug, ipc::IpcId, with};

#[derive(serde::Deserialize)]
pub struct Filter {
    q: String,
    brand: String,
}

#[derive(serde::Deserialize)]
pub struct Form {
    pub item: String,
    pub brand: String,
    pub buy: String,
    pub sell: String,
    pub reorder: String,
}

impl Trial {
    pub(crate) fn product_save(&mut self, form: &Form) -> Result<(), String> {
        let role = self.require_owner()?;
        let mut entry = self.products.form.clone().ok_or("No product is open.")?;
        let text = ProductText {
            item: &form.item,
            brand: &form.brand,
            buy: &form.buy,
            sell: &form.sell,
            reorder_level: &form.reorder,
        };
        let failed = |error: alm_core::confirmation::ProductEntryError| {
            error
                .duplicate_warning_text()
                .unwrap_or_else(|| debug(error))
        };
        entry.apply_text(&text).map_err(failed)?;
        self.authority
            .save_product_entry(role, &entry)
            .map_err(failed)?;
        self.products.form = None;
        self.message = format!("{} saved ✓", entry.item);
        Ok(())
    }
}

#[tauri::command(async)]
pub fn products_filter(state: State<'_, AppState>, terms: Filter) -> Result<String, String> {
    with(&state, "products", |trial| {
        trial.require_owner()?;
        trial.products.query = terms.q;
        trial.products.brand = (!terms.brand.is_empty()).then_some(terms.brand);
        Ok(())
    })
}

#[tauri::command(async)]
pub fn product_new(state: State<'_, AppState>) -> Result<String, String> {
    with(&state, "products", |trial| {
        let role = trial.require_owner()?;
        let id = trial.authority.next_product_id(role).map_err(debug)?;
        trial.products.form = Some(
            trial
                .authority
                .begin_product_entry(role, id)
                .map_err(debug)?,
        );
        Ok(())
    })
}

#[tauri::command(async)]
pub fn product_edit(state: State<'_, AppState>, id: IpcId) -> Result<String, String> {
    let id = ProductId::new(id.value());
    with(&state, "products", |trial| {
        let role = trial.require_owner()?;
        trial.products.form = Some(
            trial
                .authority
                .open_product_entry(role, id)
                .map_err(debug)?,
        );
        Ok(())
    })
}

#[tauri::command(async)]
pub fn product_save(state: State<'_, AppState>, terms: Form) -> Result<String, String> {
    with(&state, "products", |trial| trial.product_save(&terms))
}

#[tauri::command(async)]
pub fn product_close(state: State<'_, AppState>) -> Result<String, String> {
    with(&state, "products", |trial| {
        trial.products.form = None;
        Ok(())
    })
}
