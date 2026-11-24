use alm_core::Money;
use alm_core::confirmation::manual_cashbook::{
    CashBookAmounts, ManualCashBookDraft, parse_cash_book_amount,
};
use tauri::State;

use crate::{AppState, Trial, debug, ipc::IpcId, with};

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
    #[serde(default)]
    pub intent: String,
}

fn amount(label: &str, text: &str) -> Result<Money, String> {
    parse_cash_book_amount(text)
        .map_err(|_| format!("{label}: enter a non-negative amount with up to two decimals."))
}

impl Trial {
    /// The day-close card for the typed (unsaved) entry, for live updates.
    pub(crate) fn cash_live(&self, entry: &Entry) -> Result<String, String> {
        let role = self.require_owner()?;
        let (draft, _) = self.cash_draft(role, entry)?;
        crate::views::cash_system_card(&self.authority, role, &draft).map_err(debug)
    }

    fn cash_draft(
        &self,
        role: alm_core::Role,
        entry: &Entry,
    ) -> Result<(ManualCashBookDraft, Option<u64>), String> {
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
            Some(id) => self
                .authority
                .cashbook()
                .find(role, id)
                .map_err(debug)?
                .draft(),
            None => ManualCashBookDraft::new(None, amounts),
        };
        draft.date = date;
        draft.amounts = amounts;
        Ok((draft, editing))
    }

    pub(crate) fn cash_entry(&mut self, entry: Entry) -> Result<(), String> {
        let role = self.require_owner()?;
        if entry.intent == "delete" {
            let id = entry
                .id
                .parse::<u64>()
                .map_err(|_| "Unknown entry.".to_owned())?;
            self.authority.cashbook().find(role, id).map_err(debug)?;
            self.send(&alm_sync::Intent::DeleteCashDay { id })?;
            self.cash_form = None;
            return Ok(());
        }
        let (mut draft, editing) = self.cash_draft(role, &entry)?;
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
                let date = draft.date.ok_or("Pick the date")?;
                self.send(&alm_sync::Intent::SaveCashDay {
                    id: editing,
                    date,
                    amounts: draft.amounts,
                })?;
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

#[cfg(test)]
impl Trial {
    /// Opens a saved row in the form, as the ✎ button does.
    pub(crate) fn cash_edit_for_test(&mut self, id: u64) {
        let row = self
            .authority
            .cashbook()
            .find(alm_core::Role::Owner, id)
            .unwrap();
        self.cash_form = Some((row.draft(), Some(id)));
    }
}

#[tauri::command(async)]
pub fn cash_entry(state: State<'_, AppState>, terms: Entry) -> Result<String, String> {
    with(&state, "expenses", |trial| trial.cash_entry(terms))
}

#[tauri::command(async)]
pub fn cash_live(state: State<'_, AppState>, terms: Entry) -> Result<String, String> {
    let trial = state.lock().map_err(|_| "app state poisoned".to_owned())?;
    trial.cash_live(&terms)
}

#[tauri::command(async)]
pub fn cash_new(state: State<'_, AppState>) -> Result<String, String> {
    with(&state, "expenses", |trial| {
        trial.require_owner()?;
        trial.cash_form = Some((
            ManualCashBookDraft::new(Some(trial.today()?), CashBookAmounts::ZERO),
            None,
        ));
        Ok(())
    })
}

#[tauri::command(async)]
pub fn cash_edit(state: State<'_, AppState>, id: IpcId) -> Result<String, String> {
    let id = id.value();
    with(&state, "expenses", |trial| {
        let role = trial.require_owner()?;
        let row = trial.authority.cashbook().find(role, id).map_err(debug)?;
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
