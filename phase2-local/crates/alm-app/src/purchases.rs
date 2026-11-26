use std::collections::HashMap;

use alm_core::confirmation::PurchaseOrderId;
use tauri::State;

use crate::{AppState, Trial, ipc::IpcId, refusal, with};

impl Trial {
    pub(crate) fn purchase_id(&self, index: u64) -> Result<PurchaseOrderId, String> {
        let role = self.require_owner()?;
        self.posted
            .purchase_orders(role)
            .map_err(refusal)?
            .iter()
            .map(|order| order.id)
            .find(|id| u64::try_from(id.index()).is_ok_and(|i| i == index))
            .ok_or_else(|| "Purchase order not found".into())
    }

    pub(crate) fn po_receive_save(
        &mut self,
        mut terms: HashMap<String, String>,
    ) -> Result<(), String> {
        let role = self.require_owner()?;
        let today = self.today()?;
        let index = terms
            .remove("po")
            .and_then(|text| text.parse().ok())
            .ok_or("Purchase order not found")?;
        let id = self.purchase_id(index)?;
        let date = terms.remove("date").and_then(|text| text.parse().ok());
        let form = self
            .posted
            .open_purchase_receive_form(role, id, today)
            .map_err(|error| error.to_string())?;
        let lines = form
            .rows
            .iter()
            .enumerate()
            .map(|(i, row)| {
                let qty = terms.remove(&format!("qty_{i}")).unwrap_or_default();
                let rate = terms.remove(&format!("rate_{i}")).unwrap_or_default();
                row.line_from_text(&qty, &rate)
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;
        self.send(&alm_sync::Intent::ReceivePurchaseOrder {
            po: index,
            date,
            today,
            lines: lines.iter().map(|l| (l.quantity, l.rate)).collect(),
            remaining: form.rows.iter().map(|row| row.remaining).collect(),
        })?;
        self.receiving = None;
        self.message = "Goods received ✓ — stock and lots updated".into();
        Ok(())
    }
}

#[tauri::command(async)]
pub fn po_receive(state: State<'_, AppState>, id: IpcId) -> Result<String, String> {
    with(&state, "purchases", |trial| {
        trial.receiving = Some(trial.purchase_id(id.value())?);
        Ok(())
    })
}

#[tauri::command(async)]
pub fn po_receive_close(state: State<'_, AppState>) -> Result<String, String> {
    with(&state, "purchases", |trial| {
        trial.receiving = None;
        Ok(())
    })
}

#[tauri::command(async)]
pub fn po_receive_save(
    state: State<'_, AppState>,
    terms: HashMap<String, String>,
) -> Result<String, String> {
    with(&state, "purchases", |trial| trial.po_receive_save(terms))
}

#[tauri::command(async)]
pub fn po_close(state: State<'_, AppState>, id: IpcId) -> Result<String, String> {
    with(&state, "purchases", |trial| {
        let role = trial.require_owner()?;
        let po = id.value();
        let id = trial.purchase_id(po)?;
        let remaining = trial
            .posted
            .purchase_orders(role)
            .map_err(refusal)?
            .iter()
            .find(|order| order.id == id)
            .ok_or("Purchase order not found")?
            .items
            .iter()
            .map(alm_core::confirmation::SavedPurchaseLine::remaining)
            .collect();
        trial
            .send(&alm_sync::Intent::ClosePurchaseOrder { po, remaining })
            .map(drop)
    })
}
