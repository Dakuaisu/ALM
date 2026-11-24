//! The device data layer: actions become intents stored in the outbox before they are sent to
//! the `SalesAuthority`; posted data shown on screens comes from the account's projection.
//! A stored intent is only ever re-sent by an explicit retry with its original request ID.
use alm_core::Role;
use alm_core::projection::as_of_text;
use alm_store::{Outbox, OutboxStatus, ProjectionStore, StoredProjection};
use alm_sync::{AuthorityError, Intent, Outcome, RequestId, SalesAuthority};
use tauri::State;

use crate::{AppState, Trial, debug, text, with};

fn refused(error: &AuthorityError) -> String {
    match error {
        AuthorityError::Conflict => "This request was already used for a different action.".into(),
        AuthorityError::AlreadyPosted => "This draft is already posted.".into(),
        AuthorityError::StaleReview => {
            "Prices changed since the review: review the sale again.".into()
        }
        AuthorityError::Denied => "Access denied: Owner only.".into(),
        AuthorityError::Invalid => "The request was not valid.".into(),
        AuthorityError::Rejected(reason) => format!("Rejected: {reason}"),
        AuthorityError::Unavailable => {
            "No answer from the authority: the outcome is unknown. Check its status or retry it from Owner review.".into()
        }
    }
}

impl Trial {
    /// Stores `intent` with a new request ID, then sends it.
    pub(crate) fn send(&mut self, intent: &Intent) -> Result<Outcome, String> {
        let role = self.require_owner()?;
        let request = self.outbox.next_request_id(&self.account).map_err(text)?;
        self.outbox
            .record(&request, &intent.canonical())
            .map_err(debug)?;
        self.deliver(role, &request, intent)
    }

    fn deliver(&mut self, role: Role, request: &str, intent: &Intent) -> Result<Outcome, String> {
        let id = RequestId::new(request).map_err(debug)?;
        match self.authority.submit(role, &id, intent) {
            Ok(outcome) => {
                self.outbox
                    .mark(request, OutboxStatus::Confirmed, Some(&outcome.canonical()))
                    .map_err(debug)?;
                self.refresh()?;
                Ok(outcome)
            }
            Err(AuthorityError::Unavailable) => {
                self.outbox
                    .mark(request, OutboxStatus::Unknown, None)
                    .map_err(debug)?;
                Err(refused(&AuthorityError::Unavailable))
            }
            Err(error) => {
                self.outbox
                    .mark(request, OutboxStatus::Rejected, Some(&refused(&error)))
                    .map_err(debug)?;
                Err(refused(&error))
            }
        }
    }

    /// An explicit retry of an unresolved intent, with its original request ID and intent.
    pub(crate) fn retry(&mut self, request: &str) -> Result<Outcome, String> {
        let role = self.require_owner()?;
        let entry = self
            .outbox
            .get(request)
            .map_err(text)?
            .filter(|e| matches!(e.status, OutboxStatus::Unknown | OutboxStatus::Pending))
            .ok_or("Nothing to retry.")?;
        let intent = Intent::parse(&entry.intent).ok_or("The stored intent is unreadable.")?;
        self.deliver(role, request, &intent)
    }

    /// On reconnect: looks up the status of every unresolved intent (never re-sends one, and
    /// never posts a saved draft), then refreshes the projection.
    pub(crate) fn reconnect(&mut self) -> Result<(), String> {
        if self.role() == Role::Owner {
            for entry in self.outbox.unresolved().map_err(text)? {
                let id = RequestId::new(&entry.request_id).map_err(debug)?;
                match self.authority.status(Role::Owner, &id) {
                    Ok(Some(outcome)) => self
                        .outbox
                        .mark(
                            &entry.request_id,
                            OutboxStatus::Confirmed,
                            Some(&outcome.canonical()),
                        )
                        .map_err(debug)?,
                    Ok(None) => self
                        .outbox
                        .mark(&entry.request_id, OutboxStatus::Unknown, None)
                        .map_err(debug)?,
                    Err(_) => {}
                }
            }
        }
        self.refresh()
    }

    /// Pulls from the stored checkpoint and applies it; opens the projection of the current
    /// role first if the role changed.
    pub(crate) fn refresh(&mut self) -> Result<(), String> {
        let role = self.role();
        if self.projection.role() != role {
            self.projection = ProjectionStore::open_scoped(&self.device_root, &self.account, role)
                .map_err(text)?;
        }
        let since = self.projection.checkpoint().map_err(text)?.map(|(c, _)| c);
        match self.authority.pull(role, since) {
            Ok(batch) => match self.projection.apply(&batch) {
                Ok(()) => Ok(()),
                // The authority's checkpoint is behind ours (its history was reset): rebuild.
                Err(alm_store::ProjectionError::Rejected) if batch.role == role => {
                    self.projection.clear().map_err(text)?;
                    let fresh = self.authority.pull(role, None).map_err(|e| refused(&e))?;
                    self.projection.apply(&fresh).map_err(debug)
                }
                Err(error) => Err(debug(error)),
            },
            // Offline: keep showing the stored projection with its "as of" time.
            Err(AuthorityError::Unavailable) => Ok(()),
            Err(error) => Err(refused(&error)),
        }
    }

    /// The stored projection for the current role (refreshed first).
    pub(crate) fn projected(&mut self) -> Result<StoredProjection, String> {
        self.refresh()?;
        Ok(self
            .projection
            .load()
            .map_err(text)?
            .unwrap_or_else(|| StoredProjection {
                checkpoint: 0,
                as_of_ms: 0,
                snapshot: Default::default(),
            }))
    }

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
        self.reconnect()
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
            Outcome::Done | Outcome::GstNumber(_) => "Confirmed.".into(),
        };
        Ok(())
    })
}

#[tauri::command(async)]
pub fn account_switch(state: State<'_, AppState>, q: String) -> Result<String, String> {
    with(&state, "review", |trial| trial.switch_account(&q))
}
