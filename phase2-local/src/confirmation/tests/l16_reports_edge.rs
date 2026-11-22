use super::*;
use crate::confirmation::manual_cashbook::{CashBookAmounts, ManualCashBook, ManualCashBookDraft};
#[path = "l16_unknown_cost.rs"]
mod unknown_cost;

#[test]
fn l16_01_native_walk_in_and_checkout_loading_use_frozen_line_profit() {
    let product = ProductId::new(1);
    let mut fixture = ConfirmationFixture::with_ordered_lots(
        product,
        &[(3, Money::from_minor_units(300))],
        Money::from_minor_units(1000),
    )
    .unwrap();
    let guest = DraftId::new(1);
    fixture
        .register_draft(
            ProvisionalDraft::new(
                guest,
                vec![DraftLine::new(product, NonZeroU32::new(1).unwrap())],
            )
            .with_customer(DraftCustomer::Typed("Local - Neighbor".into()))
            .with_bill_date(Some(date("2025-04-30"))),
        )
        .unwrap();
    let review = fixture.review_walk_in_cash(Role::Owner, guest).unwrap();
    fixture
        .confirm(Role::Owner, RequestId::new(1), &review)
        .unwrap();
    fixture
        .set_checkout_product(
            product,
            ProductTerms {
                pack: NonZeroU32::new(1).unwrap(),
                piece_price: Money::from_minor_units(1000),
                case_price: Money::from_minor_units(1000),
            },
        )
        .unwrap();
    fixture
        .set_loading_rate(Money::from_minor_units(200))
        .unwrap();
    let named = DraftId::new(2);
    fixture
        .register_draft(
            ProvisionalDraft::new(
                named,
                vec![DraftLine::new(product, NonZeroU32::new(1).unwrap())],
            )
            .with_customer(DraftCustomer::Selected {
                id: DraftCustomerId::new("buyer".into()),
                name: "Buyer".into(),
            })
            .with_bill_date(Some(date("2025-04-20"))),
        )
        .unwrap();
    let review = fixture
        .review_checkout(
            Role::Owner,
            named,
            &[SaleUnit::Case],
            Money::from_minor_units(0),
        )
        .unwrap();
    fixture
        .confirm(Role::Owner, RequestId::new(2), &review)
        .unwrap();
    let book = ManualCashBook::default();
    let before = fixture.clone();
    let report = fixture
        .monthly_profit_report(Role::Owner, &book, date("2025-04-01"))
        .unwrap();
    let april = &report[5];
    println!(
        "native and checkout bills={:?} revenue={} cost={} gross={} net={}",
        april.bills,
        april.revenue.minor_units(),
        april.cost.minor_units(),
        april.gross.minor_units(),
        april.net.minor_units()
    );
    assert_eq!(
        (
            april.bills.as_slice(),
            april.revenue.minor_units(),
            april.cost.minor_units(),
            april.gross.minor_units(),
            april.net.minor_units()
        ),
        (&[guest, named][..], 2200, 600, 1400, 1400)
    );
    let ranks = fixture.customer_ranking(Role::Owner).unwrap();
    assert_eq!(
        ranks
            .iter()
            .map(|row| (row.name.as_str(), row.total.minor_units()))
            .collect::<Vec<_>>(),
        [("Buyer", 1200), ("Local - Neighbor", 1000)]
    );
    assert_eq!(fixture, before);
}

#[test]
fn l16_01_checked_cash_overflow_and_manager_denial() {
    let fixture =
        ConfirmationFixture::new(ProductId::new(1), 0, Money::from_minor_units(0)).unwrap();
    let mut book = ManualCashBook::default();
    let zero = Money::from_minor_units(0);
    let amounts = CashBookAmounts {
        cash: zero,
        paytm: zero,
        phonepe: zero,
        goods: Money::from_minor_units(i64::MAX),
        misc: zero,
        salary: zero,
        debt: zero,
    };
    book.save(
        Role::Owner,
        &ManualCashBookDraft::new(Some(date("2025-04-01")), amounts),
    )
    .unwrap();
    let mut next = amounts;
    next.goods = Money::from_minor_units(1);
    book.save(
        Role::Owner,
        &ManualCashBookDraft::new(Some(date("2025-04-02")), next),
    )
    .unwrap();
    assert_eq!(
        fixture.monthly_profit_report(Role::Owner, &book, date("2025-04-01")),
        Err(ReportsError::Confirmation(ConfirmationError::Overflow))
    );
    assert_eq!(
        fixture.monthly_profit_report(Role::Manager, &book, date("0001-01-01")),
        Err(ReportsError::Confirmation(ConfirmationError::AccessDenied))
    );
    assert_eq!(
        fixture.customer_ranking(Role::Manager),
        Err(ConfirmationError::AccessDenied)
    );
}

#[test]
fn l16_01_ranking_caps_ties_using_javascript_key_and_posting_order() {
    let product = ProductId::new(1);
    let mut fixture = ConfirmationFixture::with_ordered_lots(
        product,
        &[(12, Money::from_minor_units(1))],
        Money::from_minor_units(100),
    )
    .unwrap();
    for (id, name) in [
        (90, "old"),
        (7, "4294967295"),
        (88, "01"),
        (1, "10"),
        (44, "2"),
        (19, "4294967294"),
        (12, "0"),
        (5, "+1"),
        (23, "4294967296"),
        (2, "new"),
    ] {
        let draft = DraftId::new(id);
        fixture
            .register_draft(
                ProvisionalDraft::new(
                    draft,
                    vec![DraftLine::new(product, NonZeroU32::new(1).unwrap())],
                )
                .with_customer(DraftCustomer::Selected {
                    id: DraftCustomerId::new(name.into()),
                    name: name.into(),
                })
                .with_bill_date(Some(date("2020-01-01"))),
            )
            .unwrap();
        let review = fixture
            .review_named(Role::Owner, draft, Money::from_minor_units(0))
            .unwrap();
        fixture
            .confirm(Role::Owner, RequestId::new(id), &review)
            .unwrap();
    }
    let ranks = fixture.customer_ranking(Role::Owner).unwrap();
    assert_eq!(
        ranks
            .iter()
            .map(|row| row.name.as_str())
            .collect::<Vec<_>>(),
        [
            "0",
            "2",
            "10",
            "4294967294",
            "new",
            "4294967296",
            "+1",
            "01"
        ]
    );
}
