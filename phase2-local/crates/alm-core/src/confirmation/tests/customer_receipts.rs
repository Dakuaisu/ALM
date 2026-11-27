use super::*;
use crate::confirmation::{CashBookEntry, CashBookKind, CustomerLedgerEntry, CustomerLedgerKind};

#[test]
fn standalone_receipts_settle_opening_without_a_sale() {
    let mut f = ConfirmationFixture::new(ProductId::new(1), 5, money(125)).unwrap();
    let target = customer("receipt-target");
    let control = customer("receipt-control");
    for id in [&target, &control] {
        f.register_customer(Role::Owner, id).unwrap();
    }
    f.save_customer_opening(
        Role::Owner,
        &target,
        OpeningBalance {
            amount: Some(money(1_000)),
            date: None,
        },
    )
    .unwrap();
    let before = f.clone();
    let control_ledger = f
        .customer_ledger_with_opening(Role::Owner, &control)
        .unwrap();
    let partial = date("2024-03-30");
    let overpayment = date("2024-03-31");
    let ageing_day = date("2024-04-01");
    let mut rows = Vec::new();

    for (day, amount, mode, closing, ageing_due) in [
        (partial, 375, ReceiptMode::Cash, 625, Some(625)),
        (overpayment, 800, ReceiptMode::Online, -175, None),
    ] {
        f.record_customer_receipt(
            Role::Owner,
            CustomerReceiptInput {
                customer_id: target.clone(),
                date: day,
                amount: money(amount),
                mode,
            },
        )
        .expect("Owner can post a receipt against a registered account without a sale");

        rows.push(CustomerLedgerEntry {
            date: day,
            kind: CustomerLedgerKind::Receipt { mode },
            debit: money(0),
            credit: money(amount),
            balance: money(closing),
        });
        let ledger = f.customer_ledger(Role::Owner, &target).unwrap();
        let with_opening = f
            .customer_ledger_with_opening(Role::Owner, &target)
            .unwrap();
        let ageing = f.customer_ageing(Role::Owner, ageing_day).unwrap();
        println!("{day:?}: ledger={with_opening:?}, ageing={ageing:?}");
        assert_eq!(ledger.entries, rows);
        assert_eq!(ledger.closing, money(closing));
        assert_eq!(with_opening.closing, money(closing));
        assert_eq!(with_opening.entries.len(), rows.len() + 1);
        assert_eq!(with_opening.entries[0].kind, OpeningLedgerKind::Opening);
        assert_eq!(with_opening.entries[0].debit, money(1_000));
        assert_eq!(f.customer_due(target.value()), money(closing));
        assert_eq!(
            ageing.rows,
            ageing_due
                .map(|due| AgeingRow {
                    customer_id: target.clone(),
                    draft_id: None,
                    invoice: None,
                    date: None,
                    age: 999,
                    bucket: AgeingBucket::Old,
                    due: money(due),
                })
                .into_iter()
                .collect::<Vec<_>>()
        );
        assert_eq!(
            (ageing.d15, ageing.d30, ageing.d60, ageing.old),
            (
                money(0),
                money(0),
                money(0),
                money(ageing_due.unwrap_or_default())
            )
        );
        assert_eq!(
            f.customer_ledger_with_opening(Role::Owner, &control),
            Ok(control_ledger.clone())
        );
        assert_eq!(f.customer_due(control.value()), money(0));
        assert_eq!(f.catalog, before.catalog);
        assert_eq!(f.next_invoice, before.next_invoice);
        assert_eq!(f.current_sales, before.current_sales);
        assert_eq!(f.posted, before.posted);
    }

    for (day, expected, cash, online) in [
        (partial, Some((ReceiptMode::Cash, 375)), 375, 0),
        (overpayment, Some((ReceiptMode::Online, 800)), 0, 800),
        (ageing_day, None, 0, 0),
    ] {
        let book = f.cashbook_day_close(Role::Owner, day).unwrap();
        println!("{day:?}: Cash Book={book:?}");
        let entries: Vec<_> = expected
            .into_iter()
            .map(|(mode, amount)| CashBookEntry {
                kind: CashBookKind::Receipt,
                mode,
                amount: money(amount),
            })
            .collect();
        assert_eq!(book.count, entries.len());
        assert_eq!(book.entries, entries);
        assert_eq!((book.cash, book.online), (money(cash), money(online)));
    }
    println!(
        "unchanged control={control_ledger:?}; stock={}; next invoice={}",
        f.stock(),
        f.next_invoice
    );
}

#[test]
fn standalone_receipts_reject_invalid_attempts_without_mutation() {
    let mut f = ConfirmationFixture::new(ProductId::new(1), 5, money(125)).unwrap();
    let target = customer("receipt-target");
    f.register_customer(Role::Owner, &target).unwrap();
    let before = f.clone();

    for (role, customer_id, amount, error) in [
        (
            Role::Owner,
            customer("unknown"),
            375,
            ConfirmationError::InvalidDraft,
        ),
        (
            Role::Owner,
            target.clone(),
            0,
            ConfirmationError::InvalidAmount,
        ),
        // DEC-034: a Manager's receipt is checked like the Owner's.
        (Role::Manager, target, 0, ConfirmationError::InvalidAmount),
    ] {
        let result = f.record_customer_receipt(
            role,
            CustomerReceiptInput {
                customer_id,
                date: date("2024-03-30"),
                amount: money(amount),
                mode: ReceiptMode::Cash,
            },
        );
        println!("{role:?} receipt ({amount} paise): {result:?}");
        assert_eq!(result, Err(error));
        assert_eq!(f, before);
    }
}
