use alm_phase2_local::confirmation::{
    ConfirmationError, ConfirmationFixture, CreationKey, CustomerReceiptInput,
    CustomerReceiptMetadata, ReceiptMode, SalesDetail, SalesPeriod, SalesReceiptId, SalesSource,
};
use alm_phase2_local::{BillDate, DraftCustomerId, Money, ProductId, Role};

#[test]
fn same_name_receipts_use_saved_names_and_keys_without_merging_accounts() {
    let mut fixture =
        ConfirmationFixture::new(ProductId::new(1), 5, Money::from_minor_units(125)).unwrap();
    let day = BillDate::new(2024, 3, 30).unwrap();
    let first = DraftCustomerId::new("first".into());
    let second = DraftCustomerId::new("second".into());
    for id in [&first, &second] {
        fixture.register_customer(Role::Owner, id).unwrap();
    }
    for (id, amount, mode, key) in [
        (first.clone(), 375, ReceiptMode::Cash, 20),
        (second.clone(), 800, ReceiptMode::Online, 10),
        (second.clone(), 125, ReceiptMode::Cash, 30),
    ] {
        fixture
            .record_customer_receipt_with_metadata(
                Role::Owner,
                CustomerReceiptInput {
                    customer_id: id,
                    date: day,
                    amount: Money::from_minor_units(amount),
                    mode,
                },
                CustomerReceiptMetadata {
                    selected_name: "Shared Name".into(),
                    creation_key: CreationKey::new(key),
                },
            )
            .unwrap();
    }
    let history = fixture
        .sales_history(Role::Owner, day, SalesPeriod::Today, "shared")
        .unwrap();
    assert!(history.rows.is_empty());
    assert_eq!(
        history.collected.receipt_ids,
        [SalesReceiptId(1), SalesReceiptId(2), SalesReceiptId(3)]
    );
    let item = fixture
        .sales_history(Role::Owner, day, SalesPeriod::Today, "LiMe")
        .unwrap();
    assert!(item.rows.is_empty());
    assert!(item.collected.receipt_ids.is_empty());
    assert_eq!(item.total_collected, Money::from_minor_units(0));
    assert_eq!(
        (
            history.sales,
            history.bill_paid,
            history.receipt_paid,
            history.total_collected
        ),
        (
            Money::from_minor_units(0),
            Money::from_minor_units(0),
            Money::from_minor_units(1300),
            Money::from_minor_units(1300)
        )
    );
    assert_eq!(
        history.collected.details,
        vec![
            SalesDetail {
                source: SalesSource::Receipt(SalesReceiptId(3)),
                amount: Money::from_minor_units(125),
                mode: ReceiptMode::Cash
            },
            SalesDetail {
                source: SalesSource::Receipt(SalesReceiptId(1)),
                amount: Money::from_minor_units(375),
                mode: ReceiptMode::Cash
            },
            SalesDetail {
                source: SalesSource::Receipt(SalesReceiptId(2)),
                amount: Money::from_minor_units(800),
                mode: ReceiptMode::Online
            },
        ]
    );
    let book = fixture.cashbook_day_close(Role::Owner, day).unwrap();
    assert_eq!(
        (history.collected.cash, history.collected.online),
        (book.cash, book.online)
    );
    assert_eq!(
        (book.cash, book.online),
        (Money::from_minor_units(500), Money::from_minor_units(800))
    );
    assert_eq!(
        fixture
            .customer_ledger(Role::Owner, &first)
            .unwrap()
            .closing,
        Money::from_minor_units(-375)
    );
    assert_eq!(
        fixture
            .customer_ledger(Role::Owner, &second)
            .unwrap()
            .closing,
        Money::from_minor_units(-925)
    );
    assert_eq!(
        fixture.sales_history(Role::Manager, day, SalesPeriod::Today, "shared"),
        Err(ConfirmationError::AccessDenied)
    );
    assert_eq!(
        fixture.sales_history(Role::Owner, day, SalesPeriod::Today, "shared"),
        Ok(history.clone())
    );
    println!(
        "{day} same-name accounts first=-375p second=-925p; Sales={:?}; collected={:?}; Cash Book cash={:?} online={:?}; item-only={:?}",
        history.sales, history.collected, book.cash, book.online, item.collected.receipt_ids
    );
}
