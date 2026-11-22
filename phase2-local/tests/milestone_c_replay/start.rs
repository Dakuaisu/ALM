use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{fs, io, process};

use alm_phase2_local::confirmation::{
    ConfirmationError, ConfirmationFixture, Counter, ProposedPayment, Receipt, RequestId, Review,
    StockLots,
};
use alm_phase2_local::{DraftCustomerId, DraftLine, LocalSales, Money, ProvisionalDraft};

use super::accounts::{AccountCheckpoint, AccountReadback, assert_accounts};
use super::inventory::{StockCheckpoint, assert_stock};
use super::{A, B, BUYER, DENIED, DraftCustomer, MANAGER, OWNER, PAID, SALE, feb, money, qty};

pub(super) const INITIAL_RATE: Money = money(9_440);
const POSTING_RATE: Money = money(11_800);

pub(super) fn post_sale(
    fixture: &mut ConfirmationFixture,
    buyer: &DraftCustomerId,
) -> (Review, Receipt) {
    let before = PostingEffects::read(fixture, buyer);
    let scratch = DraftDirectory::create();
    let db = scratch.0.join("synthetic-drafts.sqlite3");
    let bill = ProvisionalDraft::new(SALE, vec![DraftLine::new(A, qty(2))])
        .with_bill_date(Some(feb(21)))
        .with_customer(DraftCustomer::Selected {
            id: buyer.clone(),
            name: BUYER.into(),
        });
    {
        let mut writer = LocalSales::open_sqlite(&db, money(0)).expect("open Manager draft store");
        writer
            .session(MANAGER)
            .save_draft(bill.clone())
            .expect("save the Milestone C draft");
    }
    let reopened = reopen_as_manager(&db);
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
    assert_eq!(reopen_as_manager(&db), bill);
    (fresh, posted)
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

fn reopen_as_manager(db: &Path) -> ProvisionalDraft {
    LocalSales::open_sqlite(db, money(0))
        .expect("restart Manager draft store")
        .session(MANAGER)
        .reopen_draft(SALE)
        .expect("read SQLite draft")
        .expect("Milestone C draft survived restart")
}

struct DraftDirectory(PathBuf);

impl DraftDirectory {
    fn create() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let target = Path::new(env!("CARGO_MANIFEST_DIR"))
            .canonicalize()
            .expect("resolve crate directory")
            .join("target");
        fs::create_dir_all(&target).expect("create crate target directory");
        assert_eq!(target.canonicalize().expect("resolve crate target"), target);
        loop {
            let seq = NEXT.fetch_add(1, Ordering::Relaxed);
            let path = target.join(format!("milestone-c-replay-{}-{seq}", process::id()));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(err) if err.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(err) => panic!("create Milestone C draft directory: {err}"),
            }
        }
    }
}

impl Drop for DraftDirectory {
    fn drop(&mut self) {
        if let Err(err) = fs::remove_dir_all(&self.0) {
            eprintln!("remove Milestone C draft directory: {err}");
        }
    }
}
