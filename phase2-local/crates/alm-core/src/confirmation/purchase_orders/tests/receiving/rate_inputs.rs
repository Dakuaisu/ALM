use std::num::{IntErrorKind, NonZeroU32, ParseIntError};

use super::super::super::PurchaseOrderStatus;
use super::{
    ConfirmationFixture, Money, OWNER, PRODUCT, PurchaseError, PurchaseReceipt, ReceiveLine,
    ReceiveRequest, ReceiveResult, Role, date, fixture, money, receive,
};

// TEST-ONLY: these raw fixture rates are whole rupees; Money holds minor units.
fn decode_fixture_rate(raw: &str) -> Result<Money, ParseIntError> {
    raw.parse::<i32>()
        .map(|rupees| money(i64::from(rupees) * 100))
}

#[test]
fn l08_06_raw_receipt_rates_on_independent_partial_po_clones() {
    let (mut base, id) = fixture();
    assert_eq!(
        receive(&mut base, id, Some(20), 2, 1300),
        Ok(ReceiveResult::Received)
    );
    let receive_one = |f: &mut ConfirmationFixture, role, rate| {
        f.receive_purchase_order(ReceiveRequest {
            role,
            id,
            date: Some(date(10)),
            today: date(25),
            lines: &[ReceiveLine { quantity: 1, rate }],
        })
    };

    // Static legacy trace: 6970-7049 logs +value || 0 as the actual rate.
    // For "0", 999-1031 keeps average/buy/quote; 1198-1222 uses the PO line rate for FIFO.
    let mut zero = base.clone();
    let zero_rate = decode_fixture_rate("0").unwrap();
    assert_eq!(zero_rate, money(0));
    assert_eq!(
        receive_one(&mut zero, OWNER, zero_rate),
        Ok(ReceiveResult::Received)
    );

    // "-3" is truthy in the legacy source: it would log -300 minor units and
    // blend (12 * 1050 - 300) / 13 to 946, keeping line/buy/quote/FIFO rate 1300.
    let mut negative = base.clone();
    let negative_rate = decode_fixture_rate("-3").unwrap();
    assert_eq!(negative_rate, money(-300));
    assert_eq!(
        receive_one(&mut negative, OWNER, negative_rate),
        Err(PurchaseError::InvalidReceipt)
    );
    assert_eq!(negative, base);

    // "not a rate" would log 0 in the legacy source. Here no Money is produced,
    // so the typed call in map is skipped, even with the same positive quantity.
    let mut nonnumeric = base.clone();
    let result =
        decode_fixture_rate("not a rate").map(|rate| receive_one(&mut nonnumeric, OWNER, rate));
    assert_eq!(result.unwrap_err().kind(), &IntErrorKind::InvalidDigit);
    assert_eq!(nonnumeric, base);

    // DEC-034: a Manager receives quantities only (rate 0: the order's rate); any rate it
    // sends is refused.
    let mut manager = base.clone();
    let mut owner = base.clone();
    assert_eq!(
        receive_one(&mut manager, Role::Manager, zero_rate),
        receive_one(&mut owner, OWNER, zero_rate)
    );
    assert_eq!(manager, owner);
    let mut denied = base.clone();
    assert_eq!(
        receive_one(&mut denied, Role::Manager, Money::from_minor_units(1)),
        Err(PurchaseError::AccessDenied)
    );
    assert_eq!(denied, base);

    for (f, stock, received, remaining, epoch) in [(&base, 12, 2, 3, 1), (&zero, 13, 3, 2, 2)] {
        let order = &f.purchase_orders(OWNER).unwrap()[0];
        let line = &order.items[0];
        assert_eq!(order.status(), PurchaseOrderStatus::PartiallyReceived);
        assert_eq!(line.line.ordered.get(), 5);
        assert_eq!(line.cancelled, 0);
        assert_eq!(line.received, received);
        assert_eq!(line.remaining(), remaining);
        assert_eq!(line.line.rate, money(1300));
        assert_eq!(f.product_stock(PRODUCT), Some(stock));
        assert_eq!(
            f.purchase_cost(OWNER, PRODUCT, "Supplier"),
            Ok(Some((money(1050), money(1300), Some(money(1300)))))
        );
        assert_eq!(f.movement_epoch, epoch);
    }

    let first = PurchaseReceipt {
        date: date(20),
        quantity: NonZeroU32::new(2).unwrap(),
        rate: money(1300),
    };
    let second = PurchaseReceipt {
        date: date(10),
        quantity: NonZeroU32::MIN,
        rate: money(0),
    };
    assert_eq!(
        base.purchase_orders(OWNER).unwrap()[0].items[0].receipts(),
        &[first]
    );
    assert_eq!(
        zero.purchase_orders(OWNER).unwrap()[0].items[0].receipts(),
        &[first, second]
    );
    assert_eq!(
        base.catalog.receipt_lots(PRODUCT),
        vec![(date(20), 2, 1300)]
    );
    assert_eq!(
        zero.catalog.receipt_lots(PRODUCT),
        vec![(date(10), 1, 1300), (date(20), 2, 1300)]
    );
    assert_eq!(base.purchase_movements(OWNER, id), Ok(vec![(date(20), 2)]));
    assert_eq!(
        zero.purchase_movements(OWNER, id),
        Ok(vec![(date(20), 2), (date(10), 1)])
    );
    assert_eq!(base.purchase_received_on(OWNER, id, date(10)), Ok(0));
    assert_eq!(zero.purchase_received_on(OWNER, id, date(10)), Ok(1));
    assert_eq!(zero.purchase_received_on(OWNER, id, date(20)), Ok(2));
}
