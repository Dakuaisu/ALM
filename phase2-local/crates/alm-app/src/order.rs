use alm_core::ProductId;
use alm_core::confirmation::PurchaseSupplier;
use tauri::State;

use crate::{AppState, Trial, ipc::IpcId, refusal, with};

#[derive(serde::Deserialize)]
pub struct Line {
    pub product: String,
    pub qty: String,
}

#[derive(serde::Deserialize)]
pub struct Po {
    pub supplier: String,
    pub date: String,
    pub note: String,
    pub paste: String,
    pub intent: String,
}

fn product(text: &str) -> Result<ProductId, String> {
    text.parse()
        .map(ProductId::new)
        .map_err(|_| "Unknown product.".into())
}

fn quantity(text: &str) -> Result<Option<i64>, String> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(None);
    }
    text.parse()
        .map(Some)
        .map_err(|_| "Quantity must be a whole number.".into())
}

impl Trial {
    fn planner(
        &mut self,
    ) -> Result<&mut alm_core::confirmation::order_planner::OrderPlanner, String> {
        let role = self.require_owner()?;
        if self.order.planner.is_none() {
            self.order.planner = Some(self.posted.open_order_planner(role).map_err(refusal)?);
        }
        self.order
            .planner
            .as_mut()
            .ok_or_else(|| "Planner unavailable.".into())
    }

    pub(crate) fn order_add(&mut self, line: &Line) -> Result<(), String> {
        let (id, qty) = (product(&line.product)?, quantity(&line.qty)?);
        if !self.planner()?.add_item(id, qty) {
            return Err("Item is already in the cart.".into());
        }
        Ok(())
    }

    pub(crate) fn order_checkout(&mut self) -> Result<(), String> {
        let today = self.today()?;
        let role = self.require_owner()?;
        let draft = self.planner()?.checkout(role, today).map_err(refusal)?;
        self.order.draft = Some(draft);
        self.order.warnings.clear();
        Ok(())
    }

    pub(crate) fn order_po(&mut self, po: &Po) -> Result<(), String> {
        let role = self.require_owner()?;
        let mut draft = self
            .order
            .draft
            .clone()
            .ok_or("No purchase order is open.")?;
        let name = po.supplier.trim();
        if draft.supplier.as_ref().is_none_or(|s| s.name != name) {
            draft.supplier = (!name.is_empty()).then(|| PurchaseSupplier {
                id: None,
                name: name.to_owned(),
            });
        }
        draft.date = po.date.parse().ok();
        draft.note.clone_from(&po.note);
        if po.intent == "paste" {
            let warnings = self
                .posted
                .parse_purchase_paste(role, &mut draft, &po.paste)
                .map_err(|error| error.to_string())?;
            self.order.warnings = [
                warnings.unmatched_warning_text(),
                warnings.mixed_party_warning_text(),
            ]
            .into_iter()
            .flatten()
            .collect();
            self.order.draft = Some(draft);
            return Ok(());
        }
        self.send(&alm_sync::Intent::SavePurchaseOrder(
            alm_sync::PurchaseOrderTerms {
                supplier: draft
                    .supplier
                    .as_ref()
                    .map(|s| -> Result<_, String> {
                        Ok((
                            s.id.map(|id| u64::try_from(id.index()))
                                .transpose()
                                .map_err(refusal)?,
                            s.name.clone(),
                        ))
                    })
                    .transpose()?,
                date: draft.date,
                note: draft.note.clone(),
                lines: draft
                    .items
                    .iter()
                    .map(|line| alm_sync::PurchaseLineTerms {
                        item: line.item.clone(),
                        product: line.product_id.map(ProductId::value),
                        ordered: line.ordered.get(),
                        rate: line.rate,
                    })
                    .collect(),
            },
        ))?;
        if let Some(planner) = self.order.planner.as_mut() {
            planner.remove_saved_draft(&draft);
        }
        self.order.draft = None;
        self.order.warnings.clear();
        self.message = "Purchase order saved ✓ (no stock until received)".into();
        Ok(())
    }
}

#[tauri::command(async)]
pub fn order_add(state: State<'_, AppState>, terms: Line) -> Result<String, String> {
    with(&state, "order", |trial| trial.order_add(&terms))
}

#[tauri::command(async)]
pub fn order_qty(state: State<'_, AppState>, terms: Line) -> Result<String, String> {
    with(&state, "order", |trial| {
        let (id, qty) = (product(&terms.product)?, quantity(&terms.qty)?.unwrap_or(0));
        trial.planner()?.set_quantity(id, qty);
        Ok(())
    })
}

#[tauri::command(async)]
pub fn order_remove(state: State<'_, AppState>, id: IpcId) -> Result<String, String> {
    with(&state, "order", |trial| {
        trial.planner()?.remove_item(ProductId::new(id.value()));
        Ok(())
    })
}

#[tauri::command(async)]
pub fn order_low(state: State<'_, AppState>) -> Result<String, String> {
    with(&state, "order", |trial| {
        trial.require_owner()?;
        trial.order.low_only = !trial.order.low_only;
        Ok(())
    })
}

#[tauri::command(async)]
pub fn order_checkout(state: State<'_, AppState>) -> Result<String, String> {
    with(&state, "order", Trial::order_checkout)
}

#[tauri::command(async)]
pub fn order_po(state: State<'_, AppState>, terms: Po) -> Result<String, String> {
    with(&state, "order", |trial| trial.order_po(&terms))
}

#[tauri::command(async)]
pub fn order_discard(state: State<'_, AppState>) -> Result<String, String> {
    with(&state, "order", |trial| {
        trial.order.draft = None;
        trial.order.warnings.clear();
        Ok(())
    })
}
