use std::num::NonZeroU32;

use super::*;
use alm_phase2_local::confirmation::{
    CheckoutAmounts, CrateTemplate, CrateTemplateError, ProductTerms,
};
use alm_phase2_local::{
    CrateComponent, DraftCustomer, DraftCustomerId, DraftLine, ProvisionalDraft, SaleUnit,
};

const OWNER: Role = Role::Owner;
const MANAGER: Role = Role::Manager;
const PIECE: ProductId = ProductId::new(2);
const CASE: ProductId = ProductId::new(1);
const SALE: DraftId = DraftId::new(43);
const REQUEST: RequestId = RequestId::new(987);
const NAME: &str = "Synthetic ₹ crate | owner's; mix,\n42";

const fn rupees(amount: i64) -> Money {
    Money::from_minor_units(100 * amount)
}

fn qty(amount: u32) -> NonZeroU32 {
    NonZeroU32::new(amount).unwrap()
}

fn template() -> CrateTemplate {
    CrateTemplate {
        name: NAME.into(),
        components: vec![
            CrateComponent::new(PIECE, qty(6), SaleUnit::Piece),
            CrateComponent::new(CASE, qty(3), SaleUnit::Case),
        ],
        price: rupees(250),
    }
}

fn authority() -> ConfirmationFixture {
    let mut authority = ConfirmationFixture::with_ordered_lots(
        PIECE,
        &[(1, rupees(14)), (5, rupees(20))],
        rupees(37),
    )
    .unwrap();
    authority.add_product(CASE, 0, rupees(73)).unwrap();
    authority
        .catalog
        .seed_lots(CASE, &[(4, rupees(23)), (5, rupees(29))])
        .unwrap();
    authority.set_crate_piece_mrp(PIECE, rupees(11)).unwrap();
    authority.set_crate_piece_mrp(CASE, rupees(59)).unwrap();
    for (product, (pack, piece, case)) in [(PIECE, (4, 19, 89)), (CASE, (8, 13, 97))] {
        authority
            .set_checkout_product(
                product,
                ProductTerms {
                    pack: qty(pack),
                    piece_price: rupees(piece),
                    case_price: rupees(case),
                },
            )
            .unwrap();
    }
    authority.set_crate_rate(rupees(321)).unwrap();
    authority
}

