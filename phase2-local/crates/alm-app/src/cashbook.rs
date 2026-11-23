use alm_core::Money;
use alm_core::confirmation::manual_cashbook::{
    CashBookAmounts, CashBookDeletion, ManualCashBookDraft,
};
use tauri::State;

use crate::{AppState, Trial, debug, ipc::IpcId, with};

const ZERO: Money = Money::from_minor_units(0);

#[derive(serde::Deserialize)]
pub struct Entry {
    pub id: String,
    pub date: String,
    pub cash: String,
    pub paytm: String,
    pub phonepe: String,
    pub debt: String,
    pub salary: String,
    pub goods: String,
    pub misc: String,
    pub intent: String,
}

fn amount(label: &str, text: &str) -> Result<Money, String> {
    if text.trim().is_empty() {
        return Ok(ZERO);
    }
    Money::parse_rupees(text)
        .map_err(|_| format!("{label}: enter a non-negative amount with up to two decimals."))
}

impl Trial {
    pub(crate) fn cash_entry(&mut self, entry: Entry) -> Result<(), String> {
        let role = self.require_owner()?;
        let editing = if entry.id.is_empty() {
            None
        } else {
            Some(
                entry
                    .id
                    .parse::<u64>()
                    .map_err(|_| "Unknown entry.".to_owned())?,
            )
        };
        if entry.intent == "delete" {
            let id = editing.ok_or("Unknown entry.")?;
            let row = self.cashbook.find(role, id).map_err(debug)?;
            self.cashbook
                .delete(role, CashBookDeletion::Confirm(row.id))
                .map_err(debug)?;
            self.cash_form = None;
            return Ok(());
        }
        let date = if entry.date.is_empty() {
            None
        } else {
            Some(entry.date.parse().map_err(|_| "Pick the date".to_owned())?)
        };
        let amounts = CashBookAmounts {
            cash: amount("Cash in Hand", &entry.cash)?,
            paytm: amount("Paytm", &entry.paytm)?,
            phonepe: amount("PhonePe", &entry.phonepe)?,
            debt: amount("Debt / Lent", &entry.debt)?,
            salary: amount("Staff Salary", &entry.salary)?,
            goods: amount("Goods Purchase", &entry.goods)?,
            misc: amount("Misc Expense", &entry.misc)?,
        };
        let mut draft = match editing {
            Some(id) => self.cashbook.find(role, id).map_err(debug)?.draft(),
            None => ManualCashBookDraft::new(None, amounts),
        };
        draft.date = date;
        draft.amounts = amounts;
        match entry.intent.as_str() {
            "system" => {
                draft
                    .use_system_figures(&self.authority, role)
                    .map_err(debug)?;
            }
            "save" => {
                if draft.date.is_none() {
                    return Err("Pick the date".into());
                }
                self.cashbook.save(role, &draft).map_err(debug)?;
                self.message = "Cash book updated ✓".into();
                self.cash_form = None;
                return Ok(());
            }
            _ => {}
        }
        self.cash_form = Some((draft, editing));
        Ok(())
    }
}

#[tauri::command(async)]
pub fn cash_entry(state: State<'_, AppState>, terms: Entry) -> Result<String, String> {
    with(&state, "expenses", |trial| trial.cash_entry(terms))
}

#[tauri::command(async)]
pub fn cash_new(state: State<'_, AppState>) -> Result<String, String> {
    with(&state, "expenses", |trial| {
        trial.require_owner()?;
        let empty = CashBookAmounts {
            cash: ZERO,
            paytm: ZERO,
            phonepe: ZERO,
            goods: ZERO,
            misc: ZERO,
            salary: ZERO,
            debt: ZERO,
        };
        trial.cash_form = Some((ManualCashBookDraft::new(Some(trial.today()?), empty), None));
        Ok(())
    })
}

#[tauri::command(async)]
pub fn cash_edit(state: State<'_, AppState>, id: IpcId) -> Result<String, String> {
    let id = id.value();
    with(&state, "expenses", |trial| {
        let role = trial.require_owner()?;
        let row = trial.cashbook.find(role, id).map_err(debug)?;
        trial.cash_form = Some((row.draft(), Some(id)));
        Ok(())
    })
}

#[tauri::command(async)]
pub fn cash_close(state: State<'_, AppState>) -> Result<String, String> {
    with(&state, "expenses", |trial| {
        trial.cash_form = None;
        Ok(())
    })
}
