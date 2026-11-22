use super::*;
use crate::confirmation::{ProductPriceInput, RequestId};
use crate::{
    BillDate, DraftCustomer, DraftCustomerId, DraftId, DraftLine, ProductId, ProvisionalDraft,
};
use std::num::NonZeroU32;

mod identity;
mod pricing;

fn date(day: u8) -> BillDate {
    BillDate::new(2024, 10, day).unwrap()
}
fn money(value: i64) -> Money {
    Money::from_minor_units(value)
}
fn product(f: &mut ConfirmationFixture, id: u64, stock: u32, buy: i64) {
    let id = ProductId::new(id);
    f.add_product(id, stock, money(100)).unwrap();
    let mut form = f.open_product_entry(Role::Owner, id).unwrap();
    form.item = format!("Synthetic {id:?}");
    form.apply_price(ProductPriceInput::Buy(money(buy)))
        .unwrap();
    f.save_product_entry(Role::Owner, &form).unwrap();
}
fn post(f: &mut ConfirmationFixture, id: u64, product: u64, qty: u32, day: u8) {
    let draft = ProvisionalDraft::new(
        DraftId::new(id),
        vec![DraftLine::new(
            ProductId::new(product),
            NonZeroU32::new(qty).unwrap(),
        )],
    )
    .with_customer(DraftCustomer::Selected {
        id: DraftCustomerId::new("buyer".into()),
        name: "Synthetic buyer".into(),
    })
    .with_bill_date(Some(date(day)));
    f.register_draft(draft).unwrap();
    let review = f
        .review_named(Role::Owner, DraftId::new(id), money(1))
        .unwrap();
    f.confirm(Role::Owner, RequestId::new(id), &review).unwrap();
}

#[test]
fn owner_sees_never_sold_positive_stock_at_current_buy() {
    let mut fixture =
        ConfirmationFixture::new(ProductId::new(1), 5, Money::from_minor_units(100)).unwrap();
    let mut form = fixture
        .open_product_entry(Role::Owner, ProductId::new(1))
        .unwrap();
    form.item = "Synthetic p".into();
    form.apply_price(super::super::ProductPriceInput::Buy(
        Money::from_minor_units(400),
    ))
    .unwrap();
    fixture.save_product_entry(Role::Owner, &form).unwrap();
    let cutoff = BillDate::new(2024, 10, 4).unwrap();

    let rows = fixture.dead_stock_report(Role::Owner, cutoff).unwrap();

    assert_eq!(
        (
            rows[0].product,
            rows[0].on_hand,
            rows[0].locked_value.minor_units()
        ),
        (ProductId::new(1), 5, 2000)
    );
}

#[test]
fn l16_02_cutoff_uses_latest_bill_date_and_positive_current_stock() {
    let mut f = ConfirmationFixture::new(ProductId::new(99), 0, money(100)).unwrap();
    for id in 1..=9 {
        product(&mut f, id, if id == 6 { 0 } else { 4 }, 100);
    }
    post(&mut f, 7, 5, 1, 5);
    f.cancel_sale(Role::Owner, DraftId::new(7)).unwrap();
    for (bill, id, day) in [
        (1, 1, 3),
        (2, 2, 4),
        (3, 3, 5),
        (4, 9, 6),
        (5, 2, 1),
        (6, 7, 3),
    ] {
        post(&mut f, bill, id, if id == 7 { 4 } else { 1 }, day);
    }
    let before = f.clone();
    for (cutoff, expected) in [(date(4), vec![1, 4, 5, 8]), (date(5), vec![1, 2, 4, 5, 8])] {
        let rows = f.dead_stock_report(Role::Owner, cutoff).unwrap();
        let mut ids: Vec<_> = rows.iter().map(|row| row.product.value()).collect();
        ids.sort_unstable();
        println!(
            "L16-02 cutoff={cutoff} rows={rows:?}; zero-opening id=6 stock={:?}; exhausted id=7 stock={:?}",
            f.product_stock(ProductId::new(6)),
            f.product_stock(ProductId::new(7))
        );
        println!(
            "L16-02 stock control: id=1 old=2024-10-03 on_hand=3 raw_paise=300; id=7 same_date on_hand=0 raw_paise=0; id=5 recent sale cancelled; id=8 never sold (undated posting is not representable)"
        );
        assert_eq!(ids, expected);
    }
    assert_eq!(f, before);
}

