use std::path::Path;

use alm_core::confirmation::{ConfirmationError, ConfirmationFixture, Receipt, RequestId};
use alm_core::{
    AccessDenied, BillDate, DraftCustomer, DraftId, Money, ProductId, ProvisionalDraft, Role,
};

use super::{PROFIT, ScratchDir, draft, open_sales, run_sql, run_worker};

const SAVED_ID: DraftId = DraftId::new(51);
const PRODUCT: u64 = 7;
const QUANTITY: u32 = 3;
const SAVED_LINES: &[(u64, u32)] = &[(PRODUCT, QUANTITY)];
const STOCK: u32 = 10;
const OLD_PRICE: Money = Money::from_minor_units(125);
const CURRENT_PRICE: Money = Money::from_minor_units(240);

fn saved_draft() -> ProvisionalDraft {
    draft(SAVED_ID, SAVED_LINES)
        .with_customer(DraftCustomer::Typed("Synthetic".into()))
        .with_bill_date(Some(BillDate::new(2024, 2, 29).unwrap()))
}

#[test]
fn saved_draft_reconnects_without_posting_until_owner_confirms_current_price() {
    // Given: a writer that commits a one-line draft and exits.
    let scratch = ScratchDir::new();
    let path = scratch.db_path();
    run_worker("write_reconnect", &path);

    // When: a fresh process reopens the draft, reviews it and confirms separately.
    run_worker("read_reconnect", &path);

    // Then: even after confirmation, SQLite still holds only the unchanged draft.
    let tables = run_sql(
        &path,
        true,
        "SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name;",
    );
    assert_eq!(
        tables.lines().collect::<Vec<_>>(),
        [
            "pending_owner_confirmations",
            "provisional_draft_crates",
            "provisional_draft_customers",
            "provisional_draft_dates",
            "provisional_draft_lines",
            "provisional_draft_loading",
            "provisional_drafts"
        ],
    );
    let persisted = open_sales(&path)
        .session(Role::Manager)
        .reopen_draft(SAVED_ID)
        .expect("reopen persisted draft after confirmation");
    assert_eq!(persisted, Some(saved_draft()));
}

pub(super) fn write_saved_draft(path: &Path) {
    assert!(!path.try_exists().expect("check fresh database path"));
    open_sales(path)
        .session(Role::Manager)
        .save_draft(saved_draft())
        .expect("save one-line reconnect draft");
}

pub(super) fn reopen_and_confirm(path: &Path) {
    let mut sales = open_sales(path);
    assert_eq!(sales.session(Role::Owner).total_profit(), Ok(PROFIT));
    let manager = sales.session(Role::Manager);
    assert_eq!(manager.total_profit(), Err(AccessDenied));
    let saved = manager
        .reopen_draft(SAVED_ID)
        .expect("read the database after writer exit")
        .expect("writer saved the draft");
    assert_eq!(saved, saved_draft());

    let mut fixture = ConfirmationFixture::new(ProductId::new(PRODUCT), STOCK, OLD_PRICE)
        .expect("create local confirmation fixture");
    assert_eq!(
        fixture.reconnect_review(Role::Manager, saved.clone()),
        Err(ConfirmationError::AccessDenied),
    );
    assert_eq!(
        fixture.review(Role::Owner, SAVED_ID),
        Err(ConfirmationError::DraftNotFound),
        "denied reconnect must not register the draft",
    );
    assert_unposted(&fixture);

    let old_review = fixture
        .reconnect_review(Role::Owner, saved.clone())
        .expect("review reopened draft without posting");
    assert_eq!(old_review.draft(), &saved);
    assert_eq!(old_review.unit_price(), OLD_PRICE);
    assert_eq!(
        fixture.reconnect_review(Role::Owner, saved.clone()),
        Ok(old_review.clone()),
    );
    assert_unposted(&fixture);

    fixture
        .set_unit_price(CURRENT_PRICE)
        .expect("change fixture price");
    assert_eq!(
        fixture.confirm(Role::Owner, RequestId::new(1), &old_review),
        Err(ConfirmationError::StaleReview),
    );
    assert_unposted(&fixture);

    let fresh_review = fixture
        .reconnect_review(Role::Owner, saved.clone())
        .expect("review current price before confirming");
    assert_eq!(fresh_review.draft(), &saved);
    assert_eq!(fresh_review.unit_price(), CURRENT_PRICE);
    assert_unposted(&fixture);

    assert_eq!(
        fixture.confirm(Role::Manager, RequestId::new(2), &fresh_review),
        Err(ConfirmationError::AccessDenied),
    );
    assert_unposted(&fixture);

    let receipt = fixture
        .confirm(Role::Owner, RequestId::new(3), &fresh_review)
        .expect("Owner explicitly confirms the current review");
    assert_eq!(
        receipt,
        Receipt {
            draft_id: SAVED_ID,
            invoice: 1,
            quantity: QUANTITY,
            total: Money::from_minor_units(720),
            received: None,
            due: None,
            customer_balance: None,
        },
    );
    assert_eq!(fixture.posted_count(), 1);
    assert_eq!(fixture.stock(), 7);
    assert_eq!(fixture.ledger(), Money::from_minor_units(720));
    assert_eq!(manager.total_profit(), Err(AccessDenied));
}

fn assert_unposted(fixture: &ConfirmationFixture) {
    assert_eq!(fixture.posted_count(), 0);
    assert_eq!(fixture.stock(), STOCK);
    assert_eq!(fixture.ledger(), Money::from_minor_units(0));
}
