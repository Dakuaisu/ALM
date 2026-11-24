//! DEVELOPMENT SIMULATION of the sales authority over the synthetic `ConfirmationFixture`.
//! It stands in for the hosted service so the device data layer can be exercised offline. Its
//! accepted commands are journaled to a file outside every account's device tables and
//! replayed on open; it is not a server, a sync engine, or a source of real records.
use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use alm_core::confirmation::manual_cashbook::{
    CashBookDeletion, ManualCashBook, ManualCashBookDraft,
};
use alm_core::confirmation::owner_billing::{
    BillingDirectory, OwnerPosting, ReceiptTerms, SaleTerms, priced_draft, save_receipt,
};
use alm_core::confirmation::{ConfirmationFixture, LineEdit, ReceiptMode};
use alm_core::projection::ProjectionBatch;
use alm_core::{DraftId, Role};

use crate::{AuthorityError, Intent, Outcome, RequestId, SaleReceipt, SalesAuthority};

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
}

pub struct LocalAuthority {
    state: State,
    directory: BillingDirectory,
    /// request ID → (canonical intent, canonical outcome).
    requests: BTreeMap<String, (String, String)>,
    gst: BTreeMap<String, u64>,
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
            },
            directory,
            requests: BTreeMap::new(),
            gst: BTreeMap::new(),
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
                ["R", request, intent] => {
                    let request = RequestId::new(request).map_err(|_| corrupt())?;
                    let intent = Intent::parse(intent).ok_or_else(corrupt)?;
                    authority.commit(&request, &intent).map_err(|_| corrupt())?;
                }
                ["G", bill, number] => {
                    let number = number.parse().map_err(|_| corrupt())?;
                    authority.gst.insert((*bill).to_owned(), number);
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
    fn commit(&mut self, request: &RequestId, intent: &Intent) -> Result<Outcome, AuthorityError> {
        let canonical = intent.canonical();
        if let Some((stored, outcome)) = self.requests.get(request.as_str()) {
            return if *stored == canonical {
                Outcome::parse(outcome).ok_or(AuthorityError::Invalid)
            } else {
                Err(AuthorityError::Conflict)
            };
        }
        let mut next = self.state.clone();
        let outcome = apply(&mut next, &self.directory, intent)?;
        self.state = next;
        self.requests.insert(
            request.as_str().to_owned(),
            (canonical, outcome.canonical()),
        );
        self.checkpoint += 1;
        Ok(outcome)
    }
}

