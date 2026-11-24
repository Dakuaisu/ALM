use super::*;
use alm_core::confirmation::{CheckoutAmounts, ProductTerms, Review};
use alm_core::{
    CrateComponent, CrateRow, DraftCustomer, DraftCustomerId, DraftLine, ProvisionalDraft, SaleUnit,
};
use std::num::NonZeroU32;

mod guards;
mod sqlite_records;

const ID: DraftId = DraftId::new(43);
const REQUEST: RequestId = RequestId::new(987);
const PIECE: ProductId = ProductId::new(2);
const CASE: ProductId = ProductId::new(1);
const fn money(value: i64) -> Money {
    Money::from_minor_units(value)
}

fn qty(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).unwrap()
}

fn components() -> [CrateComponent; 2] {
    [
        CrateComponent::new(PIECE, qty(9), SaleUnit::Piece),
        CrateComponent::new(CASE, qty(2), SaleUnit::Case),
    ]
}

fn order(row: CrateRow, lines: Vec<DraftLine>) -> ProvisionalDraft {
    let mut draft = ProvisionalDraft::new(ID, lines)
        .with_customer(DraftCustomer::Selected {
            id: DraftCustomerId::new("crate-buyer".into()),
            name: "Synthetic".into(),
        })
        .with_bill_date(test_bill_date());
    draft.add_crate_row(row);
    draft
}

fn distinct_order() -> ProvisionalDraft {
    order(CrateRow::new(qty(2), components().to_vec()), vec![])
}

pub(super) fn reopen_worker(database: &Path) {
    assert_eq!(
        open_sales(database)
            .session(Role::Manager)
            .reopen_draft(ID)
            .unwrap(),
        Some(distinct_order())
    );
    emit("REOPENED_CRATE");
}

fn authority(short: bool, order: ProvisionalDraft) -> ConfirmationFixture {
    let mut authority =
        ConfirmationFixture::with_ordered_lots(PIECE, &[(2, money(80)), (4, money(20))], money(17))
            .unwrap();
    authority.add_product(CASE, 0, money(150)).unwrap();
    let later = if short { 0 } else { 4 };
    authority
        .catalog_mut()
        .seed_lots(CASE, &[(3, money(30)), (later, money(40))])
        .unwrap();
    for product in [PIECE, CASE] {
        authority
            .set_checkout_product(
                product,
                ProductTerms {
                    pack: qty(6),
                    piece_price: money(17),
                    case_price: money(150),
                },
            )
            .unwrap();
    }
    authority.set_crate_rate(money(450)).unwrap();
    authority.set_loading_rate(money(13)).unwrap();
    authority.register_draft(order).unwrap();
    authority
}

fn approval(authority: &mut ConfirmationFixture) -> Review {
    authority
        .review_checkout(Role::Owner, ID, &[], money(100))
        .unwrap()
}

fn effects(authority: &ConfirmationFixture) -> (i64, i64, i64, u64, usize, usize) {
    (
        authority.ledger().minor_units(),
        authority.received_ledger().minor_units(),
        authority.customer_due("crate-buyer").minor_units(),
        authority.next_invoice(),
        authority.requests().len(),
        authority.posted().len(),
    )
}

