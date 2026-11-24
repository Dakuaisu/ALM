use std::collections::HashMap;

use alm_core::confirmation::{GstBillDraft, GstBillError, GstBillId, GstBillNumber, GstLineFields};
use tauri::State;

use crate::{AppState, Trial, debug, ipc::IpcId, with};

fn warn(error: GstBillError) -> String {
    error.warning().map_or_else(|| debug(error), str::to_owned)
}

fn take(terms: &mut HashMap<String, String>, key: &str) -> String {
    terms.remove(key).unwrap_or_default()
}

impl Trial {
    /// GST Sales "Pick customer" options: saved customers by name, filling `pick`.
    pub(crate) fn gst_customer_options(&self, q: &str) -> Result<String, String> {
        self.require_owner()?;
        let found = self.directory.search_customers(&self.posted, q);
        let fills: Vec<_> = found
            .iter()
            .map(|c| [("pick", c.id), ("pick_q", c.name)])
            .collect();
        let options: Vec<_> = found
            .iter()
            .zip(&fills)
            .map(|(c, fill)| crate::views::SearchOption {
                fill,
                label: c.name,
                sub: c.mobile,
                submit: Some("pick"),
            })
            .collect();
        Ok(crate::views::search_options(&options))
    }

    /// Copies the submitted text into the open form; core reads it on save.
    fn gst_apply(&mut self, mut terms: HashMap<String, String>) -> Result<String, String> {
        let form = self.gst_form.as_mut().ok_or("No GST invoice is open.")?;
        for (field, key) in [
            (&mut form.customer, "customer"),
            (&mut form.gstin, "gstin"),
            (&mut form.cust_addr, "cust_addr"),
            (&mut form.cust_phone, "cust_phone"),
            (&mut form.cust_email, "cust_email"),
            (&mut form.cust_state, "cust_state"),
            (&mut form.cust_state_code, "cust_state_code"),
            (&mut form.pos_state, "pos_state"),
            (&mut form.pos_code, "pos_code"),
            (&mut form.date, "date"),
            (&mut form.pay_mode, "pay_mode"),
            (&mut form.reverse_charge, "reverse_charge"),
            (&mut form.eway_number, "eway_number"),
            (&mut form.transporter, "transporter"),
            (&mut form.vehicle_no, "vehicle_no"),
            (&mut form.lr_no, "lr_no"),
            (&mut form.terms_delivery, "terms_delivery"),
        ] {
            *field = take(&mut terms, key);
        }
        for (i, line) in form.lines.iter_mut().enumerate() {
            let item = take(&mut terms, &format!("item_{i}"));
            if item != line.item {
                line.product = None;
            }
            line.item = item;
            line.hsn = take(&mut terms, &format!("hsn_{i}"));
            line.unit = take(&mut terms, &format!("unit_{i}"));
            line.quantity = take(&mut terms, &format!("qty_{i}"));
            line.rate = take(&mut terms, &format!("rate_{i}"));
            line.gst = take(&mut terms, &format!("gst_{i}"));
            line.packet_qty = take(&mut terms, &format!("pack_{i}"));
        }
        Ok(take(&mut terms, "intent"))
    }

