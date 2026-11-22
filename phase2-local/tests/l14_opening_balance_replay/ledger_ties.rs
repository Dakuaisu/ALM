use std::num::NonZeroU32;

use alm_phase2_local::confirmation::{
    BalanceMode, ConfirmationFixture, CreationKey, CustomerLedgerKind, CustomerReceiptInput,
    CustomerReceiptMetadata, ReceiptMode, RequestId,
};
use alm_phase2_local::{
    BillDate, DraftCustomer, DraftCustomerId, DraftId, DraftLine, Money, ProductId,
    ProvisionalDraft, Role,
};

#[test]
fn missing_zero_and_one_ledger_keys_tie_in_source_order_but_preview_uses_zero() {
    let money = Money::from_minor_units;
    let date: BillDate = "2025-02-10".parse().unwrap();
    let customer = DraftCustomerId::new("tie-customer".into());
    let product = ProductId::new(1);
    let mut fixture = ConfirmationFixture::new(product, 4, money(100)).unwrap();
    for (number, key) in [(1, None), (2, Some(0)), (3, Some(1)), (4, Some(2))] {
        let id = DraftId::new(number);
        let draft = ProvisionalDraft::new(id, vec![DraftLine::new(product, NonZeroU32::MIN)])
            .with_bill_date(Some(date))
            .with_customer(DraftCustomer::Selected {
                id: customer.clone(),
                name: "Tie".into(),
            });
        fixture.register_draft(draft).unwrap();
        let review = fixture.review_named(Role::Owner, id, money(0)).unwrap();
        fixture
            .confirm_with_creation_key(
                Role::Owner,
                RequestId::new(number),
                &review,
                key.map(CreationKey::new),
            )
            .unwrap();
    }
    for (amount, key) in [(101, None), (102, Some(0)), (103, Some(1))] {
        let input = CustomerReceiptInput {
            customer_id: customer.clone(),
            date,
            amount: money(amount),
            mode: ReceiptMode::Cash,
        };
        if let Some(key) = key {
            fixture
                .record_customer_receipt_with_metadata(
                    Role::Owner,
                    input,
                    CustomerReceiptMetadata {
                        selected_name: "Tie".into(),
                        creation_key: CreationKey::new(key),
                    },
                )
                .unwrap();
        } else {
            fixture.record_customer_receipt(Role::Owner, input).unwrap();
        }
    }
    let ledger = fixture.customer_ledger(Role::Owner, &customer).unwrap();
    let rows: Vec<_> = ledger
        .entries
        .iter()
        .map(|entry| {
            (
                entry.kind.clone(),
                entry.debit.minor_units(),
                entry.credit.minor_units(),
                entry.balance.minor_units(),
            )
        })
        .collect();
    let bill = |id| CustomerLedgerKind::Bill {
        draft_id: DraftId::new(id),
        invoice: id,
    };
    let receipt = CustomerLedgerKind::Receipt {
        mode: ReceiptMode::Cash,
    };
    assert_eq!(
        rows,
        vec![
            (bill(3), 100, 0, 100),
            (bill(2), 100, 0, 200),
            (bill(1), 100, 0, 300),
            (receipt.clone(), 0, 103, 197),
            (receipt.clone(), 0, 102, 95),
            (receipt, 0, 101, -6),
            (bill(4), 100, 0, 94),
        ]
    );
    let zero = fixture
        .bill_preview(Role::Owner, DraftId::new(2), BalanceMode::AsOnBillDate)
        .unwrap();
    assert_eq!(zero.previous_balance, None);
    assert_eq!(zero.balance, money(100));
    let one = fixture
        .bill_preview(Role::Owner, DraftId::new(3), BalanceMode::AsOnBillDate)
        .unwrap();
    assert_eq!(one.previous_balance, None);
    assert_eq!(one.balance, money(100));
    println!("tie ledger rows (kind, debit, credit, running paise): {rows:?}");
    println!("tie previews zero={zero:?} one={one:?}");
}
