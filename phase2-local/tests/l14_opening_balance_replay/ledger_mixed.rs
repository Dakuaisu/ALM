use super::*;

#[test]
fn keyed_target_uses_zero_for_missing_peers_and_unkeyed_target_uses_anchor() {
    let money = Money::from_minor_units;
    let customer = DraftCustomerId::new("fallback".into());
    let product = ProductId::new(1);
    let date: BillDate = FEB10.parse().unwrap();
    let mut fixture = ConfirmationFixture::new(product, 4, money(10_000)).unwrap();
    let post = |fixture: &mut ConfirmationFixture, number, key| {
        let id = DraftId::new(number);
        let draft = ProvisionalDraft::new(id, vec![DraftLine::new(product, NonZeroU32::MIN)])
            .with_bill_date(Some(date))
            .with_customer(DraftCustomer::Selected {
                id: customer.clone(),
                name: "Fallback".into(),
            });
        fixture.register_draft(draft).unwrap();
        let review = fixture.review_named(Role::Owner, id, money(1_000)).unwrap();
        fixture
            .confirm_with_creation_key(Role::Owner, RequestId::new(number), &review, key)
            .unwrap();
    };
    let receipt = |amount| CustomerReceiptInput {
        customer_id: customer.clone(),
        date,
        amount: money(amount),
        mode: ReceiptMode::Cash,
    };
    post(&mut fixture, 1, None);
    fixture
        .record_customer_receipt(Role::Owner, receipt(123))
        .unwrap();
    post(&mut fixture, 2, Some(CreationKey::new(20)));
    post(&mut fixture, 3, None);
    fixture
        .record_customer_receipt(Role::Owner, receipt(456))
        .unwrap();
    fixture
        .record_customer_receipt_with_metadata(
            Role::Owner,
            receipt(345),
            alm_phase2_local::confirmation::CustomerReceiptMetadata {
                selected_name: "Fallback".into(),
                creation_key: CreationKey::new(5),
            },
        )
        .unwrap();
    let keyed = fixture
        .bill_preview(Role::Owner, DraftId::new(2), BalanceMode::AsOnBillDate)
        .unwrap();
    assert_eq!(keyed.previous_balance.unwrap().amount, money(17_076));
    assert_eq!(keyed.balance, money(26_076));
    let unkeyed = fixture
        .bill_preview(Role::Owner, DraftId::new(3), BalanceMode::AsOnBillDate)
        .unwrap();
    assert_eq!(unkeyed.previous_balance.unwrap().amount, money(17_877));
    assert_eq!(unkeyed.balance, money(26_877));
}
