use std::num::NonZeroU32;

use alm_phase2_local::confirmation::{
    ConfirmationError, ConfirmationFixture, PhysicalCount, PhysicalCountError, PhysicalCountInput,
    RequestId, Review, StockCountMovement, StockCountReason, StockLots,
};
use alm_phase2_local::{
    BillDate, DraftCustomer, DraftCustomerId, DraftId, DraftLine, Money, ProductId,
    ProvisionalDraft, Role,
};

const OWNER: Role = Role::Owner;
const PRODUCT: ProductId = ProductId::new(1);
const MISSING: ProductId = ProductId::new(2);
const DRAFT: DraftId = DraftId::new(1);
const REQUEST: RequestId = RequestId::new(1);
const SELL: Money = Money::from_minor_units(500);
const OLDER: Money = Money::from_minor_units(80);
const NEWER: Money = Money::from_minor_units(120);

fn date(day: u8) -> BillDate {
    BillDate::new(2025, 2, day).expect("synthetic date")
}

fn input<'a>(counted: &'a str, fridge: Option<&'a str>) -> PhysicalCountInput<'a> {
    PhysicalCountInput {
        product: PRODUCT,
        counted,
        fridge,
        date: date(12),
        reason: None,
        note: None,
    }
}

fn fixture() -> ConfirmationFixture {
    let mut fixture =
        ConfirmationFixture::with_ordered_lots(PRODUCT, &[(3, OLDER), (4, NEWER)], SELL)
            .expect("synthetic catalog");
    fixture
        .submit_physical_count(OWNER, input("7", Some("11")))
        .expect("seed display-only fridge count");
    assert_eq!(counts(&fixture), (7, 11, 0));
    fixture
}

fn count(fixture: &ConfirmationFixture) -> PhysicalCount<'_> {
    fixture
        .open_physical_count(OWNER, PRODUCT)
        .expect("Owner count view")
}

fn counts(fixture: &ConfirmationFixture) -> (u32, u32, usize) {
    let actual = count(fixture);
    (actual.on_hand, actual.fridge, actual.movements.len())
}

fn lots(fixture: &ConfirmationFixture) -> StockLots {
    fixture.open_stock_lots(OWNER, PRODUCT).unwrap()
}

fn review(fixture: &mut ConfirmationFixture, packets: u32) -> Review {
    let line = DraftLine::new(PRODUCT, NonZeroU32::new(packets).unwrap());
    let draft = ProvisionalDraft::new(DRAFT, vec![line])
        .with_customer(DraftCustomer::Selected {
            id: DraftCustomerId::new("l11-02-buyer".into()),
            name: "Synthetic Buyer".into(),
        })
        .with_bill_date(Some(date(11)));
    fixture.register_draft(draft).expect("register sale");
    fixture.review_named(OWNER, DRAFT, SELL).unwrap()
}

#[test]
fn l11_02_above_count_appends_an_unknown_cost_layer() {
    // Given computed stock 7 and display fridge 11.
    let mut fixture = fixture();
    let original = lots(&fixture);
    let request = PhysicalCountInput {
        reason: Some(StockCountReason::CountCorrection),
        note: Some("  recount at close \t"),
        ..input(" 10 ", None)
    };

    // When the Owner counts 10.
    fixture.submit_physical_count(OWNER, request).unwrap();

    // Then the movement is positive, the note is trimmed, and no cost is invented.
    let actual = count(&fixture);
    assert_eq!((actual.on_hand, actual.fridge), (10, 11));
    let movement = StockCountMovement {
        date: request.date,
        from: 7,
        to: 10,
        delta: 3,
        reason: request.reason,
        note: Some("recount at close".into()),
    };
    assert_eq!(actual.movements, std::slice::from_ref(&movement));
    let lots = lots(&fixture);
    assert_eq!(lots.on_hand, 10);
    let (adjustment, known) = lots.rows.split_last().unwrap();
    assert_eq!(known, original.rows);
    assert_eq!(adjustment.date, Some(request.date));
    assert_eq!(adjustment.remaining.get(), 3);
    assert_eq!((adjustment.unit_cost, adjustment.value), (None, None));
    assert_eq!(lots.total_value, None);
    println!("l11_02 above: {movement:?}; fridge=11 cost unknown");
}

