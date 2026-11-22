use super::*;
use alm_phase2_local::confirmation::{SalesPeriod, SalesReceiptId, SalesSource};
use std::cell::Cell;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn l14_03_default_save_uses_unix_milliseconds_and_clock_errors_are_atomic() {
    let customers = [Customer {
        id: DraftCustomerId::new("clock-customer".into()),
        name: "Clock Customer".into(),
    }];
    let mut fixture = ConfirmationFixture::new(PRODUCT, 1, money(2000)).unwrap();
    fixture.register_customer(OWNER, &customers[0].id).unwrap();
    let mut form = ReceivePayment::default();
    form.input("Clock Customer", &customers);
    let before = u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap();
    form.save(
        OWNER,
        &mut fixture,
        receipt_date(),
        money(250),
        ReceiptMode::Cash,
    )
    .unwrap();
    let after = u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap();
    assert!(before > 0 && before <= after && after < u64::MAX);
    let low = before - 1;
    let high = after + 1;
    let key = Cell::new(Ok(CreationKey::new(low)));
    let mut controlled = ReceivePayment::with_clock(|| key.get());
    controlled.input("Clock Customer", &customers);
    for (value, amount) in [(low, 100), (high, 200)] {
        key.set(Ok(CreationKey::new(value)));
        controlled
            .save(
                OWNER,
                &mut fixture,
                receipt_date(),
                money(amount),
                ReceiptMode::Cash,
            )
            .unwrap();
    }
    let history = fixture
        .sales_history(OWNER, receipt_date(), SalesPeriod::Today, "clock customer")
        .unwrap();
    assert_eq!(
        history
            .collected
            .details
            .iter()
            .map(|row| row.source)
            .collect::<Vec<_>>(),
        [
            SalesSource::Receipt(SalesReceiptId(3)),
            SalesSource::Receipt(SalesReceiptId(1)),
            SalesSource::Receipt(SalesReceiptId(2))
        ]
    );
    let ledger_before = fixture.customer_ledger(OWNER, &customers[0].id).unwrap();
    key.set(Err(ConfirmationError::Overflow));
    assert_eq!(
        controlled.save(
            OWNER,
            &mut fixture,
            receipt_date(),
            money(50),
            ReceiptMode::Cash
        ),
        Err(SaveError::Confirmation(ConfirmationError::Overflow))
    );
    assert_eq!(
        fixture.customer_ledger(OWNER, &customers[0].id).unwrap(),
        ledger_before
    );
    assert_eq!(
        fixture
            .sales_history(OWNER, receipt_date(), SalesPeriod::Today, "clock customer")
            .unwrap(),
        history
    );
    println!(
        "l14_03_default_save_uses_unix_milliseconds_and_clock_errors_are_atomic id=clock-customer default_key_range={before}..={after} paise=250; bounds={low}:100,{high}:200; clock_error=Overflow unchanged"
    );
}
