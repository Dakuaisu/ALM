use super::{
    ConfirmationError, ConfirmationFixture, Receipt, RequestId, Role, dated_named_draft, draft,
    test_bill_date,
};
use crate::{DraftId, LocalSales, Money, ProductId};
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::{env, fs, process};

mod checkout;
mod codec;
mod competing;
mod crate_checkout;
mod crate_template;
mod fifo;
mod named_payment;
mod sqlite_pending;
mod transport;
use codec::{OwnerRequest, draft_fields, read_receipt, receipt_frame};
use transport::{Client, emit, send};

mod walk_in;

const MODE: &str = "ALM_CONFIRMATION_RESTART_MODE";
const SCRATCH: &str = "ALM_CONFIRMATION_RESTART_SCRATCH";
const PREFIX: &str = "confirmation-restart-";
const PENDING: &str = "owner.pending";
const DATABASE: &str = "synthetic-drafts.sqlite3";

#[test]
fn explicit_retry_after_client_restart_returns_original_receipt() {
    // Given one authority and an explicit Owner approval of its current review.
    let scratch = Scratch::new();
    let dir = &scratch.0;
    let pending_path = dir.join(PENDING);
    let id = DraftId::new(17);
    let mut fixture = ConfirmationFixture::new(ProductId::new(1), 5, Money::from_minor_units(90))
        .expect("create the synthetic authority");
    fixture.register_draft(draft(17, 1)).unwrap();
    fixture.register_draft(draft(17, 2)).unwrap();
    fixture
        .set_unit_price(Money::from_minor_units(125))
        .unwrap();
    let approved = OwnerRequest {
        id: RequestId::new(71),
        review: fixture.review(Role::Owner, id).unwrap(),
    };
    assert_eq!(effects(&fixture), (5, 0, 1, 0, 0));

    // When: the first client persists and sends the request, then loses the posted reply.
    let mut first = Client::start("submit", dir);
    first.write_frame(&approved.encode());
    let submitted = first.sent();
    assert_eq!(submitted, approved);
    let persisted = fs::read_to_string(&pending_path).expect("pending must exist before posting");
    assert_eq!(OwnerRequest::decode(&persisted), submitted);
    let original = fixture
        .confirm(Role::Owner, submitted.id, &submitted.review)
        .unwrap();
    assert_eq!(first.finish(), ["REPLY_EOF"]);
    assert_eq!(fs::read_to_string(&pending_path).unwrap(), persisted);
    assert_eq!(read_receipt(&persisted), None);
    assert_eq!(
        original,
        Receipt {
            draft_id: id,
            invoice: 1,
            quantity: 2,
            total: Money::from_minor_units(250),
            received: None,
            due: None,
            customer_balance: None,
        }
    );
    let committed = (3, 250, 2, 1, 1);
    assert_eq!(effects(&fixture), committed);

    // Then: reopen sends nothing; explicit retry below is the control for this same channel.
    fixture
        .set_unit_price(Money::from_minor_units(400))
        .unwrap();
    let reopened = draft_fields(approved.review.draft());
    assert_eq!(
        Client::start("reopen", dir).finish(),
        [format!("REOPENED {reopened}")],
        "SQLite reopen emitted an unexpected event (must send zero requests)"
    );
    assert_eq!(effects(&fixture), committed);

    let changed = fixture.review(Role::Owner, id).unwrap();
    assert_eq!(changed.draft(), approved.review.draft());
    assert_ne!(changed, approved.review);
    assert_eq!(changed.unit_price(), Money::from_minor_units(400));
    assert_eq!(
        fixture.confirm(Role::Owner, submitted.id, &changed),
        Err(ConfirmationError::RequestConflict)
    );
    assert_eq!(effects(&fixture), committed);

    let mut retry = Client::start("retry", dir);
    let resent = retry.sent();
    assert_eq!(
        resent, submitted,
        "retry must send the original identity and entire review"
    );
    let replay = fixture
        .confirm(Role::Owner, resent.id, &resent.review)
        .unwrap();
    assert_eq!(replay, original);
    retry.write_frame(&receipt_frame(&replay));
    assert_eq!(
        retry.finish(),
        [format!("RECEIVED {}", receipt_frame(&original))]
    );
    assert_eq!(effects(&fixture), committed);
}

