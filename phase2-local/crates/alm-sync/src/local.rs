//! DEVELOPMENT SIMULATION of the sales authority over the synthetic `ConfirmationFixture`.
//! It stands in for the hosted service so the device data layer can be exercised offline. Its
//! accepted commands are journaled to a file outside every account's device tables and
//! replayed on open; it is not a server, a sync engine, or a source of real records.
use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use alm_core::Role;
use alm_core::confirmation::ConfirmationFixture;
use alm_core::confirmation::manual_cashbook::ManualCashBook;
use alm_core::confirmation::owner_billing::BillingDirectory;

use crate::{AuthorityError, Intent, Outcome, RequestId};

#[derive(Debug)]
pub enum LocalAuthorityError {
    Io(std::io::Error),
    /// A journal line that does not replay; its 1-based line number.
    Corrupt(usize),
}

impl From<std::io::Error> for LocalAuthorityError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

/// What the authority holds from legacy imports: manifests and records, and each stored
/// source file by its SHA-256 (Owner only; never in a pull).
#[derive(Clone, Debug, Default)]
pub struct LegacyStore {
    pub imports: alm_core::legacy::import::LegacyImports,
    pub sources: BTreeMap<String, std::sync::Arc<str>>,
}

/// The simulated authority's whole state.
#[derive(Clone)]
struct State {
    fixture: ConfirmationFixture,
    cashbook: ManualCashBook,
    /// GST bill key → allocated number.
    gst: BTreeMap<String, u64>,
    /// The activity log, in recording order.
    activity: Vec<alm_core::projection::book::ActivityRow>,
    /// Legacy imports: manifests and placed records.
    legacy: alm_core::legacy::import::LegacyImports,
    /// Each imported legacy file as uploaded, by its SHA-256 (Owner-only; not in any pull).
    legacy_sources: BTreeMap<String, std::sync::Arc<str>>,
}

mod activity;
mod apply;
mod authority;
mod commands;
use apply::apply;

pub struct LocalAuthority {
    state: State,
    directory: BillingDirectory,
    /// request ID → (canonical intent, canonical outcome).
    requests: BTreeMap<String, (String, String)>,
    /// request ID → the account that sent it (a Manager sees only its own, DEC-034).
    senders: BTreeMap<String, String>,
    checkpoint: u64,
    journal: Option<PathBuf>,
    clock: fn() -> u64,
    lose_next: bool,
    /// The role the authority currently grants this device's account (`None`: revoked).
    granted: Option<Role>,
    /// The device and account the next commands come from (a server reads them from the
    /// request and its token).
    caller: (String, String),
    /// The device and account each role's commands come from (Owner, Manager): the
    /// simulation stands in for two people's devices.
    callers: [(String, String); 2],
    /// The change history pulls are answered from (rebuilt from scratch after a restart).
    changes: std::sync::Mutex<crate::delta::Tracker>,
}

impl LocalAuthority {
    /// Keeps `revisions` of change history (pulls from older checkpoints get everything).
    #[doc(hidden)]
    pub fn keep_changes(&mut self, revisions: u64) {
        self.changes = std::sync::Mutex::new(crate::delta::Tracker::new(revisions));
    }
}

fn system_clock() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

fn rejected(error: impl std::fmt::Debug) -> AuthorityError {
    AuthorityError::Rejected(format!("{error:?}"))
}

impl LocalAuthority {
    /// An in-memory simulation seeded with `fixture`.
    pub fn new(fixture: ConfirmationFixture, directory: BillingDirectory) -> Self {
        Self {
            state: State {
                fixture,
                cashbook: ManualCashBook::default(),
                gst: BTreeMap::new(),
                activity: Vec::new(),
                legacy: alm_core::legacy::import::LegacyImports::default(),
                legacy_sources: BTreeMap::new(),
            },
            directory,
            requests: BTreeMap::new(),
            senders: BTreeMap::new(),
            checkpoint: 0,
            journal: None,
            clock: system_clock,
            lose_next: false,
            granted: Some(Role::Owner),
            caller: (String::new(), String::new()),
            callers: Default::default(),
            changes: std::sync::Mutex::new(crate::delta::Tracker::default()),
        }
    }