#[test]
fn l11_02_below_count_consumes_oldest_remaining_packets() {
    // Given computed stock 7 and display fridge 11.
    let mut fixture = fixture();
    let request = PhysicalCountInput {
        note: Some(" \t "),
        ..input("3", None)
    };

    // When the Owner counts 3.
    fixture.submit_physical_count(OWNER, request).unwrap();

    // Then the correction exhausts the first lot and consumes one from the second.
    let actual = count(&fixture);
    assert_eq!((actual.on_hand, actual.fridge), (3, 11));
    let movement = StockCountMovement {
        date: request.date,
        from: 7,
        to: 3,
        delta: -4,
        reason: None,
        note: None,
    };
    assert_eq!(actual.movements, std::slice::from_ref(&movement));
    let lots = lots(&fixture);
    assert_eq!((lots.on_hand, lots.rows.len()), (3, 1));
    let remaining = &lots.rows[0];
    assert_eq!(remaining.remaining.get(), 3);
    assert_eq!(remaining.unit_cost, Some(NEWER));
    assert_eq!(lots.total_value, Some(Money::from_minor_units(360)));
    println!("l11_02 below: {movement:?}; fridge=11 oldest lot exhausted");
}

#[test]
fn l11_02_equal_count_can_leave_everything_unchanged_or_update_only_fridge() {
    for (fridge, expected) in [(None, 11), (Some(" 13 "), 13)] {
        // Given an independent copy of the same starting stock and fridge.
        let mut fixture = fixture();
        let before = lots(&fixture);

        // When a valid equal count is submitted, with or without a fridge change.
        let request = input("7", fridge);
        fixture.submit_physical_count(OWNER, request).unwrap();

        // Then there is no stock movement or lot change.
        assert_eq!(counts(&fixture), (7, expected, 0));
        assert_eq!(lots(&fixture), before);
        println!("l11_02 equal: on_hand=7 fridge=11->{expected} movements=0");
    }
}

#[test]
fn l11_02_rejects_invalid_quantity_without_partial_save() {
    let invalid = [" \t ", "NaN", "Infinity", "-1", "1.5", "4294967296", "nope"];
    for value in invalid {
        // Given the same stock and a proposed change to both fields.
        let mut fixture = fixture();
        let before = lots(&fixture);

        // When either field receives invalid text.
        let actual = fixture.submit_physical_count(OWNER, input(value, Some("12")));
        let fridge = fixture.submit_physical_count(OWNER, input("8", Some(value)));

        // Then neither field, the movement log, nor the lots changes.
        assert_eq!(actual, Err(PhysicalCountError::InvalidCount));
        assert_eq!(fridge, Err(PhysicalCountError::InvalidFridgeCount));
        assert_eq!(counts(&fixture), (7, 11, 0));
        assert_eq!(lots(&fixture), before);
    }
    println!("l11_02 invalid: {invalid:?} rejected in either field without changes");
}

#[test]
fn l11_02_accepts_both_packet_range_endpoints() {
    // Given the same starting stock for each endpoint, when an Owner submits it.
    for (text, expected) in [("0", 0), ("4294967295", u32::MAX)] {
        let mut fixture = fixture();
        let request = input(text, None);
        fixture.submit_physical_count(OWNER, request).unwrap();

        // Then the full unsigned stock range and its signed delta are represented exactly.
        let actual = count(&fixture);
        assert_eq!(actual.on_hand, expected);
        let delta = actual.movements[0].delta;
        assert_eq!(delta, i64::from(expected) - 7);
        let lots = lots(&fixture);
        let remaining = lots.rows.iter().map(|row| row.remaining.get()).sum::<u32>();
        assert_eq!(remaining, expected);
        println!("l11_02 range: target={expected} delta={delta}");
    }
}

