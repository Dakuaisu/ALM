use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

mod scratch_retry {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../alm-store/tests/common/scratch_retry.rs"
    ));
}
use std::{env, fs};

use alm_core::confirmation::{
    ConfirmationError, ConfirmationFixture, Counter, ProposedPayment, Receipt, RequestId, Review,
    StockLots,
};
use alm_core::{DraftCustomerId, DraftLine, LocalSales, Money, ProvisionalDraft};

use super::accounts::{AccountCheckpoint, AccountReadback, assert_accounts};
use super::inventory::{StockCheckpoint, assert_stock};
use super::{A, B, BUYER, DENIED, DraftCustomer, MANAGER, OWNER, PAID, SALE, feb, money, qty};
use alm_store::SqliteSales;

pub(super) const INITIAL_RATE: Money = money(9_440);
const POSTING_RATE: Money = money(11_800);
const ACCOUNT: &str = "milestone-c-manager";

pub(super) fn post_sale(
    fixture: &mut ConfirmationFixture,
    buyer: &DraftCustomerId,
) -> (Review, Receipt, DraftGuard) {
    let before = PostingEffects::read(fixture, buyer);
    let scratch = DraftDirectory::create();
    let bill = ProvisionalDraft::new(SALE, vec![DraftLine::new(A, qty(2))])
        .with_bill_date(Some(feb(21)))
        .with_customer(DraftCustomer::Selected {
            id: buyer.clone(),
            name: BUYER.into(),
        });
    let seed = ProvisionalDraft::new(SALE, vec![DraftLine::new(A, qty(1))])
        .with_bill_date(Some(feb(21)))
        .with_customer(DraftCustomer::Selected {
            id: buyer.clone(),
            name: BUYER.into(),
        });
    LocalSales::open_manager_account(&scratch.0, ACCOUNT)
        .expect("open scoped Manager draft store")
        .save_draft(seed)
        .expect("seed the Milestone C draft");
    let edited = scratch.run_cli("open\n21\nedit\n1\n2\nreview\nsave\nquit\n");
    assert_transcript(
        &edited,
        &[
            "Opened draft 21: SAVED.",
            "Edited draft 21: UNSAVED.",
            "PROVISIONAL — NOT POSTED",
            "Draft ID: 21",
            "State: UNSAVED",
            "Customer: Synthetic Buyer",
            "Line 1: Item: Synthetic Lime | Quantity: 2",
            "Saved draft 21: PROVISIONAL — NOT POSTED.",
        ],
    );
    let reopened_cli = scratch.run_cli("open\n21\nreview\nquit\n");
    assert_transcript(
        &reopened_cli,
        &[
            "Opened draft 21: SAVED.",
            "PROVISIONAL — NOT POSTED",
            "Draft ID: 21",
            "State: SAVED",
            "Customer: Synthetic Buyer",
            "Line 1: Item: Synthetic Lime | Quantity: 2",
        ],
    );
    let reopened = reopen_as_manager(&scratch.0);
    assert_eq!(reopened, bill);
    fixture
        .register_draft(reopened)
        .expect("use the SQLite draft");
    assert_eq!(fixture.review_named(MANAGER, SALE, PAID), Err(DENIED));
    let initial = fixture.review_named(OWNER, SALE, PAID).unwrap();
    assert_eq!(initial.unit_prices(), [INITIAL_RATE]);
    assert_eq!(initial.amounts().unwrap().total, money(18_880));
    assert_eq!(PostingEffects::read(fixture, buyer), before);

    let request = RequestId::new(1);
    fixture.set_product_price(A, POSTING_RATE).unwrap();
    assert_eq!(
        fixture.confirm(OWNER, request, &initial),
        Err(ConfirmationError::StaleReview)
    );
    assert_eq!(PostingEffects::read(fixture, buyer), before);

    let fresh = fixture.review_named(OWNER, SALE, PAID).unwrap();
    assert_eq!(fresh.draft(), initial.draft());
    let amounts = fresh.amounts().unwrap();
    assert_eq!((amounts.packets, amounts.total), (2, money(23_600)));
    assert_eq!(fresh.unit_prices(), [POSTING_RATE]);
    assert_eq!(fresh.payment(), ProposedPayment::Named(PAID));
    assert_eq!(
        assert_accounts(fixture, buyer, AccountCheckpoint::BeforePosting),
        before.accounts
    );
    let denied = fixture.confirm(MANAGER, request, &fresh);
    assert_eq!(denied, Err(DENIED));
    assert_eq!(PostingEffects::read(fixture, buyer), before);
    assert_eq!(before.posted_count, 0);
    assert_eq!(before.sale, Err(ConfirmationError::DraftNotFound));
    assert_stock(fixture, StockCheckpoint::Received);
    println!(
        "Feb 21 review total={}; Manager confirm={denied:?}; posted={}",
        amounts.total.minor_units(),
        before.posted_count
    );

    // Model a reply lost after the authority applies the request, before the caller sees it.
    let _ = fixture
        .confirm(OWNER, request, &fresh)
        .expect("apply before reply loss");
    let applied = PostingEffects::read(fixture, buyer);
    assert_eq!(applied.posted_count, 1);
    let (actual_review, original) = applied.sale.as_ref().expect("sale posted before retry");
    assert_eq!(actual_review, &fresh);

    let posted = fixture
        .confirm(OWNER, request, &fresh)
        .expect("retry exact review");
    assert_eq!(&posted, original);
    assert_eq!(
        PostingEffects::read(fixture, buyer),
        applied,
        "identical retry"
    );

    let changed = fixture.review_named(OWNER, SALE, money(10_001)).unwrap();
    assert_eq!(
        fixture.confirm(OWNER, request, &changed),
        Err(ConfirmationError::RequestConflict)
    );
    assert_eq!(
        PostingEffects::read(fixture, buyer),
        applied,
        "conflicting reuse"
    );
    assert_eq!(reopen_as_manager(&scratch.0), bill);
    (fresh, posted, DraftGuard { scratch, bill })
}