    pub(crate) fn gst_submit(
        &mut self,
        terms: HashMap<String, String>,
        now: u64,
    ) -> Result<(), String> {
        let role = self.require_owner()?;
        let pick = terms.get("pick").cloned().unwrap_or_default();
        let intent = self.gst_apply(terms)?;
        let form = self.gst_form.as_mut().ok_or("No GST invoice is open.")?;
        match intent.as_str() {
            "add" => form.lines.push(GstLineFields::blank()),
            "pick" => self
                .directory
                .fill_gst_customer(&self.posted, form, &pick)
                .map_err(|_| "Select a saved customer.".to_owned())?,
            "fill" => self.posted.fill_gst_lines(role, form).map_err(warn)?,
            "save" => {
                let lines = form
                    .lines
                    .iter()
                    .map(|line| self.posted.gst_line_input(role, line))
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(warn)?;
                let draft = self
                    .posted
                    .gst_bill_draft(role, form, &lines, now)
                    .map_err(warn)?;
                let id = draft.id;
                let saved = self
                    .posted
                    .gst_bill_ledger(role)
                    .map_err(warn)?
                    .bills
                    .iter()
                    .find(|bill| bill.id == id)
                    .map(|bill| bill.number);
                let number = match saved {
                    Some(number) => number,
                    // The authority numbers a new bill, as the hosted function will.
                    None => GstBillNumber(crate::dev_authority::allocate_gst(
                        &mut self.rest,
                        &mut self.authority,
                        id.0,
                    )?),
                };
                let draft = GstBillDraft { number, ..draft };
                let record = self
                    .posted
                    .gst_bill_record(role, &draft)
                    .map_err(warn)?
                    .to_record_text()
                    .map_err(debug)?;
                self.send(&alm_sync::Intent::SaveGstBill { bill: record })?;
                let mut text = String::new();
                self.posted
                    .emit_gst_bill(role, id, &mut |invoice| text = invoice.invoice_number())
                    .map_err(warn)?;
                self.message = format!("GST invoice {text} saved ✓");
                self.gst_form = None;
            }
            remove => {
                let index: usize = remove
                    .strip_prefix("remove:")
                    .and_then(|i| i.parse().ok())
                    .ok_or("Unknown action.")?;
                if index < form.lines.len() {
                    form.lines.remove(index);
                }
                if form.lines.is_empty() {
                    form.lines.push(GstLineFields::blank());
                }
            }
        }
        Ok(())
    }
}

impl Trial {
    pub(crate) fn gst_cancel_bill(&mut self, id: GstBillId) -> Result<(), String> {
        self.require_owner()?;
        self.send(&alm_sync::Intent::CancelGstBill { id: id.0 })
            .map(drop)
    }

    pub(crate) fn gst_delete_bill(&mut self, id: GstBillId) -> Result<(), String> {
        self.require_owner()?;
        self.send(&alm_sync::Intent::DeleteGstBill { id: id.0 })
            .map(drop)
    }
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| {
            u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
        })
}

#[tauri::command(async)]
pub fn gst_new(state: State<'_, AppState>) -> Result<String, String> {
    with(&state, "gstsales", |trial| {
        let role = trial.require_owner()?;
        let today = trial.today()?;
        trial.gst_form = Some(
            trial
                .posted
                .new_gst_bill_fields(role, today)
                .map_err(warn)?,
        );
        Ok(())
    })
}

#[tauri::command(async)]
pub fn gst_edit(state: State<'_, AppState>, id: IpcId) -> Result<String, String> {
    let id = GstBillId(id.value());
    with(&state, "gstsales", |trial| {
        let role = trial.require_owner()?;
        trial.gst_form = Some(trial.posted.edit_gst_bill_fields(role, id).map_err(warn)?);
        Ok(())
    })
}

#[tauri::command(async)]
pub fn gst_submit(
    state: State<'_, AppState>,
    terms: HashMap<String, String>,
) -> Result<String, String> {
    with(&state, "gstsales", |trial| trial.gst_submit(terms, now()))
}

#[tauri::command(async)]
pub fn gst_back(state: State<'_, AppState>) -> Result<String, String> {
    with(&state, "gstsales", |trial| {
        trial.gst_form = None;
        Ok(())
    })
}

#[tauri::command(async)]
pub fn gst_cancel(state: State<'_, AppState>, id: IpcId) -> Result<String, String> {
    let id = GstBillId(id.value());
    with(&state, "gstsales", |trial| trial.gst_cancel_bill(id))
}

#[tauri::command(async)]
pub fn gst_delete(state: State<'_, AppState>, id: IpcId) -> Result<String, String> {
    let id = GstBillId(id.value());
    with(&state, "gstsales", |trial| trial.gst_delete_bill(id))
}

#[tauri::command(async)]
pub fn gst_view(state: State<'_, AppState>, id: IpcId) -> Result<String, String> {
    let id = GstBillId(id.value());
    with(&state, "gst", |trial| {
        let role = trial.require_owner()?;
        trial
            .posted
            .emit_gst_bill(role, id, &mut |_| {})
            .map_err(warn)?;
        trial.gst_view = id;
        Ok(())
    })
}

#[tauri::command(async)]
pub fn gst_customer_search(state: State<'_, AppState>, q: String) -> Result<String, String> {
    let trial = state.lock().map_err(|_| "app state poisoned".to_owned())?;
    trial.gst_customer_options(&q)
}
