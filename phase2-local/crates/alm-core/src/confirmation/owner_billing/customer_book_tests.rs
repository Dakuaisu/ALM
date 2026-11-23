use super::{BalanceSide, CustomerFilter};
use crate::confirmation::demo_fixture::{directory, fixture};
use crate::confirmation::owner_billing::{ReceiptTerms, save_receipt};
use crate::confirmation::{ConfirmationError, OpeningBalance, ReceiptMode};
use crate::{DraftCustomerId, Money, Role};

fn receipt(amount: &str) -> ReceiptTerms<'_> {
    ReceiptTerms {
        date: "2025-05-01",
        amount,
        mode: ReceiptMode::Cash,
    }
}

#[test]
fn book_totals_follow_the_opening_aware_ledger_and_the_counter_debtor_rule() {
    let mut authority = fixture().unwrap();
    let directory = directory();
    authority
        .save_customer_opening(
            Role::Owner,
            &DraftCustomerId::new("batch1-market".into()),
            OpeningBalance {
                amount: Some(Money::from_minor_units(25_050)),
                date: None,
            },
        )
        .unwrap();
    save_receipt(
        &mut authority,
        &directory,
        Role::Owner,
        "batch1-customer",
        receipt("0.49"),
    )
    .unwrap();
    let before = authority.clone();
    let book = directory
        .customer_book(&authority, Role::Owner, &CustomerFilter::All)
        .unwrap();
    assert_eq!(authority, before);
    assert_eq!(book.customers, 2);
    assert_eq!(book.receivable.minor_units(), 25_050);
    assert_eq!(book.advances.minor_units(), 49);
    assert_eq!(book.debtors, 1);
    let rows: Vec<_> = book
        .rows
        .iter()
        .map(|row| (row.customer.name, row.side))
        .collect();
    assert_eq!(
        rows,
        [
            (
                "Synthetic Customer",
                BalanceSide::Advance(Money::from_minor_units(49))
            ),
            (
                "Synthetic Market",
                BalanceSide::Due(Money::from_minor_units(25_050))
            ),
        ]
    );
    let top: Vec<_> = book
        .top_debtors(6)
        .iter()
        .map(|row| row.customer.id)
        .collect();
    assert_eq!(top, ["batch1-market"]);
    assert!(book.top_debtors(0).is_empty());
    let due = directory
        .customer_book(&authority, Role::Owner, &CustomerFilter::Due)
        .unwrap();
    assert_eq!(due.rows.len(), 1);
    assert_eq!(due.receivable, book.receivable);
    let named = directory
        .customer_book(
            &authority,
            Role::Owner,
            &CustomerFilter::Name(" MARKET ".into()),
        )
        .unwrap();
    assert_eq!(named.rows[0].customer.id, "batch1-market");
    assert_eq!(
        directory.customer_book(&authority, Role::Manager, &CustomerFilter::All),
        Err(ConfirmationError::AccessDenied)
    );
}

#[test]
fn balance_side_keeps_the_exact_paise() {
    assert_eq!(
        BalanceSide::of(Money::from_minor_units(0)),
        Ok(BalanceSide::Settled)
    );
    assert_eq!(
        BalanceSide::of(Money::from_minor_units(1)),
        Ok(BalanceSide::Due(Money::from_minor_units(1)))
    );
    assert_eq!(
        BalanceSide::of(Money::from_minor_units(i64::MIN)),
        Err(ConfirmationError::Overflow)
    );
}
