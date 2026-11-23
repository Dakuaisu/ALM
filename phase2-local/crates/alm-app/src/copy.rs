//! Clipboard text and autocomplete options; each command returns a string, not a screen.
use alm_core::BillDate;
use alm_core::confirmation::ConfirmationError;
use tauri::State;

use crate::views::{self, SearchOption};
use crate::{AppState, Trial, debug};

fn lock<'a>(state: &'a State<'_, AppState>) -> Result<std::sync::MutexGuard<'a, Trial>, String> {
    state.lock().map_err(|_| "app state poisoned".to_owned())
}

impl Trial {
    pub(crate) fn reorder_copy_text(&self) -> Result<String, String> {
        let role = self.require_owner()?;
        let mut text = None;
        self.authority
            .copy_reorder_list(role, self.today()?, &mut |copied| text = Some(copied))
            .map_err(debug)?;
        text.ok_or_else(|| "Nothing to copy.".into())
    }

    pub(crate) fn godown_copy_text(&self) -> Result<String, String> {
        let role = self.require_owner()?;
        self.authority
            .manual_godown_copy_text(role, self.today()?)
            .map_err(debug)?
            .ok_or_else(|| "Godown list is empty".into())
    }

    pub(crate) fn rate_copy_text(&mut self) -> Result<String, String> {
        let role = self.require_owner()?;
        let today: BillDate = self.today()?;
        self.authority
            .rate_list_text(role, &mut self.rates, crate::SELLER.name, today)
            .map_err(debug)?
            .map(|(text, _)| text)
            .ok_or_else(|| "Tick at least one item.".into())
    }

    /// Owner item autocomplete for GST lines (`field` is the line's item box) and the Godown.
    pub(crate) fn item_options(
        &self,
        q: &str,
        field: &str,
        submit: Option<&str>,
    ) -> Result<String, String> {
        let role = self.require_owner()?;
        let rows = self.authority.product_suggestions(role, q).map_err(debug)?;
        let fills: Vec<_> = rows
            .iter()
            .map(|row| [(field, row.item.as_str())])
            .collect();
        let options: Vec<_> = rows
            .iter()
            .zip(&fills)
            .map(|(row, fill)| SearchOption {
                fill,
                label: &row.item,
                sub: &row.brand,
                submit,
            })
            .collect();
        Ok(views::search_options(&options))
    }
}

#[tauri::command(async)]
pub fn reorder_copy_text(state: State<'_, AppState>) -> Result<String, String> {
    lock(&state)?.reorder_copy_text()
}

#[tauri::command(async)]
pub fn godown_copy_text(state: State<'_, AppState>) -> Result<String, String> {
    lock(&state)?.godown_copy_text()
}

#[tauri::command(async)]
pub fn rate_copy_text(state: State<'_, AppState>) -> Result<String, String> {
    lock(&state)?.rate_copy_text()
}

#[tauri::command(async)]
pub fn rate_whatsapp_url(state: State<'_, AppState>) -> Result<String, String> {
    let mut trial = lock(&state)?;
    let role = trial.require_owner()?;
    let today = trial.today()?;
    let trial = &mut *trial;
    trial
        .authority
        .rate_list_text(role, &mut trial.rates, crate::SELLER.name, today)
        .map_err(debug)?
        .map(|(_, uri)| uri)
        .ok_or_else(|| "Tick at least one item first".into())
}

#[tauri::command(async)]
pub fn receipt_customer_search(state: State<'_, AppState>, q: String) -> Result<String, String> {
    let trial = lock(&state)?;
    let role = trial.require_owner()?;
    views::receipt_customer_options(&trial.authority, &trial.directory, role, &q)
        .map_err(|error: ConfirmationError| debug(error))
}

#[tauri::command(async)]
pub fn gst_item_search(
    state: State<'_, AppState>,
    q: String,
    field: String,
) -> Result<String, String> {
    if !field.starts_with("item_") {
        return Err("Unknown field.".into());
    }
    lock(&state)?.item_options(&q, &field, Some("fill"))
}

#[tauri::command(async)]
pub fn godown_item_search(state: State<'_, AppState>, q: String) -> Result<String, String> {
    lock(&state)?.item_options(&q, "item", None)
}
