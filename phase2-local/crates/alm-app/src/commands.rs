//! Small commands: screen switch, start, role switch, Dashboard period, Rate List, print, share.
use tauri::State;

use crate::{AppState, debug, gst_print, ipc, text, with};

// Commands run off the UI thread (`async`): a SQLite fsync on a slow device disk
// otherwise blocks Android's input thread and triggers an ANR.
#[tauri::command(async)]
pub fn screen(state: State<'_, AppState>, tab: String) -> Result<String, String> {
    with(&state, &tab, |_| Ok(()))
}

#[tauri::command(async)]
pub fn dash_period(state: State<'_, AppState>, period: String) -> Result<String, String> {
    use alm_core::confirmation::DashboardPeriod;
    let period = match period.as_str() {
        "today" => DashboardPeriod::Today,
        "week" => DashboardPeriod::Week,
        "month" => DashboardPeriod::Month,
        _ => return Err("Unknown period.".into()),
    };
    with(&state, "dashboard", |trial| {
        trial.require_owner()?;
        trial.dash_period = period;
        Ok(())
    })
}

#[tauri::command(async)]
pub fn rate_brand(state: State<'_, AppState>, q: String) -> Result<String, String> {
    with(&state, "ratelist", |trial| {
        trial.require_owner()?;
        trial.rates.toggle_brand_filter(&q);
        Ok(())
    })
}

#[tauri::command(async)]
pub fn rate_group(state: State<'_, AppState>, q: String) -> Result<String, String> {
    with(&state, "ratelist", |trial| {
        let role = trial.require_owner()?;
        trial
            .posted
            .rate_list_group_toggle(role, &mut trial.rates, &q)
            .map_err(debug)
    })
}

#[derive(serde::Deserialize)]
pub struct RateSearch {
    q: String,
}

#[tauri::command(async)]
pub fn rate_search(state: State<'_, AppState>, terms: RateSearch) -> Result<String, String> {
    with(&state, "ratelist", |trial| {
        trial.require_owner()?;
        trial.rates.query = terms.q;
        Ok(())
    })
}

#[tauri::command(async)]
pub fn rate_toggle(state: State<'_, AppState>, id: ipc::IpcId) -> Result<String, String> {
    with(&state, "ratelist", |trial| {
        trial.require_owner()?;
        trial.rates.toggle(alm_core::ProductId::new(id.value()));
        Ok(())
    })
}

/// The page sends the device's local calendar date once at start.
#[tauri::command(async)]
pub fn start(state: State<'_, AppState>, today: String) -> Result<String, String> {
    let today = today
        .parse()
        .map_err(|_| "The device date is not a valid date.".to_owned())?;
    with(&state, "bill", |trial| {
        trial.today = Some(today);
        Ok(())
    })
}

#[tauri::command(async)]
pub fn review_role(state: State<'_, AppState>, owner: bool) -> Result<String, String> {
    with(&state, "review", |trial| {
        if trial.owner != owner {
            trial.posting.invalidate();
        }
        trial.set_owner(owner)
    })
}

/// Opens the platform print dialog for the current page; its print CSS keeps only the invoice.
#[tauri::command]
pub fn print_page(window: tauri::WebviewWindow) -> Result<(), String> {
    #[cfg(desktop)]
    return window.print().map_err(text);
    #[cfg(mobile)]
    {
        let _ = window;
        Err("Printing on mobile goes through the platform print bridge.".into())
    }
}

/// Plain-text summary for the platform share sheet.
#[tauri::command(async)]
pub fn share_text(state: State<'_, AppState>) -> Result<String, String> {
    let trial = state.lock().map_err(|_| "app state poisoned".to_owned())?;
    let role = trial.require_owner()?;
    let mut summary = Err("GST bill missing".to_owned());
    trial
        .posted
        .emit_gst_bill(role, trial.gst_view, &mut |invoice| {
            summary = Ok(format!(
                "GST Invoice {} dated {} for {}: ₹{}",
                invoice.invoice_number(),
                invoice.dated(),
                invoice.record.customer,
                gst_print::inr(invoice.totals.total)
            ));
        })
        .map_err(debug)?;
    summary
}
