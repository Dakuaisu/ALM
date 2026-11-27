use std::num::NonZeroU32;

use super::LineEdit;
use crate::confirmation::demo_fixture::{customer_id, directory, fixture};
use crate::confirmation::owner_billing::{OwnerPosting, SaleTerms};
use crate::confirmation::{ConfirmationError, ConfirmationFixture, StockLotsError};
use crate::{DraftCustomer, DraftId, DraftLine, Money, ProductId, ProvisionalDraft, Role};

const LIME: ProductId = ProductId::new(1);

fn qty(n: u32) -> NonZeroU32 {
    NonZeroU32::new(n).unwrap()
}

fn posted(packets: u32) -> ConfirmationFixture {
    let mut f = fixture().unwrap();
    f.catalog
        .seed_lots(
            LIME,
            &[
                (50, Money::from_minor_units(7_000)),
                (5, Money::from_minor_units(7_500)),
            ],
        )
        .unwrap();
    let draft = ProvisionalDraft::new(DraftId::new(1), vec![DraftLine::new(LIME, qty(packets))])
        .with_customer(DraftCustomer::Selected {
            id: customer_id(),
            name: "Synthetic Customer".into(),
        });
    let mut posting = OwnerPosting::default();
    posting
        .prepare(
            &mut f,
            &directory(),
            Role::Owner,
            Some(&draft),
            SaleTerms {
                date: "2025-04-30",
                paid: "50",
            },
        )
        .unwrap();
    posting.confirm(&mut f, Role::Owner, 1).unwrap();
    f
}

fn lots(f: &ConfirmationFixture) -> Result<Vec<(u32, i64)>, StockLotsError> {
    Ok(f.open_stock_lots(Role::Owner, LIME)?
        .rows
        .iter()
        .map(|lot| (lot.remaining.get(), lot.unit_cost.unwrap().minor_units()))
        .collect())
}

#[test]
fn in_place_edit_moves_stock_lots_balance_and_keeps_the_frozen_unit_cost() {
    let mut f = posted(2);
    assert_eq!(lots(&f), Ok(vec![(48, 7_000), (5, 7_500)]));
    let edit = |n, paise| LineEdit {
        quantity: qty(n),
        price: Money::from_minor_units(paise),
    };
    let before = f.clone();
    // DEC-034: a Manager edits a posted bill as the Owner does.
    let (mut manager, mut owner) = (f.clone(), f.clone());
    assert_eq!(
        manager.edit_posted_lines(Role::Manager, DraftId::new(1), &[edit(1, 12_050)]),
        owner.edit_posted_lines(Role::Owner, DraftId::new(1), &[edit(1, 12_050)])
    );
    assert_eq!(manager, owner);
    assert_eq!(
        f.edit_posted_lines(Role::Owner, DraftId::new(1), &[edit(1, 12_050), edit(1, 1)]),
        Err(ConfirmationError::InvalidDraft)
    );
    assert_eq!(
        f.edit_posted_lines(Role::Owner, DraftId::new(1), &[edit(1, 4_000)]),
        Err(ConfirmationError::InvalidAmount)
    );
    assert_eq!(f, before);

    // Down to 1 at a new rate: one packet returns to its lot; due and balance follow.
    let receipt = f
        .edit_posted_lines(Role::Owner, DraftId::new(1), &[edit(1, 11_000)])
        .unwrap();
    assert_eq!(
        (
            receipt.invoice,
            receipt.quantity,
            receipt.total.minor_units()
        ),
        (1, 1, 11_000)
    );
    assert_eq!(receipt.due, Some(Money::from_minor_units(6_000)));
    assert_eq!(f.customer_due(customer_id().value()).minor_units(), 6_000);
    assert_eq!(f.ledger().minor_units(), 11_000);
    assert_eq!(f.product_stock(LIME), Some(54));
    assert_eq!(lots(&f), Ok(vec![(49, 7_000), (5, 7_500)]));
    assert_eq!(f.catalog.frozen_cost(DraftId::new(1)), Ok(Some(7_000)));

    // Up to 51: takes the rest of the first lot and one of the second; cost stays frozen.
    f.edit_posted_lines(Role::Owner, DraftId::new(1), &[edit(51, 12_050)])
        .unwrap();
    assert_eq!(f.product_stock(LIME), Some(4));
    assert_eq!(lots(&f), Ok(vec![(4, 7_500)]));
    assert_eq!(f.catalog.frozen_cost(DraftId::new(1)), Ok(Some(51 * 7_000)));
    let close = f
        .cashbook_day_close(Role::Owner, crate::BillDate::new(2025, 4, 30).unwrap())
        .unwrap();
    assert_eq!(close.cash.minor_units(), 5_000);

    // Cancelling the edited bill returns exactly the packets it now holds.
    f.cancel_sale(Role::Owner, DraftId::new(1)).unwrap();
    assert_eq!(f.product_stock(LIME), Some(55));
    assert_eq!(lots(&f), Ok(vec![(50, 7_000), (5, 7_500)]));
    assert_eq!(f.customer_due(customer_id().value()).minor_units(), 0);
    assert_eq!(
        f.edit_posted_lines(Role::Owner, DraftId::new(1), &[edit(1, 12_050)]),
        Err(ConfirmationError::InvalidDraft)
    );
}

#[test]
fn typed_line_edits_parse_in_core() {
    assert_eq!(
        LineEdit::from_text(" 3 ", "120.5"),
        Ok(LineEdit {
            quantity: qty(3),
            price: Money::from_minor_units(12_050)
        })
    );
    for (q, r) in [("0", "1"), ("x", "1"), ("1", "-1"), ("1", "1.001")] {
        assert_eq!(
            LineEdit::from_text(q, r),
            Err(ConfirmationError::InvalidAmount)
        );
    }
}
