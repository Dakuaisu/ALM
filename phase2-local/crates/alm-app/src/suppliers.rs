use std::collections::HashMap;

use alm_core::ProductId;
use alm_core::confirmation::{SupplierPriceEdit, parse_quote};
use tauri::State;

use crate::{AppState, Trial, debug, with};

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
            .authority
            .purchase_suppliers(role)
            .map_err(debug)?
            .to_vec();
        let mut edits = Vec::new();
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
            edits.push(SupplierPriceEdit {
                product,
                supplier: supplier.id,
                rate,
            });
        }
        for edit in edits {
            self.authority
                .edit_comparison_price(role, edit)
                .map_err(debug)?;
        }
        self.message = "Supplier prices saved ✓".into();
        Ok(())
    }
}

#[tauri::command(async)]
pub fn supplier_add(state: State<'_, AppState>, terms: NewSupplier) -> Result<String, String> {
    with(&state, "suppliers", |trial| {
        let role = trial.require_owner()?;
        trial
            .authority
            .add_comparison_supplier(role, &terms.name)
            .map_err(debug)?
            .ok_or("Enter a new supplier name.")?;
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
