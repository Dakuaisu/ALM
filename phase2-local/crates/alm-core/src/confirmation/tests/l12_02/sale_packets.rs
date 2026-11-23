use super::*;
use crate::{CrateComponent, CrateRow};

const OTHER: ProductId = ProductId::new(3);

fn dispatch_row(
    product: ProductId,
    (opening, dispatched, closing): (i64, i64, i64),
) -> DailyMovementRow<'static> {
    DailyMovementRow {
        product,
        item: None,
        opening,
        received: 0,
        dispatched,
        adjustment: 0,
        display_received: None,
        display_dispatched: NonZeroI64::new(dispatched),
        closing,
    }
}

#[test]
fn l12_02_current_edit_counts_the_appended_allocation_on_the_original_date() {
    let mut fixture = ConfirmationFixture::new(PRODUCT, 10, SELL).unwrap();
    fixture.add_product(OTHER, 8, SELL).unwrap();
    fixture
        .register_draft(named_draft(SALE, vec![DraftLine::new(PRODUCT, qty(3))], 10))
        .unwrap();
    let original = fixture.review_named(OWNER, SALE, SELL).unwrap();
    fixture
        .confirm(OWNER, RequestId::new(1), &original)
        .unwrap();
    let edit = fixture
        .begin_edit(
            OWNER,
            SALE,
            EditChanges {
                retained_price: SELL,
                appended_line: DraftLine::new(OTHER, qty(2)),
                appended_price: SELL,
            },
        )
        .unwrap();
    fixture.save_edit(OWNER, &edit).unwrap();

    check_rows(
        &fixture,
        10,
        &[
            dispatch_row(PRODUCT, (10, 3, 7)),
            dispatch_row(OTHER, (8, 2, 6)),
        ],
    );
}

#[test]
fn l12_02_checkout_and_crates_use_posted_packets_after_pack_terms_change() {
    let mut fixture = ConfirmationFixture::new(PRODUCT, 15, SELL).unwrap();
    fixture.add_product(OTHER, 12, SELL).unwrap();
    let terms = ProductTerms {
        pack: qty(6),
        piece_price: BUY,
        case_price: SELL,
    };
    fixture.set_checkout_product(PRODUCT, terms).unwrap();
    fixture.set_checkout_product(OTHER, terms).unwrap();
    fixture
        .register_draft(named_draft(
            SALE,
            vec![
                DraftLine::new(PRODUCT, qty(12)),
                DraftLine::new(OTHER, qty(3)),
            ],
            10,
        ))
        .unwrap();
    let review = fixture
        .review_checkout(OWNER, SALE, &[SaleUnit::Piece, SaleUnit::Case], SELL)
        .unwrap();
    fixture.confirm(OWNER, RequestId::new(1), &review).unwrap();

    let crate_id = DraftId::new(9);
    let mut draft = named_draft(crate_id, Vec::new(), 11);
    draft
        .add_crate_row(CrateRow::new(
            qty(2),
            vec![
                CrateComponent::new(PRODUCT, qty(12), SaleUnit::Piece),
                CrateComponent::new(OTHER, qty(1), SaleUnit::Case),
            ],
        ))
        .unwrap();
    fixture.set_crate_rate(SELL).unwrap();
    fixture.register_draft(draft).unwrap();
    let review = fixture.review_checkout(OWNER, crate_id, &[], SELL).unwrap();
    fixture.confirm(OWNER, RequestId::new(2), &review).unwrap();
    fixture
        .set_checkout_product(
            PRODUCT,
            ProductTerms {
                pack: qty(3),
                ..terms
            },
        )
        .unwrap();

    check_rows(
        &fixture,
        10,
        &[
            dispatch_row(PRODUCT, (15, 2, 13)),
            dispatch_row(OTHER, (12, 3, 9)),
        ],
    );
    check_rows(
        &fixture,
        11,
        &[
            dispatch_row(PRODUCT, (13, 4, 9)),
            dispatch_row(OTHER, (9, 2, 7)),
        ],
    );
}