#[test]
fn sole_crate_persists_and_posts_once_after_lost_response() {
    let scratch = Scratch::new();
    let database = scratch.0.join(DATABASE);
    let draft = distinct_order();
    let mut authority = authority(false, draft.clone());
    open_sales(&database)
        .session(Role::Manager)
        .save_draft(draft.clone())
        .unwrap();
    assert_eq!(
        Client::start("sqlite_crate_reopen", &scratch.0).finish(),
        ["REOPENED_CRATE"]
    );
    let reopened = open_sales(&database)
        .session(Role::Owner)
        .reopen_draft(ID)
        .unwrap()
        .unwrap();
    assert_eq!(reopened, draft);
    assert_eq!(authority.catalog().lot_snapshot(PIECE), vec![2, 4]);
    assert_eq!(authority.catalog().lot_snapshot(CASE), vec![3, 4]);
    assert!(authority.catalog().allocation_snapshot().is_empty());
    assert_eq!(effects(&authority), (0, 0, 0, 1, 0, 0));
    let review = approval(&mut authority);
    assert_eq!(review.unit_prices(), &[money(450)]);
    assert_eq!(
        review.crate_packet_demands().unwrap().unwrap(),
        vec![DraftLine::new(PIECE, qty(3)), DraftLine::new(CASE, qty(4))]
    );
    assert_eq!(
        review.amounts().unwrap(),
        CheckoutAmounts {
            packets: 7,
            subtotal: money(900),
            loading: money(91),
            total: money(991),
        }
    );
    let approved = OwnerRequest {
        id: REQUEST,
        review,
    };
    let mut first = Client::start("sqlite_submit", &scratch.0);
    first.write_frame(&approved.encode());
    let sent = first.sent();
    assert_eq!(sent, approved);
    assert_eq!(
        authority.confirm(Role::Manager, sent.id, &sent.review),
        Err(ConfirmationError::AccessDenied)
    );
    let receipt = authority
        .confirm(Role::Owner, sent.id, &sent.review)
        .unwrap();
    assert_eq!(
        receipt,
        Receipt {
            draft_id: ID,
            invoice: 1,
            quantity: 7,
            total: money(991),
            received: Some(money(100)),
            due: Some(money(891)),
            customer_balance: Some(money(891)),
        }
    );
    sqlite_records::assert_cost_free_records(&database);
    first.write_frame("CONTINUE");
    assert_eq!(first.finish(), ["REPLY_EOF"]);
    let allocations = vec![
        (43, 0, 0, 2, Some(160)),
        (43, 0, 1, 1, Some(20)),
        (43, 1, 0, 3, Some(90)),
        (43, 1, 1, 1, Some(40)),
    ];
    assert_eq!(authority.catalog().allocation_snapshot(), allocations);
    assert_eq!(authority.catalog().allocated_cost(ID), Some(Some(310)));
    authority.set_crate_rate(money(999)).unwrap();
    assert_eq!(
        Client::start("sqlite_reopen", &scratch.0).finish(),
        ["REOPENED"]
    );
    assert_eq!(
        Client::start("sqlite_manager", &scratch.0).finish(),
        ["DENIED"]
    );
    let mut retry = Client::start("sqlite_retry", &scratch.0);
    let resent = retry.sent();
    assert_eq!(resent, approved);
    assert_eq!(
        authority.confirm(Role::Manager, resent.id, &resent.review),
        Err(ConfirmationError::AccessDenied)
    );
    let replay = authority
        .confirm(Role::Owner, resent.id, &resent.review)
        .unwrap();
    assert_eq!(replay, receipt);
    retry.write_frame(&receipt_frame(&replay));
    assert_eq!(
        retry.finish(),
        [format!("RECEIVED {}", receipt_frame(&receipt))]
    );
    assert_eq!(authority.catalog().lot_snapshot(PIECE), vec![0, 3]);
    assert_eq!(authority.catalog().lot_snapshot(CASE), vec![0, 3]);
    assert_eq!(authority.catalog().allocation_snapshot(), allocations);
    assert_eq!(effects(&authority), (991, 100, 891, 2, 1, 1));
}

#[test]
fn last_crate_component_shortage_rolls_back_all_effects() {
    let mut authority = authority(true, distinct_order());
    let review = approval(&mut authority);
    assert_eq!(
        authority.confirm(Role::Owner, REQUEST, &review),
        Err(ConfirmationError::InsufficientStock)
    );
    assert_eq!(authority.catalog().lot_snapshot(PIECE), vec![2, 4]);
    assert_eq!(authority.catalog().lot_snapshot(CASE), vec![3, 0]);
    assert!(authority.catalog().allocation_snapshot().is_empty());
    assert_eq!(authority.catalog().allocated_cost(ID), None);
    assert_eq!(effects(&authority), (0, 0, 0, 1, 0, 0));
}

#[test]
fn repeated_crate_products_round_trip_and_consume_the_same_lot_once_per_packet() {
    let scratch = Scratch::new();
    let database = scratch.0.join(DATABASE);
    let draft = order(
        CrateRow::new(
            qty(1),
            vec![
                CrateComponent::new(PIECE, qty(1), SaleUnit::Case),
                CrateComponent::new(PIECE, qty(1), SaleUnit::Case),
            ],
        ),
        vec![],
    );
    open_sales(&database)
        .session(Role::Manager)
        .save_draft(draft.clone())
        .unwrap();
    let reopened = open_sales(&database)
        .session(Role::Manager)
        .reopen_draft(ID)
        .unwrap()
        .unwrap();
    assert_eq!(reopened, draft);
    let mut authority = authority(false, reopened);
    let review = authority
        .review_checkout(Role::Owner, ID, &[], money(0))
        .unwrap();
    assert_eq!(effects(&authority), (0, 0, 0, 1, 0, 0));
    assert_eq!(
        review.crate_packet_demands().unwrap().unwrap(),
        [DraftLine::new(PIECE, qty(1)); 2]
    );
    let receipt = authority.confirm(Role::Owner, REQUEST, &review).unwrap();
    assert_eq!((receipt.quantity, receipt.total), (2, money(476)));
    assert_eq!(authority.catalog().lot_snapshot(PIECE), vec![0, 4]);
    assert_eq!(authority.catalog().allocated_cost(ID), Some(Some(160)));
    assert_eq!(effects(&authority), (476, 0, 476, 2, 1, 1));
}