#[test]
fn l11_02_denies_manager_before_validation_or_lookup() {
    // Given an existing product and an unknown ID.
    let mut fixture = fixture();
    let before = lots(&fixture);
    let missing = PhysicalCountInput {
        product: MISSING,
        ..input("8", None)
    };

    // When a Manager opens either product or submits valid, invalid, or unknown inputs.
    for product in [PRODUCT, MISSING] {
        let result = fixture.open_physical_count(Role::Manager, product);
        assert_eq!(result, Err(PhysicalCountError::AccessDenied));
    }
    let requests = [
        input("8", None),
        input("", None),
        missing,
        input("7", Some("12")),
    ];
    for request in requests {
        let result = fixture.submit_physical_count(Role::Manager, request);
        assert_eq!(result, Err(PhysicalCountError::AccessDenied));
    }

    // Then owner-only state stays unchanged, and the Owner can distinguish a missing product.
    let result = fixture.submit_physical_count(OWNER, missing);
    assert_eq!(result, Err(PhysicalCountError::ProductNotFound));
    assert_eq!(counts(&fixture), (7, 11, 0));
    assert_eq!(lots(&fixture), before);
    println!("l11_02 role: Manager denied before input validation and product lookup; no changes");
}

#[test]
fn l11_02_fridge_does_not_cover_posting_shortage() {
    // Given 7 on hand, 11 in fridge, and a sale requiring 8 packets.
    let mut fixture = fixture();
    let sale = review(&mut fixture, 8);
    let before = lots(&fixture);

    // When the Owner tries to post the sale.
    let result = fixture.confirm(OWNER, REQUEST, &sale);

    // Then posting rejects atomically because only on-hand stock is available.
    assert_eq!(result, Err(ConfirmationError::InsufficientStock));
    assert_eq!(fixture.posted_count(), 0);
    assert_eq!(fixture.ledger().minor_units(), 0);
    assert_eq!(counts(&fixture), (7, 11, 0));
    assert_eq!(lots(&fixture), before);
    println!("l11_02 posting: demand=8 on_hand=7 fridge=11 result={result:?}");
}

#[test]
fn l11_02_only_real_correction_invalidates_cancel_undo() {
    let stale = Err(ConfirmationError::StaleReview);
    for (request, expected, state) in [
        (input("7", Some("12")), Ok(()), (6, 12, 0)),
        (input("3", None), stale, (3, 11, 1)),
    ] {
        // Given a canceled sale, plus rejected and no-op submissions before undo.
        let mut fixture = fixture();
        let sale = review(&mut fixture, 1);
        fixture.confirm(OWNER, REQUEST, &sale).unwrap();
        fixture.cancel_sale(OWNER, DRAFT).unwrap();
        let rejected = fixture.submit_physical_count(OWNER, input("", None));
        assert_eq!(rejected, Err(PhysicalCountError::InvalidCount));
        let unchanged = input("7", None);
        fixture.submit_physical_count(OWNER, unchanged).unwrap();

        // When the Owner saves the count and tries to undo cancellation.
        fixture.submit_physical_count(OWNER, request).unwrap();
        let result = fixture.undo_cancel_sale(OWNER, DRAFT);

        // Then only a real correction makes undo stale.
        assert_eq!(result, expected);
        assert_eq!(counts(&fixture), state);
        println!("l11_02 epoch: target={} undo={result:?}", request.counted);
    }
}

#[test]
fn l11_02_deltas_use_submission_time_on_hand() {
    // Given a view at seven, followed by a sale reducing stock to six.
    let mut fixture = fixture();
    let shown = count(&fixture).on_hand.to_string();
    let sale = review(&mut fixture, 1);
    fixture.confirm(OWNER, REQUEST, &sale).unwrap();

    // When the Owner submits the originally displayed count.
    let request = input(&shown, None);
    fixture.submit_physical_count(OWNER, request).unwrap();

    // Then it is a correction from six, not a no-op from the earlier value of seven.
    let actual = count(&fixture);
    assert_eq!((actual.on_hand, actual.movements.len()), (7, 1));
    let movement = &actual.movements[0];
    assert_eq!((movement.from, movement.to, movement.delta), (6, 7, 1));
    println!("l11_02 submission stock: displayed={shown} movement=6->7 delta=+1");
}