fn effects(fixture: &ConfirmationFixture) -> (u32, i64, u64, usize, usize) {
    (
        fixture.stock(),
        fixture.ledger().minor_units(),
        fixture.next_invoice,
        fixture.posted_count(),
        fixture.posted.len(),
    )
}

#[test]
#[ignore = "subprocess worker dispatched by the confirmation restart test"]
fn client_worker() {
    let Ok(mode) = env::var(MODE) else { return };
    let dir = PathBuf::from(env::var_os(SCRATCH).expect("worker scratch path"));
    assert_eq!(dir.parent(), Some(target().as_path()));
    assert!(
        dir.file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with(PREFIX))
    );
    assert_eq!(dir.canonicalize().unwrap(), dir);

    let pending = dir.join(PENDING);
    let database = dir.join(DATABASE);
    let mut input = io::stdin().lock().lines();
    match mode.as_str() {
        "submit" => {
            let approved =
                OwnerRequest::decode(&input.next().expect("explicit Owner approval").unwrap());
            assert_eq!(
                fs::symlink_metadata(&database).unwrap_err().kind(),
                io::ErrorKind::NotFound
            );
            open_sales(&database)
                .session(Role::Owner)
                .save_draft(approved.review.draft().clone())
                .unwrap();
            let mut file = fs::File::create_new(&pending).expect("create isolated pending request");
            file.write_all(approved.encode().as_bytes()).unwrap();
            file.sync_all().unwrap();
            send(&approved);
            assert!(
                input.next().is_none(),
                "initial client must get EOF, not a receipt"
            );
            emit("REPLY_EOF");
        }
        "reopen" => {
            let pending = load_pending(&pending);
            assert!(fs::symlink_metadata(&database).unwrap().is_file());
            let reopened = open_sales(&database)
                .session(Role::Owner)
                .reopen_draft(pending.review.draft().id())
                .unwrap()
                .expect("reopen the saved SQLite draft");
            assert_eq!(&reopened, pending.review.draft());
            emit(&format!("REOPENED {}", draft_fields(&reopened)));
        }
        "retry" => {
            send(&load_pending(&pending));
            let reply = input.next().expect("explicit retry response").unwrap();
            let receipt = read_receipt(&reply).expect("expected an authority receipt");
            emit(&format!("RECEIVED {}", receipt_frame(&receipt)));
        }
        "sqlite_submit" | "sqlite_reopen" | "sqlite_manager" | "sqlite_retry" | "sqlite_fail" => {
            sqlite_pending::worker(mode.as_str(), &database, &mut input);
        }
        "sqlite_compete" => competing::worker(&database, &mut input),
        "sqlite_crate_reopen" => crate_checkout::reopen_worker(&database),
        "walk_in_submit" | "walk_in_reopen" | "walk_in_denied" | "walk_in_retry"
        | "walk_in_edit" | "walk_in_reject" => {
            walk_in::worker(mode.as_str(), &database, &mut input)
        }
        "stall" => {
            let _ = input.next();
        }
        _ => panic!("unknown confirmation restart mode: {mode}"),
    }
}

fn open_sales(database: &Path) -> LocalSales {
    LocalSales::open_sqlite(database, Money::from_minor_units(0)).expect("open synthetic SQLite")
}

fn load_pending(path: &Path) -> OwnerRequest {
    assert!(fs::symlink_metadata(path).unwrap().is_file());
    OwnerRequest::decode(&fs::read_to_string(path).expect("read persisted Owner request"))
}

pub(super) struct Scratch(pub(super) PathBuf);

impl Scratch {
    pub(super) fn new() -> Self {
        let target = target();
        fs::create_dir_all(&target).expect("create manifest target directory");
        assert_eq!(target.canonicalize().unwrap(), target);
        for sequence in 0_u64.. {
            let dir = target.join(format!("{PREFIX}{}-{sequence}", process::id()));
            match fs::create_dir(&dir) {
                Ok(()) => return Self(dir),
                Err(err) if err.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(err) => panic!("create synthetic scratch directory: {err}"),
            }
        }
        unreachable!("scratch sequence exhausted");
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn target() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .canonicalize()
        .unwrap()
        .join("target")
}
