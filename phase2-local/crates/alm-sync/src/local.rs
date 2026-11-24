//! DEVELOPMENT SIMULATION of the sales authority over the synthetic `ConfirmationFixture`.
//! It stands in for the hosted service so the device data layer can be exercised offline. Its
//! accepted commands are journaled to a file outside every account's device tables and
//! replayed on open; it is not a server, a sync engine, or a source of real records.
use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::{Path, PathBuf};

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

/// The simulated authority's whole state.
#[derive(Clone)]
struct State {
    fixture: ConfirmationFixture,
    cashbook: ManualCashBook,
    /// GST bill key → allocated number.
    gst: BTreeMap<String, u64>,
}

mod apply;
mod authority;
mod commands;
use apply::apply;

pub struct LocalAuthority {
    state: State,
    directory: BillingDirectory,
    /// request ID → (canonical intent, canonical outcome).
    requests: BTreeMap<String, (String, String)>,
    checkpoint: u64,
    journal: Option<PathBuf>,
    clock: fn() -> u64,
    lose_next: bool,
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
            },
            directory,
            requests: BTreeMap::new(),
            checkpoint: 0,
            journal: None,
            clock: system_clock,
            lose_next: false,
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
                ["R", request, intent, stamp] => {
                    let request = RequestId::new(request).map_err(|_| corrupt())?;
                    let intent = Intent::parse(intent).ok_or_else(corrupt)?;
                    let stamp = stamp.parse().map_err(|_| corrupt())?;
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
        Ok(authority)
    }

    /// Replaces the wall clock used for projection timestamps (tests).
    #[must_use]
    pub fn with_clock(mut self, clock: fn() -> u64) -> Self {
        self.clock = clock;
        self
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
        let mut next = self.state.clone();
        let outcome = apply(&mut next, &self.directory, intent, stamp)?;
        self.state = next;
        self.requests.insert(
            request.as_str().to_owned(),
            (canonical, outcome.canonical()),
        );
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
