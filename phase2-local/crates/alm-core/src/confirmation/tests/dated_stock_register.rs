//! Legacy case L12-02.
use std::num::NonZeroI64;

use super::*;
use crate::DraftCustomerId;

mod purchase_receipts;
mod sale_packets;

const OWNER: Role = Role::Owner;
const PRODUCT: ProductId = ProductId::new(1);
const IDLE: ProductId = ProductId::new(2);
const SALE: DraftId = DraftId::new(8);
const ITEM: &str = "Synthetic Orange Soda";
const SELL: Money = Money::from_minor_units(500);
const BUY: Money = Money::from_minor_units(200);

fn date(day: u8) -> BillDate {
    BillDate::new(2025, 2, day).unwrap()
}

fn qty(packets: u32) -> NonZeroU32 {
    NonZeroU32::new(packets).unwrap()
}

fn purchase(fixture: &mut ConfirmationFixture) {
    fixture
        .register_purchase_product(PurchaseProduct {
            id: PRODUCT,
            item: ITEM.into(),
            buy_rate: BUY,
        })
        .unwrap();
    let id = fixture
        .save_purchase_order(
            OWNER,
            &PurchaseDraft {
                date: Some(date(8)),
                supplier: Some(PurchaseSupplier {
                    id: None,
                    name: "Synthetic Grove".into(),
                }),
                note: String::new(),
                items: vec![PurchaseLine {
                    item: ITEM.into(),
                    product_id: Some(PRODUCT),
                    ordered: qty(20),
                    rate: BUY,
                }],
            },
        )
        .unwrap();
    for (day, quantity) in [(11, 4), (10, 5)] {
        assert_eq!(
            fixture.receive_purchase_order(ReceiveRequest {
                role: OWNER,
                id,
                date: Some(date(day)),
                today: date(25),
                lines: &[ReceiveLine {
                    quantity,
                    rate: BUY,
                }],
            }),
            Ok(ReceiveResult::Received)
        );
    }
}

fn count(fixture: &mut ConfirmationFixture, product: ProductId, input: (u8, &str, Option<&str>)) {
    let (day, counted, fridge) = input;
    fixture
        .submit_physical_count(
            OWNER,
            PhysicalCountInput {
                product,
                counted,
                fridge,
                date: date(day),
                reason: None,
                note: None,
            },
        )
        .unwrap();
}

fn named_draft(id: DraftId, lines: Vec<DraftLine>, day: u8) -> ProvisionalDraft {
    ProvisionalDraft::new(id, lines)
        .with_bill_date(Some(date(day)))
        .with_customer(DraftCustomer::Selected {
            id: DraftCustomerId::new("l12-02-buyer".into()),
            name: "Synthetic Buyer".into(),
        })
}

fn fixture_with_sale() -> ConfirmationFixture {
    let mut fixture =
        ConfirmationFixture::with_ordered_lots(PRODUCT, &[(5, BUY), (11, BUY)], SELL).unwrap();
    let mut entry = fixture.open_product_entry(OWNER, PRODUCT).unwrap();
    entry.item = ITEM.into();
    fixture.save_product_entry(OWNER, &entry).unwrap();
    fixture.add_product(IDLE, 0, SELL).unwrap();
    purchase(&mut fixture);
    // Post adjustments before the sale to preserve the existing cancellation epoch.
    count(&mut fixture, PRODUCT, (10, "28", Some("13")));
    count(&mut fixture, PRODUCT, (10, "27", None));
    count(&mut fixture, PRODUCT, (11, "24", None));
    count(&mut fixture, IDLE, (10, "2", Some("6")));
    count(&mut fixture, IDLE, (10, "0", None));
    fixture
        .save_manual_godown(
            OWNER,
            ManualGodownInput {
                item: ITEM,
                cases: "31",
                note: "back rack",
            },
        )
        .unwrap();
    fixture
        .register_draft(named_draft(SALE, vec![DraftLine::new(PRODUCT, qty(7))], 11))
        .unwrap();
    let review = fixture.review_named(OWNER, SALE, SELL).unwrap();
    fixture.confirm(OWNER, RequestId::new(1), &review).unwrap();
    fixture
}

fn check_rows(fixture: &ConfirmationFixture, day: u8, expected: &[DailyMovementRow<'_>]) {
    let before = fixture.clone();
    let date = date(day);
    let rows = fixture.open_daily_movement_register(OWNER, date).unwrap();
    println!("L12-02 {date}: {rows:?}");
    assert_eq!(rows, expected);
    assert_eq!(*fixture, before);
}

#[test]
fn l12_02_projects_dated_rows_from_seed_not_current_stock_or_location_counts() {
    let fixture = fixture_with_sale();
    assert_eq!(fixture.product_stock(PRODUCT), Some(17));
    assert_eq!(
        fixture.open_physical_count(OWNER, PRODUCT).unwrap().fridge,
        13
    );
    assert_eq!(
        fixture.open_manual_godown(OWNER).unwrap()[0].cases.get(),
        31
    );

    for (
        day,
        opening,
        received,
        dispatched,
        adjustment,
        display_received,
        display_dispatched,
        closing,
    ) in [
        (8, 16, 0, 0, 0, 0, 0, 16),
        (10, 16, 5, 0, 2, 7, 0, 23),
        (11, 23, 4, 7, -3, 4, 10, 17),
        (12, 17, 0, 0, 0, 0, 0, 17),
        (25, 17, 0, 0, 0, 0, 0, 17),
    ] {
        check_rows(
            &fixture,
            day,
            &[DailyMovementRow {
                product: PRODUCT,
                item: Some(ITEM),
                opening,
                received,
                dispatched,
                adjustment,
                display_received: NonZeroI64::new(display_received),
                display_dispatched: NonZeroI64::new(display_dispatched),
                closing,
            }],
        );
    }
}

#[test]
fn l12_02_cancel_removes_original_dispatch_without_a_return_receipt() {
    let mut fixture = fixture_with_sale();
    fixture.cancel_sale(OWNER, SALE).unwrap();
    assert_eq!(fixture.product_stock(PRODUCT), Some(24));

    for (day, opening, received, adjustment, display_received, display_dispatched) in
        [(11, 23, 4, -3, 4, 3), (12, 24, 0, 0, 0, 0)]
    {
        check_rows(
            &fixture,
            day,
            &[DailyMovementRow {
                product: PRODUCT,
                item: Some(ITEM),
                opening,
                received,
                dispatched: 0,
                adjustment,
                display_received: NonZeroI64::new(display_received),
                display_dispatched: NonZeroI64::new(display_dispatched),
                closing: 24,
            }],
        );
    }
}

#[test]
fn l12_02_denies_manager_before_reading_sale_data() {
    let mut fixture = fixture_with_sale();
    // Fault injection: an Owner gets a distinct error if the sale packet is read.
    let (review, _, _) = fixture.current_sales.get_mut(&SALE).unwrap();
    review.draft = review.draft.clone().with_bill_date(None);
    let before = fixture.clone();

    assert_eq!(
        fixture.open_daily_movement_register(Role::Manager, date(11)),
        Err(DailyMovementError::AccessDenied)
    );
    assert_eq!(
        fixture.open_daily_movement_register(OWNER, date(11)),
        Err(DailyMovementError::MissingSaleDate)
    );
    assert_eq!(fixture, before);
}