fn apply(
    state: &mut State,
    directory: &BillingDirectory,
    intent: &Intent,
) -> Result<Outcome, AuthorityError> {
    let role = Role::Owner;
    let fixture = &mut state.fixture;
    match intent {
        Intent::PostSale(sale) => {
            let id = sale.draft.id();
            if fixture.current_sale(role, id).is_ok() {
                return Err(AuthorityError::AlreadyPosted);
            }
            let priced = priced_draft(fixture, role, &sale.draft).map_err(rejected)?;
            let current: Vec<_> = priced.lines.iter().map(|line| line.unit_price).collect();
            if current != sale.prices {
                return Err(AuthorityError::StaleReview);
            }
            let mut posting = OwnerPosting::default();
            posting
                .prepare(
                    fixture,
                    directory,
                    role,
                    Some(&sale.draft),
                    SaleTerms {
                        date: &sale.date.to_string(),
                        paid: &sale.paid.to_rupee_text(),
                    },
                )
                .map_err(rejected)?;
            let serial = posting
                .pending(role)
                .map(|p| p.serial())
                .ok_or(AuthorityError::Invalid)?;
            let receipt = posting.confirm(fixture, role, serial).map_err(rejected)?;
            Ok(Outcome::Sale(SaleReceipt {
                draft: id.value(),
                invoice: receipt.invoice,
                quantity: receipt.quantity,
                total: receipt.total,
            }))
        }
        Intent::CancelSale { draft } => fixture
            .cancel_sale(role, DraftId::new(*draft))
            .map(|()| Outcome::Done)
            .map_err(rejected),
        Intent::RestoreSale { draft } => fixture
            .undo_cancel_sale(role, DraftId::new(*draft))
            .map(|()| Outcome::Done)
            .map_err(rejected),
        Intent::EditSale { draft, lines } => {
            let edits = lines
                .iter()
                .map(|(quantity, rate)| {
                    Some(LineEdit {
                        quantity: std::num::NonZeroU32::new(*quantity)?,
                        price: alm_core::Money::from_minor_units(*rate),
                    })
                })
                .collect::<Option<Vec<_>>>()
                .ok_or(AuthorityError::Invalid)?;
            let receipt = fixture
                .edit_posted_lines(role, DraftId::new(*draft), &edits)
                .map_err(rejected)?;
            Ok(Outcome::Sale(SaleReceipt {
                draft: *draft,
                invoice: receipt.invoice,
                quantity: receipt.quantity,
                total: receipt.total,
            }))
        }
        Intent::RecordReceipt {
            customer,
            date,
            amount,
            online,
        } => save_receipt(
            fixture,
            directory,
            role,
            customer,
            ReceiptTerms {
                date: &date.to_string(),
                amount: &amount.to_rupee_text(),
                mode: if *online {
                    ReceiptMode::Online
                } else {
                    ReceiptMode::Cash
                },
            },
        )
        .map(|()| Outcome::Done)
        .map_err(rejected),
        Intent::SaveCashDay { id, date, amounts } => {
            let mut draft = match id {
                Some(id) => state.cashbook.find(role, *id).map_err(rejected)?.draft(),
                None => ManualCashBookDraft::new(None, *amounts),
            };
            draft.date = Some(*date);
            draft.amounts = *amounts;
            state.cashbook.save(role, &draft).map_err(rejected)?;
            Ok(Outcome::Done)
        }
        Intent::DeleteCashDay { id } => {
            let row = state.cashbook.find(role, *id).map_err(rejected)?;
            state
                .cashbook
                .delete(role, CashBookDeletion::Confirm(row.id))
                .map_err(rejected)?;
            Ok(Outcome::Done)
        }
    }
}

impl SalesAuthority for LocalAuthority {
    fn submit(
        &mut self,
        role: Role,
        request: &RequestId,
        intent: &Intent,
    ) -> Result<Outcome, AuthorityError> {
        if role != Role::Owner {
            return Err(AuthorityError::Denied);
        }
        let known = self.requests.contains_key(request.as_str());
        let outcome = self.commit(request, intent)?;
        if !known {
            self.append(&format!("R\t{}\t{}", request.as_str(), intent.canonical()))?;
        }
        if std::mem::take(&mut self.lose_next) {
            return Err(AuthorityError::Unavailable);
        }
        Ok(outcome)
    }

    fn status(&self, role: Role, request: &RequestId) -> Result<Option<Outcome>, AuthorityError> {
        if role != Role::Owner {
            return Err(AuthorityError::Denied);
        }
        Ok(self
            .requests
            .get(request.as_str())
            .and_then(|(_, outcome)| Outcome::parse(outcome)))
    }

    fn allocate_gst_number(&mut self, role: Role, bill_id: &str) -> Result<u64, AuthorityError> {
        if role != Role::Owner {
            return Err(AuthorityError::Denied);
        }
        if bill_id.is_empty() || bill_id.len() > 64 || bill_id.contains(['\t', '\n']) {
            return Err(AuthorityError::Invalid);
        }
        if let Some(number) = self.gst.get(bill_id) {
            return Ok(*number);
        }
        let stored = self
            .state
            .fixture
            .next_gst_bill_number(role)
            .map_err(rejected)?
            .0;
        let number = self
            .gst
            .values()
            .max()
            .map_or(stored, |max| stored.max(max + 1));
        self.append(&format!("G\t{bill_id}\t{number}"))?;
        self.gst.insert(bill_id.to_owned(), number);
        self.checkpoint += 1;
        Ok(number)
    }

    fn pull(&self, role: Role, checkpoint: Option<u64>) -> Result<ProjectionBatch, AuthorityError> {
        let snapshot = if checkpoint == Some(self.checkpoint) {
            None
        } else {
            Some(
                self.state
                    .fixture
                    .projection(role, &self.directory, &self.state.cashbook)
                    .map_err(rejected)?,
            )
        };
        Ok(ProjectionBatch {
            role,
            checkpoint: self.checkpoint,
            as_of_ms: (self.clock)(),
            snapshot,
        })
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