    /// Seeds with `fixture`, replays the journal at `journal` (created when missing) and keeps
    /// appending accepted commands to it.
    ///
    /// # Errors
    /// Fails on I/O errors and on a journal line that does not replay.
    pub fn open(
        journal: &Path,
        fixture: ConfirmationFixture,
        directory: BillingDirectory,
    ) -> Result<Self, LocalAuthorityError> {
        let mut authority = Self::new(fixture, directory);
        let text = match std::fs::read_to_string(journal) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(error) => return Err(error.into()),
        };
        for (index, line) in text.lines().enumerate() {
            let corrupt = || LocalAuthorityError::Corrupt(index + 1);
            match line.split('\t').collect::<Vec<_>>().as_slice() {
                ["R", request, intent, stamp, caller @ ..] => {
                    let request = RequestId::new(request).map_err(|_| corrupt())?;
                    let intent = Intent::parse(intent).ok_or_else(corrupt)?;
                    let stamp = stamp.parse().map_err(|_| corrupt())?;
                    authority.caller = match caller {
                        [] => (String::new(), String::new()),
                        [device, account] => ((*device).to_owned(), (*account).to_owned()),
                        _ => return Err(corrupt()),
                    };
                    authority
                        .commit(&request, &intent, stamp)
                        .map_err(|_| corrupt())?;
                }
                // Journals written before commands were time-stamped.
                ["R", request, intent] => {
                    let request = RequestId::new(request).map_err(|_| corrupt())?;
                    let intent = Intent::parse(intent).ok_or_else(corrupt)?;
                    authority
                        .commit(&request, &intent, system_clock())
                        .map_err(|_| corrupt())?;
                }
                ["G", bill, number] => {
                    let number = number.parse().map_err(|_| corrupt())?;
                    authority.state.gst.insert((*bill).to_owned(), number);
                    authority.checkpoint += 1;
                }
                _ => return Err(corrupt()),
            }
        }
        authority.journal = Some(journal.to_path_buf());
        authority.caller = (String::new(), String::new());
        Ok(authority)
    }

    /// The legacy imports and stored source files a journal holds, replayed through the
    /// authority's legacy rules only (the rest of the journal is read, not applied), as
    /// `alm-cli verify-legacy-import` reads a device's simulated authority.
    ///
    /// # Errors
    /// Fails on I/O errors and on a journal line that does not parse or a legacy command that
    /// does not replay.
    pub fn legacy_journal(journal: &Path) -> Result<LegacyStore, LocalAuthorityError> {
        let text = std::fs::read_to_string(journal)?;
        let mut store = LegacyStore::default();
        for (index, line) in text.lines().enumerate() {
            let corrupt = || LocalAuthorityError::Corrupt(index + 1);
            let intent = match line.split('\t').collect::<Vec<_>>().as_slice() {
                ["R", _, intent, ..] => Intent::parse(intent).ok_or_else(corrupt)?,
                ["G", ..] => continue,
                _ => return Err(corrupt()),
            };
            match &intent {
                Intent::ImportLegacy(plan) => {
                    commands::import_legacy(&mut store.imports, &mut store.sources, plan)
                        .map_err(|_| corrupt())?;
                }
                Intent::ResolveLegacyQuarantine(decision) => {
                    alm_core::legacy::resolve::resolve(&mut store.imports, decision)
                        .map_err(|_| corrupt())?;
                }
                _ => {}
            }
        }
        Ok(store)
    }

    /// The legacy imports and stored source files of this authority (Owner only).
    ///
    /// # Errors
    /// Denies Managers.
    pub fn legacy_store(&self, role: Role) -> Result<LegacyStore, AuthorityError> {
        if role != Role::Owner {
            return Err(AuthorityError::Denied);
        }
        Ok(LegacyStore {
            imports: self.state.legacy.clone(),
            sources: self.state.legacy_sources.clone(),
        })
    }

    /// Replaces the wall clock used for projection timestamps (tests).
    #[must_use]
    pub fn with_clock(mut self, clock: fn() -> u64) -> Self {
        self.clock = clock;
        self
    }

    /// Simulates the account's role changing at the authority (`None` revokes it); devices see
    /// it on their next pull. Not journaled.
    pub const fn set_granted(&mut self, role: Option<Role>) {
        self.granted = role;
    }

    /// The device and account the next commands come from (recorded in the activity log).
    /// Tabs and line breaks are not allowed.
    pub fn set_caller(&mut self, device: &str, account: &str) {
        let clean = |t: &str| t.replace(['\t', '\n', '\r'], " ");
        self.caller = (clean(device), clean(account));
        self.callers = [self.caller.clone(), self.caller.clone()];
    }

    /// The device and account the next commands sent as `role` come from.
    pub fn set_role_caller(&mut self, role: Role, device: &str, account: &str) {
        let clean = |t: &str| t.replace(['\t', '\n', '\r'], " ");
        self.callers[usize::from(role == Role::Manager)] = (clean(device), clean(account));
    }

    fn caller_of(&self, role: Role) -> &(String, String) {
        &self.callers[usize::from(role == Role::Manager)]
    }

    /// Simulates a lost response: the next command commits but its reply is dropped.
    pub const fn lose_next_response(&mut self) {
        self.lose_next = true;
    }

    /// The simulation's fixture, for screens and actions not yet routed through the trait.
    pub const fn fixture(&self) -> &ConfirmationFixture {
        &self.state.fixture
    }

    /// Unjournaled access for actions not yet routed through the trait; such changes are not
    /// replayed after a restart.
    pub fn fixture_mut(&mut self) -> &mut ConfirmationFixture {
        self.checkpoint += 1;
        &mut self.state.fixture
    }

    pub const fn cashbook(&self) -> &ManualCashBook {
        &self.state.cashbook
    }

    pub const fn directory(&self) -> &BillingDirectory {
        &self.directory
    }

    pub const fn checkpoint(&self) -> u64 {
        self.checkpoint
    }

    fn append(&self, line: &str) -> Result<(), AuthorityError> {
        let Some(path) = &self.journal else {
            return Ok(());
        };
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|_| AuthorityError::Unavailable)?;
        file.write_all(format!("{line}\n").as_bytes())
            .and_then(|()| file.sync_all())
            .map_err(|_| AuthorityError::Unavailable)
    }

    /// Validates and applies one intent atomically: a failure leaves no effect.
    /// `stamp` is the authority's time for the command (journaled, so a replay repeats it).
    fn commit(
        &mut self,
        request: &RequestId,
        intent: &Intent,
        stamp: u64,
    ) -> Result<Outcome, AuthorityError> {
        let canonical = intent.canonical();
        if let Some((stored, outcome)) = self.requests.get(request.as_str()) {
            return if *stored == canonical {
                Outcome::parse(outcome).ok_or(AuthorityError::Invalid)
            } else {
                Err(AuthorityError::Conflict)
            };
        }
        let outcome = if matches!(intent, Intent::PostSale(_)) {
            // A sale is checked before anything changes (core `post_saved_sale`), so it applies
            // in place: copying the whole state for every bill made posting quadratic.
            let outcome = apply(&mut self.state, &self.directory, intent, stamp)?;
            if let Some(row) = activity::record(
                &self.state,
                &self.state,
                &self.directory,
                intent,
                &outcome,
                (stamp, &self.caller.0, &self.caller.1),
            ) {
                self.state.activity.push(row);
            }
            outcome
        } else {
            let mut next = self.state.clone();
            let outcome = apply(&mut next, &self.directory, intent, stamp)?;
            if let Some(row) = activity::record(
                &self.state,
                &next,
                &self.directory,
                intent,
                &outcome,
                (stamp, &self.caller.0, &self.caller.1),
            ) {
                next.activity.push(row);
            }
            self.state = next;
            outcome
        };
        self.requests.insert(
            request.as_str().to_owned(),
            (canonical, outcome.canonical()),
        );
        self.senders
            .insert(request.as_str().to_owned(), self.caller.1.clone());
        self.checkpoint += 1;
        Ok(outcome)
    }
}

/// Reads (and, for actions not yet routed through [`SalesAuthority`], unjournaled writes) go to
/// the simulation's fixture.
impl std::ops::Deref for LocalAuthority {
    type Target = ConfirmationFixture;

    fn deref(&self) -> &ConfirmationFixture {
        &self.state.fixture
    }
}

/// Any unjournaled write counts as a new authority revision, so devices pull it.
impl std::ops::DerefMut for LocalAuthority {
    fn deref_mut(&mut self) -> &mut ConfirmationFixture {
        self.checkpoint += 1;
        &mut self.state.fixture
    }
}
