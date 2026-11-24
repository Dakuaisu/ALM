//! Account switching, the freshness line with unresolved intents, and their commands.
use alm_core::Role;
use alm_core::projection::as_of_text;
use alm_store::{Outbox, ProjectionStore, StoredProjection};
use alm_sync::Outcome;
use tauri::State;

use crate::{AppState, Trial, text, with};

impl Trial {
    /// Switches the synthetic account: the previous account's drafts, GST bills, outbox and
    /// projection are closed and every in-memory screen state is cleared.
    pub(crate) fn switch_account(&mut self, account: &str) -> Result<(), String> {
        use alm_store::SqliteSales as _;
        let drafts =
            alm_core::LocalSales::open_manager_account(&self.device_root, account).map_err(text)?;
        let gst =
            alm_store::GstBillDatabase::open_scoped(&self.device_root, account).map_err(text)?;
        let outbox = Outbox::open_scoped(&self.device_root, account).map_err(text)?;
        let projection =
            ProjectionStore::open_scoped(&self.device_root, account, self.role()).map_err(text)?;
        self.drafts = drafts;
        self.gst_db = gst;
        self.outbox = outbox;
        self.projection = projection;
        self.account = account.to_owned();
        self.current = None;
        self.reviewed = None;
        self.posting.invalidate();
        self.sale_edit = None;
        self.cash_form = None;
        self.gst_form = None;
        self.customers = crate::views::CustomersState::default();
        self.sales_filter = crate::views::Filter::default();
        self.restored = None;
        self.print_view = None;
        self.message = format!("Switched to synthetic account {account}.");
        self.reconnect()?;
        self.migrate_gst_tables()
    }

    /// The "as of" line for projected screens and any unresolved intents, with their actions.
    pub(crate) fn freshness(&self, stored: &StoredProjection) -> Result<String, String> {
        let mut html = format!(
            r#"<div class="help no-print" data-freshness="{}">Posted data {} · checkpoint {} · account {}</div>"#,
            stored.checkpoint,
            if stored.as_of_ms == 0 {
                "not received yet".to_owned()
            } else {
                as_of_text(stored.as_of_ms)
            },
            stored.checkpoint,
            crate::gst_print::esc(&self.account)
        );
        if self.role() == Role::Owner {
            for entry in self.outbox.unresolved().map_err(text)? {
                html += &format!(
                    r#"<div class="card" role="status" style="margin:8px 0;border-left:3px solid var(--warn)"><b>Outcome unknown</b> · request {0}<div class="help">{1}</div><div class="row" style="margin-top:6px"><button class="btn sm" data-action="outbox_check">Check status</button><button class="btn sm" data-action="outbox_retry" data-q="{0}">Retry the same request</button></div></div>"#,
                    crate::gst_print::esc(&entry.request_id),
                    crate::gst_print::esc(&entry.intent)
                );
            }
        }
        Ok(html)
    }
}

#[tauri::command(async)]
pub fn outbox_check(state: State<'_, AppState>) -> Result<String, String> {
    with(&state, "review", |trial| {
        trial.require_owner()?;
        trial.reconnect()?;
        trial.message = "Status checked with the authority.".into();
        Ok(())
    })
}

#[tauri::command(async)]
pub fn outbox_retry(state: State<'_, AppState>, q: String) -> Result<String, String> {
    with(&state, "review", |trial| {
        let outcome = trial.retry(&q)?;
        trial.message = match outcome {
            Outcome::Sale(receipt) => format!("Bill #{} confirmed.", receipt.invoice),
            Outcome::Done | Outcome::GstNumber(_) | Outcome::Created(_) => "Confirmed.".into(),
        };
        Ok(())
    })
}

#[tauri::command(async)]
pub fn account_switch(state: State<'_, AppState>, q: String) -> Result<String, String> {
    with(&state, "review", |trial| trial.switch_account(&q))
}
