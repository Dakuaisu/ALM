use std::collections::HashMap;

use alm_core::confirmation::crate_component_from_text;
use alm_core::{Money, ProductId};
use tauri::State;

use crate::views::CrateForm;
use crate::{AppState, Trial, debug, ipc::IpcId, with};

fn blank_row() -> (String, String, String) {
    (String::new(), "1".into(), "Case".into())
}

impl Trial {
    pub(crate) fn crate_submit(
        &mut self,
        mut terms: HashMap<String, String>,
    ) -> Result<(), String> {
        let role = self.require_owner()?;
        let mut form = self.crate_form.take().ok_or("No crate is open.")?;
        form.name = terms.remove("name").unwrap_or_default();
        form.price = terms.remove("price").unwrap_or_default();
        for (i, row) in form.rows.iter_mut().enumerate() {
            row.0 = terms.remove(&format!("product_{i}")).unwrap_or_default();
            row.1 = terms.remove(&format!("qty_{i}")).unwrap_or_default();
            row.2 = terms.remove(&format!("unit_{i}")).unwrap_or_default();
        }
        if terms.remove("intent").as_deref() != Some("save") {
            form.rows.push(blank_row());
            self.crate_form = Some(form);
            return Ok(());
        }
        let saved = self.save_crate(role, &form);
        if saved.is_err() {
            self.crate_form = Some(form);
        }
        saved
    }

    fn save_crate(&mut self, role: alm_core::Role, form: &CrateForm) -> Result<(), String> {
        let components = form
            .rows
            .iter()
            .filter(|(product, _, _)| !product.is_empty())
            .map(|(product, qty, unit)| {
                let id = product
                    .parse()
                    .map(ProductId::new)
                    .map_err(|_| "Unknown item.".to_owned())?;
                crate_component_from_text(id, qty, unit)
                    .map_err(|_| "Each item needs a whole quantity above zero.".to_owned())
            })
            .collect::<Result<Vec<_>, _>>()?;
        let price = Money::parse_rupees(&form.price)
            .map_err(|_| "Enter the crate price in rupees.".to_owned())?;
        if let Some(value) = form.id {
            self.posted.crate_template_id(role, value).map_err(debug)?;
        }
        self.send(&alm_sync::Intent::SaveCrate(alm_sync::CrateTerms {
            id: form.id,
            name: form.name.trim().to_owned(),
            price,
            components: components
                .iter()
                .map(|c| {
                    (
                        c.product.value(),
                        c.quantity.get(),
                        c.unit == alm_core::SaleUnit::Piece,
                    )
                })
                .collect(),
        }))?;
        self.message = "Crate template saved ✓".into();
        Ok(())
    }
}

