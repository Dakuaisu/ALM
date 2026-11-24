//! The device data layer: actions become intents stored in the outbox before they are sent to
//! the `SalesAuthority`; posted data shown on screens comes from the account's projection.
//! A stored intent is only ever re-sent by an explicit retry with its original request ID.
use alm_core::confirmation::ConfirmationFixture;
use alm_core::confirmation::manual_cashbook::ManualCashBook;
use alm_core::{Money, ProductId, Role};
use alm_store::{OutboxStatus, ProjectionStore, StoredProjection};
use alm_sync::{AuthorityError, Intent, Lookup, Outcome, RequestId};

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

/// The posted data screens read: rebuilt by the core from this account's stored projection
/// (the Owner's book rows), never from the authority. A Manager projection carries no book, so
/// the Manager view is empty and the core denies its screens before reading.
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
    /// A Manager drafts from the catalog its device pulled (names and sale prices only), from
    /// whichever authority it uses; `None` for the Owner, who drafts from the book rows.
    pub(crate) fn manager_catalog(&self) -> Option<Vec<alm_core::projection::ProjectedProduct>> {
        (self.role() == Role::Manager).then(|| {
            self.projection
                .load()
                .ok()
                .flatten()
                .map(|stored| stored.snapshot.catalog)
                .unwrap_or_default()
        })
    }

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
        self.posted = match stored.snapshot.owner.as_ref() {
            Some(owner) if role == Role::Owner => {
                let (fixture, cashbook) =
                    ConfirmationFixture::from_owner_book(&owner.book).map_err(debug)?;
                Posted {
                    fixture,
                    cashbook,
                    built: None,
                }
            }
            _ => Posted::empty()?,
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
        let prefix = alm_sync::request_prefix(&self.account, &self.device_id);
        let request = self.outbox.next_request_id(&prefix).map_err(text)?;
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
        let (device, account) = (self.device_id.clone(), self.account.clone());
        self.authority.set_caller(&device, &account);
        match self.sales_authority().submit(role, &id, intent) {
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
                let (status, note) = match self.sales_authority().lookup(Role::Owner, &id, &intent)
                {
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
        match self.sales_authority().pull(role, since) {
            // The authority grants less than this device holds: drop it before anything else.
            Ok(batch) if batch.role != role => return self.downgrade(batch.role),
            Err(AuthorityError::Denied) => return self.revoke(),
            Ok(batch) => match self.projection.apply(&batch) {
                Ok(()) => {}
                // The authority's checkpoint is behind ours (its history was reset): rebuild.
                Err(alm_store::ProjectionError::Rejected) if batch.role == role => {
                    self.projection.clear().map_err(text)?;
                    let fresh = self
                        .sales_authority()
                        .pull(role, None)
                        .map_err(|e| refused(&e))?;
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
mod revoke;

/// Every Owner screen rendered from the book rows equals the same screen rendered from the
/// simulation's own state.
#[cfg(test)]
pub(crate) fn assert_book_view(trial: &mut Trial) {
    const SCREENS: [&str; 17] = [
        "customers",
        "expenses",
        "dashboard",
        "reorder",
        "stock",
        "godown",
        "gstsales",
        "products",
        "suppliers",
        "order",
        "purchases",
        "crates",
        "ratelist",
        "reports",
        "counter",
        "receipt",
        "sales",
    ];
    trial.set_owner(true).unwrap();
    // A screen the core cannot build fails the same way from both.
    let render = |trial: &mut Trial, screen: &str| {
        trial.message.clear();
        trial.render(screen)
    };
    let from_book: Vec<Result<String, String>> = SCREENS.iter().map(|s| render(trial, s)).collect();
    trial.posted.fixture = trial.authority.fixture().clone();
    trial.posted.cashbook = trial.authority.cashbook().clone();
    for (screen, book) in SCREENS.iter().zip(from_book) {
        let sim = render(trial, screen);
        if sim != book {
            let (a, b) = (format!("{book:?}"), format!("{sim:?}"));
            let at = a.chars().zip(b.chars()).take_while(|(x, y)| x == y).count();
            let (a, b): (String, String) = (
                a.chars().skip(at.saturating_sub(200)).take(500).collect(),
                b.chars().skip(at.saturating_sub(200)).take(500).collect(),
            );
            panic!("{screen} differs:\nbook {a}\nsim  {b}");
        }
    }
    trial.posted = Posted::empty().unwrap();
    trial.refresh().unwrap();
}
