use super::*;

#[test]
fn l14_03_zero_amount_is_rejected_before_clock_without_receipt() {
    let customers = [Customer {
        id: DraftCustomerId::new("zero-customer".into()),
        name: "Zero Customer".into(),
    }];
    let mut fixture = ConfirmationFixture::new(PRODUCT, 1, money(2_000)).unwrap();
    fixture.register_customer(OWNER, &customers[0].id).unwrap();
    let mut form = ReceivePayment::with_clock(|| panic!("zero amount sampled clock"));
    form.input("Zero Customer", &customers);
    let before = fixture.customer_ledger(OWNER, &customers[0].id).unwrap();
    let result = form.save(
        OWNER,
        &mut fixture,
        receipt_date(),
        money(0),
        ReceiptMode::Cash,
    );
    assert_eq!(
        result,
        Err(SaveError::Confirmation(ConfirmationError::InvalidAmount))
    );
    assert_eq!(
        fixture.customer_ledger(OWNER, &customers[0].id).unwrap(),
        before
    );
    assert!(
        fixture
            .cashbook_day_close(OWNER, receipt_date())
            .unwrap()
            .entries
            .is_empty()
    );
    println!(
        "l14_03_zero_amount_is_rejected_before_clock_without_receipt id=zero-customer paise=0 result={result:?}; clock not sampled; Cash Book empty"
    );
}
