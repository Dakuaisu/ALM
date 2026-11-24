//! New Bill's crate row (ALM_Business_Manager.html L3640 "Add Crate", L3755 `crateRow`): the
//! Owner adds a saved crate template as the bill's line, changes its crates or removes it, and
//! posts the saved bill through the authority's crate-sale command. A crate bill holds its crate
//! only (the core's sole-crate bill).
use std::num::NonZeroU32;

use alm_core::{DraftCustomer, DraftId, Money, ProvisionalDraft};
use alm_sync::{CrateSale, Intent, Outcome};
use tauri::State;

use super::quantity;
use crate::{AppState, Trial, debug, ipc::IpcId, with};

const SOLE: &str = "A crate bill holds its crate only: remove the items first.";
pub(super) const SOLE_ITEMS: &str = "A crate bill holds its crate only: remove the crate first.";

#[derive(serde::Deserialize)]
pub struct CrateLine {
    template: IpcId,
    quantity: String,
}

#[derive(serde::Deserialize)]
pub struct CrateQuantity {
    quantity: String,
}

#[derive(serde::Deserialize)]
pub struct CratePost {
    #[serde(default)]
    paid: String,
}

impl Trial {
    /// The saved crate templates New Bill offers (Owner only; OPEN-004: a Manager sells none).
    pub(super) fn bill_crates(&self) -> Vec<(u64, String)> {
        if self.role() != alm_core::Role::Owner {
            return Vec::new();
        }
        self.posted
            .crate_template_list(alm_core::Role::Owner)
            .map(|rows| {
                rows.into_iter()
                    .map(|row| (row.id.value(), row.template.name))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub(crate) fn bill_crate_add(
        &mut self,
        template: u64,
        crates: NonZeroU32,
    ) -> Result<(), String> {
        let role = self.require_owner()?;
        let id = self
            .posted
            .crate_template_id(role, template)
            .map_err(|_| "Choose a saved crate.".to_owned())?;
        let mut loaded = ProvisionalDraft::new(DraftId::new(0), Vec::new());
        self.posted
            .load_crate_template(role, id, crates, &mut loaded)
            .map_err(debug)?;
        let row = loaded.remove_crate_row().ok_or("Choose a saved crate.")?;
        self.edit_bill(|draft| {
            if !draft.lines().is_empty() {
                return Err(SOLE.into());
            }
            draft.remove_crate_row();
            draft.add_crate_row(row).map_err(|_| SOLE.to_owned())?;
            Ok(true)
        })
    }

    pub(crate) fn bill_crate_quantity(&mut self, crates: NonZeroU32) -> Result<(), String> {
        self.require_owner()?;
        self.edit_bill(|draft| {
            let row = draft.crate_row().ok_or("No crate on this bill.")?;
            if row.quantity() == crates {
                return Ok(false);
            }
            let row = row.with_quantity(crates);
            draft.replace_crate_row(row);
            Ok(true)
        })
    }

    pub(crate) fn bill_crate_remove(&mut self) -> Result<(), String> {
        self.require_owner()?;
        self.edit_bill(|draft| Ok(draft.remove_crate_row().is_some()))
    }

    /// Posts the saved crate bill: the crate row's template, crates, rate and contents as shown,
    /// the bill's customer and date, and the amount paid. The authority rechecks all of them.
    pub(crate) fn bill_crate_post(&mut self, paid: &str) -> Result<(), String> {
        let role = self.require_owner()?;
        let (draft, saved) = self.current.clone().ok_or("No draft is open.")?;
        if !saved {
            return Err("Save the bill as a draft first.".into());
        }
        let row = draft.crate_row().ok_or("No crate on this bill.")?;
        let template = self.posted.crate_template_of(role, row).map_err(|_| {
            "The crate changed since it was added to this bill: add it again.".to_owned()
        })?;
        let current = self
            .posted
            .crate_template(role, template)
            .map_err(debug)?
            .clone();
        let DraftCustomer::Selected { id, name } = draft.customer() else {
            return Err("Select a connected customer.".into());
        };
        let date = draft.bill_date().ok_or("Enter a valid bill date.")?;
        let paid = Money::parse_rupees(if paid.trim().is_empty() {
            "0"
        } else {
            paid.trim()
        })
        .map_err(|_| "Enter the amount paid in rupees.")?;
        let outcome = self.send(&Intent::SellCrate(CrateSale {
            draft: draft.id().value(),
            template: template.value(),
            quantity: row.quantity().get(),
            customer: (id.value().to_owned(), name.clone()),
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
        if let Outcome::Sale(receipt) = outcome {
            self.message = format!(
                "Crate bill #{} posted: {} packets ✓",
                receipt.invoice, receipt.quantity
            );
        }
        self.current = None;
        Ok(())
    }
}

#[tauri::command(async)]
pub fn bill_crate_add(state: State<'_, AppState>, line: CrateLine) -> Result<String, String> {
    with(&state, "bill", |trial| {
        trial.bill_crate_add(line.template.value(), quantity(&line.quantity)?)
    })
}

#[tauri::command(async)]
pub fn bill_crate_quantity(
    state: State<'_, AppState>,
    terms: CrateQuantity,
) -> Result<String, String> {
    with(&state, "bill", |trial| {
        trial.bill_crate_quantity(quantity(&terms.quantity)?)
    })
}

#[tauri::command(async)]
pub fn bill_crate_remove(state: State<'_, AppState>) -> Result<String, String> {
    with(&state, "bill", Trial::bill_crate_remove)
}

#[tauri::command(async)]
pub fn bill_crate_post(state: State<'_, AppState>, terms: CratePost) -> Result<String, String> {
    with(&state, "bill", |trial| trial.bill_crate_post(&terms.paid))
}

/// Sells `crates` of `template` to `customer` from a fresh New Bill: customer and date, the
/// crate row, save, post.
#[cfg(test)]
pub(crate) fn sell_on_new_bill(
    trial: &mut Trial,
    template: u64,
    customer: &str,
    crates: u32,
    date: &str,
    paid: &str,
) -> Result<(), String> {
    use super::{Header, HeaderIntent};
    trial.current = None;
    trial.render("bill")?;
    trial.bill_header(Header::new(customer, date, None, HeaderIntent::Select))?;
    trial.bill_crate_add(template, NonZeroU32::new(crates).ok_or("crates")?)?;
    trial.bill_header(Header::new(customer, date, None, HeaderIntent::Save))?;
    trial.bill_crate_post(paid)
}