impl Trial {
    /// Sells crates of a saved template to a directory customer: the bill is saved as a draft
    /// holding the template's crate row (reserving its number), then sent to the authority.
    pub(crate) fn crate_sell(&mut self, terms: &HashMap<String, String>) -> Result<(), String> {
        let role = self.require_owner()?;
        let field = |key: &str| terms.get(key).map_or("", |v| v.trim());
        let template = self
            .posted
            .crate_template_id(role, field("id").parse().map_err(|_| "Unknown crate.")?)
            .map_err(debug)?;
        let current = self
            .posted
            .crate_template(role, template)
            .map_err(debug)?
            .clone();
        let crates: u32 = field("qty")
            .parse()
            .ok()
            .filter(|q| *q > 0)
            .ok_or("Enter how many crates.")?;
        let customer = self
            .directory
            .customers
            .iter()
            .find(|c| c.id == field("customer"))
            .ok_or("Choose a customer.")?;
        let date: alm_core::BillDate = field("date").parse().map_err(|_| "Enter the bill date.")?;
        let paid = Money::parse_rupees(if field("paid").is_empty() {
            "0"
        } else {
            field("paid")
        })
        .map_err(|_| "Enter the amount paid in rupees.")?;
        let id = self
            .saved_ids()?
            .last()
            .copied()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or("Draft limit reached.")?;
        let selected = alm_core::DraftCustomer::Selected {
            id: alm_core::DraftCustomerId::new(customer.id.into()),
            name: customer.name.into(),
        };
        let mut draft = alm_core::ProvisionalDraft::new(alm_core::DraftId::new(id), Vec::new())
            .with_customer(selected)
            .with_bill_date(Some(date));
        self.posted
            .load_crate_template(
                role,
                template,
                std::num::NonZeroU32::new(crates).ok_or("Enter how many crates.")?,
                &mut draft,
            )
            .map_err(debug)?;
        self.drafts.save_draft(draft).map_err(debug)?;
        let outcome = self.send(&alm_sync::Intent::SellCrate(alm_sync::CrateSale {
            draft: id,
            template: template.value(),
            quantity: crates,
            customer: (customer.id.into(), customer.name.into()),
            date,
            paid,
            rate: current.price,
            components: current
                .components
                .iter()
                .map(|c| {
                    (
                        c.product.value(),
                        c.quantity.get(),
                        c.unit == alm_core::SaleUnit::Piece,
                    )
                })
                .collect(),
        }))?;
        if let alm_sync::Outcome::Sale(receipt) = outcome {
            self.message = format!(
                "Crate bill #{} posted: {} packets ✓",
                receipt.invoice, receipt.quantity
            );
        }
        Ok(())
    }
}

#[tauri::command(async)]
pub fn crate_sell(
    state: State<'_, AppState>,
    terms: HashMap<String, String>,
) -> Result<String, String> {
    with(&state, "crates", |trial| trial.crate_sell(&terms))
}

#[tauri::command(async)]
pub fn crate_new(state: State<'_, AppState>) -> Result<String, String> {
    with(&state, "crates", |trial| {
        trial.require_owner()?;
        trial.crate_form = Some(CrateForm {
            rows: vec![blank_row()],
            ..CrateForm::default()
        });
        Ok(())
    })
}

#[tauri::command(async)]
pub fn crate_edit(state: State<'_, AppState>, id: IpcId) -> Result<String, String> {
    with(&state, "crates", |trial| {
        let role = trial.require_owner()?;
        let id = trial
            .posted
            .crate_template_id(role, id.value())
            .map_err(debug)?;
        let template = trial.posted.crate_template(role, id).map_err(debug)?;
        trial.crate_form = Some(CrateForm {
            id: Some(id.value()),
            name: template.name.clone(),
            price: template.price.to_rupee_text(),
            rows: template
                .components
                .iter()
                .map(|c| {
                    let unit = match c.unit {
                        alm_core::SaleUnit::Case => "Case",
                        alm_core::SaleUnit::Piece => "Piece",
                    };
                    (
                        c.product.to_string(),
                        c.quantity.to_string(),
                        unit.to_owned(),
                    )
                })
                .collect(),
        });
        Ok(())
    })
}

#[tauri::command(async)]
pub fn crate_submit(
    state: State<'_, AppState>,
    terms: HashMap<String, String>,
) -> Result<String, String> {
    with(&state, "crates", |trial| trial.crate_submit(terms))
}

#[tauri::command(async)]
pub fn crate_close(state: State<'_, AppState>) -> Result<String, String> {
    with(&state, "crates", |trial| {
        trial.crate_form = None;
        Ok(())
    })
}

#[tauri::command(async)]
pub fn crate_delete(state: State<'_, AppState>, id: IpcId) -> Result<String, String> {
    with(&state, "crates", |trial| {
        let role = trial.require_owner()?;
        let id = trial
            .posted
            .crate_template_id(role, id.value())
            .map_err(debug)?;
        trial
            .send(&alm_sync::Intent::DeleteCrate { id: id.value() })
            .map(drop)
    })
}