#[derive(Debug, Eq, PartialEq)]
struct PostingEffects {
    lots: [StockLots; 2],
    accounts: AccountReadback,
    ledger: Money,
    received: Money,
    gross_profit: Result<Option<Money>, ConfirmationError>,
    posted_count: usize,
    sale: Result<(Review, Receipt), ConfirmationError>,
    counter: Counter,
}

impl PostingEffects {
    fn read(fixture: &ConfirmationFixture, buyer: &DraftCustomerId) -> Self {
        Self {
            lots: [A, B].map(|product| fixture.open_stock_lots(OWNER, product).unwrap()),
            accounts: AccountReadback::open(fixture, buyer),
            ledger: fixture.ledger(),
            received: fixture.received_ledger(),
            gross_profit: fixture.gross_profit(OWNER, SALE),
            posted_count: fixture.posted_count(),
            sale: fixture
                .current_sale(OWNER, SALE)
                .map(|(review, receipt)| (review.clone(), receipt.clone())),
            counter: fixture.counter(OWNER, feb(21)).unwrap(),
        }
    }
}

fn reopen_as_manager(root: &Path) -> ProvisionalDraft {
    LocalSales::open_manager_account(root, ACCOUNT)
        .expect("restart Manager draft store")
        .reopen_draft(SALE)
        .expect("read SQLite draft")
        .expect("Milestone C draft survived restart")
}

pub(super) struct DraftGuard {
    scratch: DraftDirectory,
    bill: ProvisionalDraft,
}

impl DraftGuard {
    pub(super) fn assert_unchanged(&self) {
        assert_eq!(reopen_as_manager(&self.scratch.0), self.bill);
    }
}

fn assert_transcript(stdout: &str, expected: &[&str]) {
    let mut lines = stdout.lines().map(str::trim);
    for expected_line in expected {
        assert!(
            lines.any(|line| line == *expected_line),
            "missing {expected_line:?} in CLI transcript:\n{stdout}"
        );
    }
}

struct DraftDirectory(PathBuf);

impl DraftDirectory {
    fn run_cli(&self, script: &str) -> String {
        let mut child = Command::new(env!("CARGO_BIN_EXE_alm-draft-demo"))
            .env_clear()
            .env("PATH", env::var_os("PATH").unwrap_or_default())
            .env("HOME", &self.0)
            .env("USERPROFILE", &self.0)
            .current_dir(&self.0)
            .arg("--device-root")
            .arg(&self.0)
            .arg("--account")
            .arg(ACCOUNT)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("launch scoped Manager CLI");
        child
            .stdin
            .take()
            .expect("CLI stdin")
            .write_all(script.as_bytes())
            .expect("send CLI script");
        let output = child.wait_with_output().expect("wait for Manager CLI");
        let stdout = String::from_utf8_lossy(&output.stdout);
        println!("Manager CLI script:\n{script}Manager CLI transcript:\n{stdout}");
        assert!(
            output.status.success() && output.stderr.is_empty(),
            "CLI failed: {output:?}"
        );
        String::from_utf8(output.stdout).expect("CLI output is UTF-8")
    }

    fn create() -> Self {
        let target = Path::new(env!("CARGO_MANIFEST_DIR"))
            .canonicalize()
            .expect("resolve crate directory")
            .join("target");
        fs::create_dir_all(&target).expect("create crate target directory");
        assert_eq!(target.canonicalize().expect("resolve crate target"), target);
        Self(
            scratch_retry::create(&target, "milestone-c-replay-")
                .expect("create Milestone C draft directory"),
        )
    }
}

impl Drop for DraftDirectory {
    fn drop(&mut self) {
        if let Err(err) = fs::remove_dir_all(&self.0) {
            eprintln!("remove Milestone C draft directory: {err}");
        }
    }
}
