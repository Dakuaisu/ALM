use alm_core::ProductId;
use alm_core::confirmation::{PriceField, ProductText};
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
    #[serde(default)]
    pub margin: String,
    pub sell: String,
    pub reorder: String,
    /// The price box the user typed in, sent by live forms (`_changed`).
    #[serde(default, rename = "_changed")]
    pub changed: String,
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
        let (id, new) = entry.target();
        let reviewed = if new {
            None
        } else {
            let pricing = self
                .posted
                .open_product_entry(role, id)
                .map_err(failed)?
                .pricing();
            Some((pricing.buy(), pricing.sell()))
        };
        let pricing = entry.pricing();
        self.send(&alm_sync::Intent::SaveProduct(alm_sync::ProductTerms {
            id: id.value(),
            new,
            item: entry.item.clone(),
            brand: entry.brand.clone(),
            buy: pricing.buy(),
            sell: pricing.sell(),
            reorder: entry.reorder_level,
            reviewed,
        }))?;
        self.products.form = None;
        self.message = format!("{} saved ✓", entry.item);
        Ok(())
    }
}

impl Trial {
    /// Live "buy keeps margin" preview: core recalculates; only the form card is returned.
    pub(crate) fn product_preview(&mut self, form: &Form) -> Result<String, String> {
        self.require_owner()?;
        let entry = self.products.form.as_mut().ok_or("No product is open.")?;
        let changed = match form.changed.as_str() {
            "buy" => PriceField::Buy,
            "margin" => PriceField::Margin,
            "sell" => PriceField::Sell,
            _ => return Ok(crate::views::product_form(entry)),
        };
        entry.item.clone_from(&form.item);
        entry.brand.clone_from(&form.brand);
        entry
            .preview_prices(&form.buy, &form.margin, &form.sell, changed)
            .map_err(|_| "Enter prices in rupees (up to two decimals).".to_owned())?;
        Ok(crate::views::product_form(entry))
    }
}

#[tauri::command(async)]
pub fn product_preview(state: State<'_, AppState>, terms: Form) -> Result<String, String> {
    let mut trial = state.lock().map_err(|_| "app state poisoned".to_owned())?;
    trial.product_preview(&terms)
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
        let id = trial.posted.next_product_id(role).map_err(debug)?;
        trial.products.form = Some(trial.posted.begin_product_entry(role, id).map_err(debug)?);
        Ok(())
    })
}

#[tauri::command(async)]
pub fn product_edit(state: State<'_, AppState>, id: IpcId) -> Result<String, String> {
    let id = ProductId::new(id.value());
    with(&state, "products", |trial| {
        let role = trial.require_owner()?;
        trial.products.form = Some(trial.posted.open_product_entry(role, id).map_err(debug)?);
        Ok(())
    })
}

#[tauri::command(async)]
pub fn product_save(state: State<'_, AppState>, terms: Form) -> Result<String, String> {
    with(&state, "products", |trial| trial.product_save(&terms))
}

#[tauri::command(async)]
pub fn product_delete(state: State<'_, AppState>, q: String) -> Result<String, String> {
    let mut trial = state.lock().map_err(|_| "app state poisoned".to_owned())?;
    let html = trial.product_delete(&q)?;
    match html {
        Some(html) => Ok(html),
        None => trial.render("products"),
    }
}

impl Trial {
    /// "ask" shows the confirmation; "confirm" deletes through the authority.
    pub(crate) fn product_delete(&mut self, step: &str) -> Result<Option<String>, String> {
        self.require_owner()?;
        let entry = self.products.form.as_ref().ok_or("No product is open.")?;
        let (id, new) = entry.target();
        if new {
            return Err("This product is not saved yet.".into());
        }
        if step != "confirm" {
            let html = self.render("products")?;
            return Ok(Some(html.replacen(
                "data-delete-ask hidden",
                "data-delete-ask",
                1,
            )));
        }
        let item = entry.item.clone();
        self.send(&alm_sync::Intent::DeleteProduct { id: id.value() })?;
        self.products.form = None;
        self.message = format!("{item} deleted ✓");
        Ok(None)
    }
}

#[tauri::command(async)]
pub fn product_close(state: State<'_, AppState>) -> Result<String, String> {
    with(&state, "products", |trial| {
        trial.products.form = None;
        Ok(())
    })
}
