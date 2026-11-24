//! The device data layer: actions become intents stored in the outbox before they are sent to
//! the `SalesAuthority`; posted data shown on screens comes from the account's projection.
//! A stored intent is only ever re-sent by an explicit retry with its original request ID.
use alm_core::confirmation::ConfirmationFixture;
use alm_core::confirmation::manual_cashbook::ManualCashBook;
use alm_core::{Money, ProductId, Role};
use alm_store::{OutboxStatus, ProjectionStore, StoredProjection};
use alm_sync::{AuthorityError, Intent, Lookup, Outcome, RequestId, SalesAuthority};

use crate::{Trial, debug, text};

pub(crate) fn refused(error: &AuthorityError) -> String {
    match error {
        AuthorityError::Conflict => "This request was already used for a different action.".into(),
        AuthorityError::AlreadyPosted => "This draft is already posted.".into(),
        AuthorityError::StaleReview => {
            "The terms changed since your review (prices, stock or the order): review again.".into()
        }
        AuthorityError::Denied => "Access denied: Owner only.".into(),
        AuthorityError::Invalid => "The request was not valid.".into(),
        AuthorityError::Rejected(reason) => reason.clone(),
        AuthorityError::Unavailable => {
            "No answer from the authority: the outcome is unknown. Check status, or retry with the same request.".into()
        }
    }
}

/// The posted data screens read: rebuilt from this account's stored projection (the Owner's
/// state sections), never from the authority. A Manager projection carries no state, so the
/// Manager view is empty and the core denies its screens before reading.
pub(crate) struct Posted {
    pub(crate) fixture: ConfirmationFixture,
    cashbook: ManualCashBook,
    /// The role and checkpoint this view was built for.
    built: Option<(Role, u64)>,
}

impl Posted {
    pub(crate) fn empty() -> Result<Self, String> {
        Ok(Self {
            fixture: ConfirmationFixture::new(ProductId::new(0), 0, Money::from_minor_units(0))
                .map_err(debug)?,
            cashbook: ManualCashBook::default(),
            built: None,
        })
    }

    pub(crate) const fn cashbook(&self) -> &ManualCashBook {
        &self.cashbook
    }
}

impl std::ops::Deref for Posted {
    type Target = ConfirmationFixture;

    fn deref(&self) -> &ConfirmationFixture {
        &self.fixture
    }
}

impl Trial {
    /// What New Bill drafts against: the Owner's projected view. A Manager projection holds
    /// only the catalog, so Manager drafting still reads the simulation's catalog (OPEN-004).
    pub(crate) fn drafting(&self) -> &ConfirmationFixture {
        match self.role() {
            Role::Owner => &self.posted.fixture,
            Role::Manager => self.authority.fixture(),
        }
    }

    /// Rebuilds the screens' view when the stored projection moved on.
    fn rebuild_posted(&mut self) -> Result<(), String> {
        let role = self.role();
        let Some(stored) = self.projection.load().map_err(text)? else {
            if self.posted.built.is_some() {
                self.posted = Posted::empty()?;
            }
            return Ok(());
        };
        if self.posted.built == Some((role, stored.checkpoint)) {
            return Ok(());
        }
        let state = stored
            .snapshot
            .owner
            .as_ref()
            .map(|owner| owner.state.as_slice())
            .unwrap_or_default();
        self.posted = if state.is_empty() {
            Posted::empty()?
        } else {
            let (fixture, cashbook) =
                ConfirmationFixture::from_state_sections(state).map_err(debug)?;
            Posted {
                fixture,
                cashbook,
                built: None,
            }
        };
        self.posted.built = Some((role, stored.checkpoint));
        Ok(())
    }

    /// Stores `intent` with a new request ID, then sends it. An intent already waiting with an
    /// unknown outcome is never sent again under a new ID: it is checked or retried as it is.
    pub(crate) fn send(&mut self, intent: &Intent) -> Result<Outcome, String> {
        let role = self.require_owner()?;
        let canonical = intent.canonical();
        if let Some(entry) = self
            .outbox
            .unresolved()
            .map_err(text)?
            .into_iter()
            .find(|e| e.intent == canonical)
        {
            return Err(format!(
                "Outcome unknown for request {}: check status / retry with the same request.",
                entry.request_id
            ));
        }
        let request = self.outbox.next_request_id(&self.account).map_err(text)?;
        self.outbox
            .record(&request, &intent.canonical())
            .map_err(debug)?;
        self.deliver(role, &request, intent)
    }

    pub(crate) fn deliver(
        &mut self,
        role: Role,
        request: &str,
        intent: &Intent,
    ) -> Result<Outcome, String> {
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

    /// On reconnect: resolves every unresolved intent by a status lookup against its stored
    /// intent (never re-sends one, and never posts a saved draft), then refreshes the projection.
    pub(crate) fn reconnect(&mut self) -> Result<(), String> {
        if self.role() == Role::Owner {
            for entry in self.outbox.unresolved().map_err(text)? {
                let id = RequestId::new(&entry.request_id).map_err(debug)?;
                let Some(intent) = Intent::parse(&entry.intent) else {
                    continue;
                };
                let (status, note) = match self.authority.lookup(Role::Owner, &id, &intent) {
                    Ok(Lookup::Committed(outcome)) => {
                        (OutboxStatus::Confirmed, Some(outcome.canonical()))
                    }
                    Ok(Lookup::NotCommitted) => (OutboxStatus::Unknown, None),
                    Ok(Lookup::OtherIntent) => (
                        OutboxStatus::Rejected,
                        Some(refused(&AuthorityError::Conflict)),
                    ),
                    Err(_) => continue,
                };
                self.outbox
                    .mark(&entry.request_id, status, note.as_deref())
                    .map_err(debug)?;
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
                Ok(()) => {}
                // The authority's checkpoint is behind ours (its history was reset): rebuild.
                Err(alm_store::ProjectionError::Rejected) if batch.role == role => {
                    self.projection.clear().map_err(text)?;
                    let fresh = self.authority.pull(role, None).map_err(|e| refused(&e))?;
                    self.projection.apply(&fresh).map_err(debug)?;
                }
                Err(error) => return Err(debug(error)),
            },
            // Offline: keep showing the stored projection with its "as of" time.
            Err(AuthorityError::Unavailable) => {}
            Err(error) => return Err(refused(&error)),
        }
        self.rebuild_posted()
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
}

pub mod account;
