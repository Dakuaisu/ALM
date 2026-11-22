use std::io::{self, Write};
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::process::{self, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{env, fs};

use alm_phase2_local::confirmation::{
    ConfirmationError, ConfirmationFixture, Receipt, RequestId, Review, StockLotRow, StockLots,
};
use alm_phase2_local::{
    BillDate, DraftCustomer, DraftCustomerId, DraftId, DraftLine, LocalSales, Money, ProductId,
    ProvisionalDraft, Role,
};

#[path = "owner_simulation/quantity_reconnect.rs"]
mod quantity_reconnect;

const DRAFT: DraftId = DraftId::new(1);
const PRODUCT: ProductId = ProductId::new(1);
const CUSTOMER: &str = "Synthetic Customer";
const CUSTOMER_ID: &str = "synthetic-owner-simulation-customer";
const ZERO: Money = Money::from_minor_units(0);
const RECEIVED: Money = Money::from_minor_units(20_000);

struct ScratchDir(PathBuf);

impl ScratchDir {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let target = Path::new(env!("CARGO_MANIFEST_DIR"))
            .canonicalize()
            .expect("resolve crate directory")
            .join("target");
        fs::create_dir_all(&target).expect("ensure crate target directory exists");
        assert_eq!(
            target.canonicalize().expect("resolve target directory"),
            target,
            "the disposable database must stay inside the crate target directory",
        );
        loop {
            let seq = NEXT.fetch_add(1, Ordering::Relaxed);
            let dir = target.join(format!("owner-simulation-{}-{seq}", process::id()));
            match fs::create_dir(&dir) {
                Ok(()) => return Self(dir),
                Err(err) if err.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(err) => panic!("create Owner simulation scratch directory: {err}"),
            }
        }
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        if let Err(err) = fs::remove_dir_all(&self.0) {
            eprintln!("remove Owner simulation scratch directory: {err}");
        }
    }
}

fn save_cli_draft(scratch: &ScratchDir) -> PathBuf {
    let db = scratch.0.join("synthetic-drafts.sqlite3");
    let mut child = Command::new(env!("CARGO_BIN_EXE_alm-draft-demo"))
        .arg("--db")
        .arg(&db)
        .env_clear()
        .env("PATH", env::var_os("PATH").unwrap_or_default())
        .env("HOME", &scratch.0)
        .env("USERPROFILE", &scratch.0)
        .current_dir(&scratch.0)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("launch the synthetic Manager CLI");
    let script_written = child
        .stdin
        .take()
        .expect("piped CLI stdin")
        .write_all(b"new\nSynthetic Customer\n1\n3\nsave\nquit\n");
    let output = child.wait_with_output().expect("wait for Manager CLI exit");
    assert!(output.status.success(), "Manager CLI failed: {output:?}");
    script_written.expect("send the synthetic draft script");
    db
}

fn reopen_as_manager(db: &Path) -> ProvisionalDraft {
    LocalSales::open_sqlite(db, ZERO)
        .expect("reopen the CLI's SQLite database")
        .session(Role::Manager)
        .reopen_draft(DRAFT)
        .expect("read the saved draft as Manager")
        .expect("CLI draft must exist")
}

#[derive(Debug, PartialEq, Eq)]
struct PostingEffects {
    lots: StockLots,
    posted_count: usize,
    sales_ledger: Money,
    received_ledger: Money,
    customer_due: Money,
    gross_profit: Result<Option<Money>, ConfirmationError>,
}

impl PostingEffects {
    fn observe(fixture: &ConfirmationFixture) -> Self {
        Self {
            lots: fixture.open_stock_lots(Role::Owner, PRODUCT).unwrap(),
            posted_count: fixture.posted_count(),
            sales_ledger: fixture.ledger(),
            received_ledger: fixture.received_ledger(),
            customer_due: fixture.customer_due(CUSTOMER_ID),
            gross_profit: fixture.gross_profit(Role::Owner, DRAFT),
        }
    }
}

#[derive(Default)]
struct LoseFirstReply {
    dropped_receipt: Option<Receipt>,
}

impl LoseFirstReply {
    fn confirm(
        &mut self,
        fixture: &mut ConfirmationFixture,
        request: RequestId,
        review: &Review,
    ) -> Result<Option<Receipt>, ConfirmationError> {
        let receipt = fixture.confirm(Role::Owner, request, review)?;
        if self.dropped_receipt.is_none() {
            self.dropped_receipt = Some(receipt);
            return Ok(None);
        }
        Ok(Some(receipt))
    }
}

