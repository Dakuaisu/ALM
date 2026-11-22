use super::*;
use alm_phase2_local::confirmation::{
    CashBookEntry, CashBookKind, CustomerReceiptInput, SalesDetail, SalesPeriod, SalesReceiptId,
    SalesSource,
};
use std::cell::Cell;

#[test]
fn l14_03_library_save_replays_selected_names_creation_order_and_historical_rows() {
    let customers =
        [("twin-east", "Twin Market"), ("twin-west", "Twin Market")].map(|(id, name)| Customer {
            id: DraftCustomerId::new(id.into()),
            name: name.into(),
        });
    let mut fixture = ConfirmationFixture::new(PRODUCT, 1, money(2000)).unwrap();
    for customer in &customers {
        fixture.register_customer(OWNER, &customer.id).unwrap();
    }
    fixture
        .record_customer_receipt(
            OWNER,
            CustomerReceiptInput {
                customer_id: customers[0].id.clone(),
                date: receipt_date(),
                amount: money(90),
                mode: ReceiptMode::Cash,
            },
        )
        .unwrap();
    let historical = fixture
        .customer_ledger(OWNER, &customers[0].id)
        .unwrap()
        .entries[0]
        .clone();
    let key = Cell::new(0);
    let sampled = Cell::new(false);
    let mut form = ReceivePayment::with_clock(|| {
        sampled.set(true);
        Ok(CreationKey::new(key.get()))
    });
    let mut expected_cashbook = vec![CashBookEntry {
        kind: CashBookKind::Receipt,
        mode: ReceiptMode::Cash,
        amount: money(90),
    }];
    let mut cash = 90;
    let mut online = 0;
    for (keyboard, id, creation_key, amount, mode) in [
        (true, "twin-east", 1_700_000_000_020, 375, ReceiptMode::Cash),
        (
            false,
            "twin-west",
            1_700_000_000_010,
            800,
            ReceiptMode::Online,
        ),
        (
            false,
            "twin-west",
            1_700_000_000_030,
            125,
            ReceiptMode::Cash,
        ),
    ] {
        form.input("Twin Market", &customers);
        if keyboard {
            form.arrow_down();
            form.enter();
        } else {
            form.pick(1);
        }
        assert_eq!(form.selected().unwrap().id.value(), id);
        let before = customers
            .each_ref()
            .map(|customer| fixture.customer_ledger(OWNER, &customer.id).unwrap());
        key.set(creation_key);
        sampled.set(false);
        form.save(OWNER, &mut fixture, receipt_date(), money(amount), mode)
            .unwrap();
        assert!(sampled.get());
        for (customer, previous) in customers.iter().zip(&before) {
            let current = fixture.customer_ledger(OWNER, &customer.id).unwrap();
            if customer.id.value() == id {
                assert_eq!(current.entries.len(), previous.entries.len() + 1);
                assert_eq!(&current.entries[..previous.entries.len()], previous.entries);
                let receipt = current.entries.last().unwrap();
                assert_eq!(
                    (
                        receipt.date,
                        &receipt.kind,
                        receipt.debit,
                        receipt.credit,
                        receipt.balance
                    ),
                    (
                        receipt_date(),
                        &CustomerLedgerKind::Receipt { mode },
                        money(0),
                        money(amount),
                        money(previous.closing.minor_units() - amount)
                    )
                );
                assert_eq!(
                    current.closing,
                    money(previous.closing.minor_units() - amount)
                );
            } else {
                assert_eq!(&current, previous);
            }
        }
        expected_cashbook.push(CashBookEntry {
            kind: CashBookKind::Receipt,
            mode,
            amount: money(amount),
        });
        match mode {
            ReceiptMode::Cash => cash += amount,
            ReceiptMode::Online => online += amount,
        }
        let book = fixture.cashbook_day_close(OWNER, receipt_date()).unwrap();
        assert_eq!(book.entries, expected_cashbook);
        assert_eq!((book.cash, book.online), (money(cash), money(online)));
        println!(
            "l14_03_library_save_replays_selected_names_creation_order_and_historical_rows id={id} creation_key={creation_key} paise={amount} mode={mode:?} cash_paise={cash} online_paise={online}"
        );
    }
    let history = fixture
        .sales_history(OWNER, receipt_date(), SalesPeriod::Today, "tWiN")
        .unwrap();
    assert!(history.rows.is_empty());
    assert_eq!(
        history.collected.receipt_ids,
        [SalesReceiptId(2), SalesReceiptId(3), SalesReceiptId(4)]
    );
    assert_eq!(
        history.collected.details,
        vec![
            SalesDetail {
                source: SalesSource::Receipt(SalesReceiptId(4)),
                amount: money(125),
                mode: ReceiptMode::Cash
            },
            SalesDetail {
                source: SalesSource::Receipt(SalesReceiptId(2)),
                amount: money(375),
                mode: ReceiptMode::Cash
            },
            SalesDetail {
                source: SalesSource::Receipt(SalesReceiptId(3)),
                amount: money(800),
                mode: ReceiptMode::Online
            },
        ]
    );
    assert_eq!(history.total_collected, money(1300));
    let east = fixture.customer_ledger(OWNER, &customers[0].id).unwrap();
    let west = fixture.customer_ledger(OWNER, &customers[1].id).unwrap();
    assert_eq!(east.entries[0], historical);
    assert_eq!((east.closing, west.closing), (money(-465), money(-925)));
    assert_eq!(fixture.customer_due("twin-east"), money(-465));
    assert_eq!(fixture.customer_due("twin-west"), money(-925));
    let all = fixture
        .sales_history(OWNER, receipt_date(), SalesPeriod::Today, "")
        .unwrap();
    assert_eq!(all.total_collected, money(1390));
    assert_eq!(
        all.collected.details.last().unwrap().source,
        SalesSource::Receipt(SalesReceiptId(1))
    );
    println!(
        "l14_03_library_save_replays_selected_names_creation_order_and_historical_rows legacy receipt=1 paise=90; east=-465 west=-925; searched=1300 all=1390 paise"
    );
}
