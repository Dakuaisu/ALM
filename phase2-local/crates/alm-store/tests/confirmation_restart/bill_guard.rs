use super::*;
use alm_core::confirmation::ProductTerms;
use alm_core::{CrateRow, DraftCustomerId, SaleUnit};
use std::collections::BTreeMap;

const ID: DraftId = DraftId::new(1);
const PRODUCT: ProductId = ProductId::new(1);
const REQUEST: RequestId = RequestId::new(19);
const RECEIVED: Money = Money::from_minor_units(40);
const PRICE: Money = Money::from_minor_units(125);

fn customer() -> DraftCustomer {
    DraftCustomer::Selected {
        id: DraftCustomerId::new("buyer".into()),
        name: "Synthetic".into(),
    }
}

fn fixture_with_checkout() -> ConfirmationFixture {
    let mut fixture = ConfirmationFixture::with_ordered_lots(
        PRODUCT,
        &[
            (1, Money::from_minor_units(80)),
            (4, Money::from_minor_units(20)),
        ],
        PRICE,
    )
    .unwrap();
    fixture
        .set_checkout_product(
            PRODUCT,
            ProductTerms {
                pack: NonZeroU32::MIN,
                piece_price: PRICE,
                case_price: PRICE,
            },
        )
        .unwrap();
    fixture
}

fn assert_unposted(fixture: &ConfirmationFixture) {
    assert_eq!(fixture.stock(), 5);
    assert_eq!(fixture.catalog().lot_snapshot(PRODUCT), [1, 4]);
    assert!(fixture.catalog().allocation_snapshot().is_empty());
    assert_eq!(fixture.catalog().allocated_cost(ID), None);
    assert_eq!(fixture.next_invoice(), 1);
    assert_eq!(fixture.ledger().minor_units(), 0);
    assert_eq!(fixture.received_ledger().minor_units(), 0);
    assert!(fixture.customer_balances().is_empty());
    assert!(fixture.requests().is_empty());
    assert!(fixture.posted().is_empty());
}

#[test]
fn l03_03_sqlite_reopens_incomplete_and_corrected_draft_before_one_owner_post() {
    let scratch = Scratch::new();
    let db = scratch.0.join("bill-guard.sqlite3");
    let open = || LocalSales::open_sqlite(&db, Money::from_minor_units(0)).unwrap();
    let save = |draft| open().session(Role::Manager).save_draft(draft).unwrap();
    let reopen = || {
        open()
            .session(Role::Manager)
            .reopen_draft(ID)
            .unwrap()
            .unwrap()
    };
    let mut incomplete = ProvisionalDraft::new(ID, Vec::new());
    incomplete.add_crate_row(CrateRow::new(NonZeroU32::MIN, vec![]));
    save(incomplete.clone());
    let reopened = reopen();
    assert_eq!(reopened, incomplete);
    assert_eq!(reopened.bill_date(), None);

    let mut fixture = fixture_with_checkout();
    fixture.register_draft(reopened).unwrap();
    assert_eq!(
        fixture.review_checkout(Role::Owner, ID, &[], RECEIVED),
        Err(ConfirmationError::MissingBillableItems)
    );
    assert_unposted(&fixture);
    assert!(fixture.checkout_approvals().is_empty());
    assert_eq!(reopen(), incomplete);
    let pending = open()
        .session(Role::Owner)
        .retry_pending(REQUEST, |_, _| ());
    assert!(matches!(
        pending,
        Err(alm_core::confirmation::PendingError::NotFound)
    ));

    let corrected = draft(1, 2).with_customer(customer());
    save(corrected.clone());
    let corrected_reopened = reopen();
    assert_eq!(corrected_reopened, corrected);
    fixture.register_draft(corrected_reopened).unwrap();
    let units = &[SaleUnit::Case];
    assert_eq!(
        fixture.review_checkout(Role::Manager, ID, units, RECEIVED),
        Err(ConfirmationError::AccessDenied)
    );
    let review = fixture
        .review_checkout(Role::Owner, ID, units, RECEIVED)
        .unwrap();
    let approvals = BTreeMap::from([(ID, review.clone())]);
    assert_eq!(
        fixture.confirm(Role::Manager, REQUEST, &review),
        Err(ConfirmationError::AccessDenied)
    );
    assert_unposted(&fixture);
    assert_eq!(fixture.checkout_approvals(), &approvals);

    let receipt = fixture.confirm(Role::Owner, REQUEST, &review).unwrap();
    assert_eq!(
        receipt,
        Receipt {
            draft_id: ID,
            invoice: 1,
            quantity: 2,
            total: Money::from_minor_units(250),
            received: Some(RECEIVED),
            due: Some(Money::from_minor_units(210)),
            customer_balance: Some(Money::from_minor_units(210)),
        }
    );
    assert_eq!(fixture.confirm(Role::Owner, REQUEST, &review), Ok(receipt));
    assert_eq!(
        fixture.confirm(Role::Owner, RequestId::new(20), &review),
        Err(ConfirmationError::DraftAlreadyPosted)
    );
    assert_eq!(fixture.stock(), 3);
    assert_eq!(fixture.catalog().lot_snapshot(PRODUCT), [0, 3]);
    assert_eq!(
        fixture.catalog().allocation_snapshot(),
        [(1, 0, 0, 1, Some(80)), (1, 0, 1, 1, Some(20))]
    );
    assert_eq!(fixture.catalog().allocated_cost(ID), Some(Some(100)));
    assert_eq!(fixture.next_invoice(), 2);
    assert_eq!(fixture.ledger().minor_units(), 250);
    assert_eq!(fixture.received_ledger().minor_units(), 40);
    assert_eq!(fixture.customer_due("buyer"), Money::from_minor_units(210));
    assert_eq!(fixture.posted_count(), 1);
    assert_eq!(fixture.posted(), &BTreeMap::from([(ID, REQUEST)]));
    assert_eq!(fixture.checkout_approvals(), &approvals);
}