#[test]
fn l10_01_template_sale_survives_template_edit_delete_and_pending_reopen() {
    let scratch = Scratch::new();
    let database = scratch.0.join(DATABASE);
    let mut authority = authority();
    let original = template();
    let id = authority
        .save_crate_template(OWNER, original.clone())
        .unwrap();
    let saved = authority.crate_template(OWNER, id).unwrap();
    assert_eq!(saved, &original);
    assert_eq!(
        authority.suggest_crate_price(OWNER, &saved.components),
        Ok(rupees(285))
    );

    let mut draft = ProvisionalDraft::new(SALE, vec![])
        .with_customer(DraftCustomer::Selected {
            id: DraftCustomerId::new("crate-template-buyer".into()),
            name: "Synthetic".into(),
        })
        .with_bill_date(test_bill_date());
    authority
        .load_crate_template(OWNER, id, qty(2), &mut draft)
        .unwrap();
    assert_eq!(
        authority.load_crate_template(OWNER, id, qty(1), &mut draft),
        Err(CrateTemplateError::DraftHasRows)
    );
    assert!(draft.lines().is_empty());
    let row = draft.crate_row().unwrap();
    assert_eq!(row.quantity(), qty(2));
    assert_eq!(row.components(), original.components);
    assert_eq!(row.name(), Some(NAME));
    assert_eq!(row.rate(), Some(rupees(250)));

    open_sales(&database)
        .session(OWNER)
        .save_draft(draft.clone())
        .unwrap();
    let mut reopened = open_sales(&database);
    let persisted = reopened.session(OWNER).reopen_draft(SALE).unwrap().unwrap();
    assert_eq!(persisted, draft);
    authority.register_draft(persisted).unwrap();
    let review = authority
        .review_checkout(OWNER, SALE, &[], rupees(500))
        .unwrap();
    assert_eq!(review.unit_prices(), &[rupees(250)]);
    assert_eq!(
        review.crate_packet_demands().unwrap(),
        Some(vec![
            DraftLine::new(PIECE, qty(3)),
            DraftLine::new(CASE, qty(6))
        ])
    );
    assert_eq!(
        review.amounts().unwrap(),
        CheckoutAmounts {
            packets: 9,
            subtotal: rupees(500),
            loading: rupees(0),
            total: rupees(500)
        }
    );
    let receipt = reopened
        .session(OWNER)
        .confirm_pending(REQUEST, &review, |request, pending| {
            authority.confirm(OWNER, request, pending).unwrap()
        })
        .unwrap();
    drop(reopened);
    assert_eq!(receipt.total, rupees(500));
    assert_eq!(receipt.quantity, 9);

    let assert_original = |authority: &ConfirmationFixture| {
        assert_eq!(authority.current_sale(OWNER, SALE), Ok((&review, &receipt)));
        assert_eq!(authority.product_stock(PIECE), Some(3));
        assert_eq!(authority.product_stock(CASE), Some(3));
        assert_eq!(authority.catalog.lot_snapshot(PIECE), [0, 3]);
        assert_eq!(authority.catalog.lot_snapshot(CASE), [0, 3]);
        assert_eq!(authority.catalog.allocated_cost(SALE), Some(Some(20_400)));
        assert_eq!(
            authority.catalog.allocation_snapshot(),
            [
                (43, 0, 0, 1, Some(1_400)),
                (43, 0, 1, 2, Some(4_000)),
                (43, 1, 0, 4, Some(9_200)),
                (43, 1, 1, 2, Some(5_800)),
            ]
        );
        assert_eq!(authority.ledger(), rupees(500));
    };
    assert_original(&authority);

    let edited = CrateTemplate {
        name: "Edited crate".into(),
        components: vec![
            CrateComponent::new(CASE, qty(1), SaleUnit::Case),
            CrateComponent::new(PIECE, qty(4), SaleUnit::Piece),
        ],
        price: rupees(95),
    };
    authority
        .edit_crate_template(OWNER, id, edited.clone())
        .unwrap();
    assert_eq!(authority.crate_template(OWNER, id), Ok(&edited));
    assert_original(&authority);

    let mut fresh = ProvisionalDraft::new(DraftId::new(44), vec![]);
    authority
        .load_crate_template(OWNER, id, qty(1), &mut fresh)
        .unwrap();
    let new_row = fresh.crate_row().unwrap();
    assert_eq!(new_row.components(), edited.components);
    assert_eq!(new_row.quantity(), qty(1));
    assert_eq!(new_row.name(), Some("Edited crate"));
    assert_eq!(new_row.rate(), Some(rupees(95)));
    assert_original(&authority);

    authority.delete_crate_template(OWNER, id).unwrap();
    assert_eq!(
        authority.crate_template(OWNER, id),
        Err(CrateTemplateError::NotFound)
    );
    assert_original(&authority);
    let replay = open_sales(&database)
        .session(OWNER)
        .retry_pending(REQUEST, |request, pending| {
            assert_eq!(pending, &review);
            authority.confirm(OWNER, request, pending).unwrap()
        })
        .unwrap();
    assert_eq!(replay, receipt);
    assert_original(&authority);
}

#[test]
fn l10_01_manager_template_actions_leave_authority_and_draft_untouched() {
    let mut authority = authority();
    let template = template();
    let id = authority
        .save_crate_template(OWNER, template.clone())
        .unwrap();
    let before = authority.clone();
    let mut draft = ProvisionalDraft::new(SALE, vec![]);
    let untouched = draft.clone();

    assert_eq!(
        authority.save_crate_template(MANAGER, template.clone()),
        Err(CrateTemplateError::AccessDenied)
    );
    assert_eq!(
        authority.crate_template(MANAGER, id),
        Err(CrateTemplateError::AccessDenied)
    );
    assert_eq!(
        authority.suggest_crate_price(MANAGER, &template.components),
        Err(CrateTemplateError::AccessDenied)
    );
    assert_eq!(
        authority.edit_crate_template(MANAGER, id, template),
        Err(CrateTemplateError::AccessDenied)
    );
    assert_eq!(
        authority.delete_crate_template(MANAGER, id),
        Err(CrateTemplateError::AccessDenied)
    );
    assert_eq!(
        authority.load_crate_template(MANAGER, id, qty(1), &mut draft),
        Err(CrateTemplateError::AccessDenied)
    );
    assert_eq!(authority, before);
    assert_eq!(draft, untouched);
}