#[test]
fn l16_03_cancellation_and_undo_use_actual_stock_movements() {
    let mut f = ConfirmationFixture::new(ProductId::new(99), 0, money(100)).unwrap();
    product(&mut f, 1, 5, 400);
    post(&mut f, 1, 1, 1, 3);
    post(&mut f, 2, 1, 2, 5);
    f.cancel_sale(Role::Owner, DraftId::new(2)).unwrap();
    let before = f.clone();
    let rows = f.dead_stock_report(Role::Owner, date(4)).unwrap();
    assert_eq!(f, before);
    println!("L16-03 old active recent cancelled rows={rows:?}");
    assert_eq!(
        (
            rows[0].on_hand,
            rows[0].last_sale,
            rows[0].locked_value.minor_units()
        ),
        (4, Some(date(3)), 1600)
    );
    f.undo_cancel_sale(Role::Owner, DraftId::new(2)).unwrap();
    let before = f.clone();
    let restored = f.dead_stock_report(Role::Owner, date(4)).unwrap();
    assert_eq!(f, before);
    println!(
        "L16-03 recent restored stock={:?} raw_paise=800 rows={restored:?}",
        f.product_stock(ProductId::new(1))
    );
    assert_eq!(f.product_stock(ProductId::new(1)), Some(2));
    assert!(restored.is_empty());
    let mut only = ConfirmationFixture::new(ProductId::new(99), 0, money(100)).unwrap();
    product(&mut only, 1, 5, 400);
    post(&mut only, 1, 1, 2, 5);
    only.cancel_sale(Role::Owner, DraftId::new(1)).unwrap();
    let before = only.clone();
    let rows = only.dead_stock_report(Role::Owner, date(4)).unwrap();
    assert_eq!(only, before);
    println!("L16-03 cancelled only rows={rows:?}");
    assert_eq!(
        (
            rows[0].on_hand,
            rows[0].last_sale,
            rows[0].locked_value.minor_units()
        ),
        (5, None, 2000)
    );
}

#[test]
fn l16_04_exact_values_and_stable_catalog_ties_at_cap() {
    for reverse in [false, true] {
        let mut f = ConfirmationFixture::new(ProductId::new(99), 0, money(100)).unwrap();
        let mut entries: Vec<_> = (1..=12).map(|id| (id, 1, 2000)).collect();
        entries.extend([(13, 2, 425), (14, 1, 1075), (15, 1, 0), (16, 1, 0)]);
        if reverse {
            entries.reverse();
        }
        for (id, stock, buy) in entries {
            product(&mut f, id, stock, buy);
        }
        let before = f.clone();
        let rows = f.dead_stock_report(Role::Owner, date(4)).unwrap();
        println!(
            "L16-04 reverse={reverse} exact rows={rows:?}; fractional packet/paise model cannot be represented by fixture"
        );
        let mut expected: Vec<_> = (1..=12).collect();
        if reverse {
            expected.reverse();
        }
        expected.extend([14, 13, if reverse { 16 } else { 15 }]);
        assert_eq!(
            rows.iter().map(|r| r.product.value()).collect::<Vec<_>>(),
            expected
        );
        assert_eq!(
            (
                rows[12].locked_value.minor_units(),
                rows[13].on_hand,
                rows[13].locked_value.minor_units()
            ),
            (1075, 2, 850)
        );
        assert_eq!(f, before);
    }
}

#[test]
fn empty_denial_and_checked_overflow_leave_fixture_unchanged() {
    let mut f = ConfirmationFixture::new(ProductId::new(99), 0, money(100)).unwrap();
    product(&mut f, 1, 0, 1000);
    product(&mut f, 2, 2, 1000);
    post(&mut f, 1, 2, 1, 4);
    let before = f.clone();
    let empty = f.dead_stock_report(Role::Owner, date(4)).unwrap();
    let denied = f.dead_stock_report(Role::Manager, date(4));
    println!(
        "L16-04 empty={empty:?} zero stock={:?} boundary stock={:?} manager={denied:?}",
        f.product_stock(ProductId::new(1)),
        f.product_stock(ProductId::new(2))
    );
    assert!(empty.is_empty());
    assert_eq!(denied, Err(ConfirmationError::AccessDenied));
    assert_eq!(f, before);
    let mut overflow = ConfirmationFixture::new(ProductId::new(1), 2, money(0)).unwrap();
    let mut form = overflow
        .open_product_entry(Role::Owner, ProductId::new(1))
        .unwrap();
    form.item = "Overflow".into();
    form.apply_price(ProductPriceInput::Buy(money(i64::MAX)))
        .unwrap();
    overflow.save_product_entry(Role::Owner, &form).unwrap();
    let before = overflow.clone();
    assert_eq!(
        overflow.dead_stock_report(Role::Owner, date(4)),
        Err(ConfirmationError::Overflow)
    );
    assert_eq!(overflow, before);
}
