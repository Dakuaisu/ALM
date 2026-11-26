use std::collections::HashMap;

use alm_core::ProductId;
use alm_core::confirmation::parse_quote;
use tauri::State;

use crate::{AppState, Trial, refusal, with};

#[derive(serde::Deserialize)]
pub struct NewSupplier {
    name: String,
}

impl Trial {
    /// Saves every supplier column of one product row; inputs are named `s<supplier index>`.
    pub(crate) fn supplier_quotes(
        &mut self,
        mut terms: HashMap<String, String>,
    ) -> Result<(), String> {
        let role = self.require_owner()?;
        let product = terms
            .remove("product")
            .and_then(|id| id.parse().ok())
            .map(ProductId::new)
            .ok_or("Unknown product.")?;
        let suppliers = self
            .posted
            .purchase_suppliers(role)
            .map_err(refusal)?
            .to_vec();
        let mut rates = Vec::new();
        for supplier in suppliers {
            let Some(text) = terms.remove(&format!("s{}", supplier.id.index())) else {
                continue;
            };
            let rate = parse_quote(&text).map_err(|_| {
                format!(
                    "{}: enter exact rupees, or leave blank to clear.",
                    supplier.name
                )
            })?;
            rates.push((u64::try_from(supplier.id.index()).map_err(refusal)?, rate));
        }
        self.send(&alm_sync::Intent::SetSupplierPrices {
            product: product.value(),
            rates,
        })?;
        self.message = "Supplier prices saved ✓".into();
        Ok(())
    }
}

#[tauri::command(async)]
pub fn supplier_add(state: State<'_, AppState>, terms: NewSupplier) -> Result<String, String> {
    with(&state, "suppliers", |trial| {
        trial.require_owner()?;
        trial.send(&alm_sync::Intent::AddSupplier { name: terms.name })?;
        Ok(())
    })
}

#[tauri::command(async)]
pub fn supplier_quotes(
    state: State<'_, AppState>,
    terms: HashMap<String, String>,
) -> Result<String, String> {
    with(&state, "suppliers", |trial| trial.supplier_quotes(terms))
}