#[test]
fn owner_simulation_posts_cli_draft_once_after_response_loss() {
    // Given: the real Manager demo creates the only draft in a fresh SQLite database.
    let scratch = ScratchDir::new();
    let db = save_cli_draft(&scratch);
    let saved = reopen_as_manager(&db);
    assert_eq!(saved.id(), DRAFT);
    let DraftCustomer::Typed(name) = saved.customer() else {
        panic!("the CLI draft must have a typed customer name");
    };
    assert_eq!(name, CUSTOMER);
    assert_eq!(
        saved.lines(),
        [DraftLine::new(PRODUCT, NonZeroU32::new(3).unwrap())],
    );
    assert_eq!(saved.bill_date(), None);
    assert_eq!(saved.crate_row(), None);
    println!("Owner simulation | Manager reopened provisional SQLite draft: {saved:?}");

    let detached = saved
        .clone()
        .with_bill_date(Some(BillDate::new(2025, 2, 21).unwrap()))
        .with_customer(DraftCustomer::Selected {
            id: DraftCustomerId::new(CUSTOMER_ID.into()),
            name: name.clone(),
        });
    let initial_rate = Money::from_minor_units(11_800);
    let fresh_rate = Money::from_minor_units(14_160);
    let older_cost = Money::from_minor_units(6_000);
    let newer_cost = Money::from_minor_units(7_000);
    let mut fixture = ConfirmationFixture::with_ordered_lots(
        PRODUCT,
        &[(2, older_cost), (5, newer_cost)],
        initial_rate,
    )
    .unwrap();
    let before = PostingEffects::observe(&fixture);
    assert_eq!(before.lots.on_hand, 7);
    assert_eq!(
        before.lots.total_value,
        Some(Money::from_minor_units(47_000))
    );
    assert_eq!(before.posted_count, 0);
    assert_eq!(before.sales_ledger, ZERO);
    assert_eq!(before.received_ledger, ZERO);
    assert_eq!(before.customer_due, ZERO);
    assert_eq!(before.gross_profit, Err(ConfirmationError::DraftNotFound));
    fixture.register_draft(detached).unwrap();
    let id = saved.id();
    let initial = fixture.review_named(Role::Owner, id, RECEIVED).unwrap();
    assert_eq!(initial.unit_prices(), [initial_rate]);
    assert_eq!(
        initial.amounts().unwrap().total,
        Money::from_minor_units(35_400)
    );
    assert_eq!(PostingEffects::observe(&fixture), before);

    // When: terms change and only an explicit Owner confirm can apply the refreshed review.
    let request = RequestId::new(1);
    fixture.set_product_price(PRODUCT, fresh_rate).unwrap();
    let stale = fixture.confirm(Role::Owner, request, &initial);
    assert_eq!(stale, Err(ConfirmationError::StaleReview));
    assert_eq!(PostingEffects::observe(&fixture), before);
    let fresh = fixture.review_named(Role::Owner, id, RECEIVED).unwrap();
    assert_eq!(fresh.draft(), initial.draft());
    assert_eq!(fresh.unit_prices(), [fresh_rate]);
    let total = Money::from_minor_units(42_480);
    assert_eq!(fresh.amounts().unwrap().total, total);
    println!(
        "Owner simulation | Unit rate (minor units): {} -> {}; stale confirm: {stale:?}",
        initial.unit_price().minor_units(),
        fresh.unit_price().minor_units(),
    );
    assert_eq!(PostingEffects::observe(&fixture), before);
    let manager = fixture.confirm(Role::Manager, request, &fresh);
    assert_eq!(manager, Err(ConfirmationError::AccessDenied));
    assert_eq!(PostingEffects::observe(&fixture), before);
    println!("Owner simulation | Manager confirm: {manager:?}; pre-confirm effects: {before:?}");

    let mut transport = LoseFirstReply::default();
    let lost = transport.confirm(&mut fixture, request, &fresh).unwrap();
    assert_eq!(lost, None);

    // Then: the sale is already applied when the reply is lost; replay must not apply it again.
    let applied = PostingEffects::observe(&fixture);
    let due = Money::from_minor_units(22_480);
    assert_eq!(
        applied,
        PostingEffects {
            lots: StockLots {
                on_hand: 4,
                rows: vec![StockLotRow {
                    date: None,
                    remaining: NonZeroU32::new(4).unwrap(),
                    unit_cost: Some(newer_cost),
                    value: Some(Money::from_minor_units(28_000)),
                }],
                total_value: Some(Money::from_minor_units(28_000)),
            },
            posted_count: 1,
            sales_ledger: total,
            received_ledger: RECEIVED,
            customer_due: due,
            gross_profit: Ok(Some(Money::from_minor_units(23_480))),
        },
    );
    println!("Owner simulation | Reply after apply: {lost:?}; applied effects: {applied:?}");

    let retry = transport.confirm(&mut fixture, request, &fresh).unwrap();
    assert_eq!(retry, transport.dropped_receipt);
    assert_eq!(
        retry,
        Some(Receipt {
            draft_id: id,
            invoice: 1,
            quantity: 3,
            total,
            received: Some(RECEIVED),
            due: Some(due),
            customer_balance: Some(due),
        }),
    );
    assert_eq!(PostingEffects::observe(&fixture), applied);
    println!("Owner simulation | Identical retry returned original simulated receipt: {retry:?}");

    let changed = fixture
        .review_named(Role::Owner, id, Money::from_minor_units(20_001))
        .unwrap();
    let conflict = transport.confirm(&mut fixture, request, &changed);
    assert_eq!(conflict, Err(ConfirmationError::RequestConflict));
    assert_eq!(PostingEffects::observe(&fixture), applied);
    println!("Owner simulation | Conflicting request reuse: {conflict:?}");

    let reopened = reopen_as_manager(&db);
    assert_eq!(reopened, saved);
    println!("Owner simulation | Manager SQLite draft unchanged: {reopened:?}");
}
