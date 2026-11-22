//! L14-03 local picker and receipt replay; no browser event or collation claim.
use alm_phase2_local::confirmation::{
    ConfirmationError, ConfirmationFixture, CreationKey, CustomerLedger, CustomerLedgerEntry,
    CustomerLedgerKind, ReceiptMode, RequestId,
};
use alm_phase2_local::receive_payment::{Customer, ReceivePayment, SaveError};
use alm_phase2_local::{
    BillDate, DraftCustomer, DraftCustomerId, DraftId, DraftLine, Money, ProductId,
    ProvisionalDraft, Role,
};
use std::num::NonZeroU32;

#[path = "l14_receive_payment_replay/clock.rs"]
mod clock;
#[path = "l14_receive_payment_replay/receipt.rs"]
mod receipt;
#[path = "l14_receive_payment_replay/validation.rs"]
mod validation;

const OWNER: Role = Role::Owner;
const PRODUCT: ProductId = ProductId::new(1);
const fn money(units: i64) -> Money {
    Money::from_minor_units(units)
}
fn bill_date() -> BillDate {
    BillDate::new(2025, 3, 14).unwrap()
}
fn receipt_date() -> BillDate {
    BillDate::new(2025, 3, 17).unwrap()
}

fn seeded() -> (ConfirmationFixture, [Customer; 3]) {
    let customers = [
        ("twin-east", "Twin Market"),
        ("twin-west", "Twin Market"),
        ("solo", "Solo Market"),
    ]
    .map(|(id, name)| Customer {
        id: DraftCustomerId::new(id.into()),
        name: name.into(),
    });
    let mut fixture = ConfirmationFixture::new(PRODUCT, 8, money(2_000)).unwrap();
    for (customer, (invoice, paid)) in customers.iter().zip([(1, 325), (2, 510), (3, 275)]) {
        let draft_id = DraftId::new(invoice);
        let draft = ProvisionalDraft::new(draft_id, vec![DraftLine::new(PRODUCT, NonZeroU32::MIN)])
            .with_bill_date(Some(bill_date()))
            .with_customer(DraftCustomer::Selected {
                id: customer.id.clone(),
                name: customer.name.clone(),
            });
        fixture.register_draft(draft).unwrap();
        let review = fixture.review_named(OWNER, draft_id, money(paid)).unwrap();
        fixture
            .confirm(OWNER, RequestId::new(invoice), &review)
            .unwrap();
    }
    (fixture, customers)
}

#[derive(Debug, PartialEq, Eq)]
struct LedgerReadback {
    accounts: [(CustomerLedger, Money); 3],
    bill_totals: (Money, Money, usize),
}
fn readback(fixture: &ConfirmationFixture, customers: &[Customer; 3]) -> LedgerReadback {
    LedgerReadback {
        accounts: customers.each_ref().map(|customer| {
            (
                fixture.customer_ledger(OWNER, &customer.id).unwrap(),
                fixture.customer_due(customer.id.value()),
            )
        }),
        bill_totals: (
            fixture.ledger(),
            fixture.received_ledger(),
            fixture.posted_count(),
        ),
    }
}

#[test]
fn l14_03_unresolved_save_never_calls_receipt_writer() {
    let (mut fixture, customers) = seeded();
    let before = readback(&fixture, &customers);
    let twins = &["twin-east", "twin-west"][..];
    for (case, text, ids, prior_pick) in [
        ("wrong case", "solo market", &["solo"][..], false),
        ("duplicate", "Twin Market", twins, false),
        ("stale pick cleared", " Twin Market ", twins, true),
    ] {
        let mut form = ReceivePayment::with_clock(|| panic!("unresolved Save sampled clock"));
        if prior_pick {
            form.input("Twin Market", &customers);
            form.arrow_down();
            form.pick(1);
            assert_eq!(form.selected().unwrap().id.value(), "twin-west");
        }
        form.input(text, &customers);
        let suggestions: Vec<_> = form
            .suggestions
            .iter()
            .map(|customer| customer.id.value())
            .collect();
        assert_eq!(suggestions, ids);
        assert_eq!(form.highlighted, None);
        form.enter();
        let attempt = form.save(
            OWNER,
            &mut fixture,
            receipt_date(),
            money(725),
            ReceiptMode::Online,
        );
        let toast = attempt.unwrap_err().toast();
        assert_eq!(toast, Some("Pick a customer"));
        assert_eq!(readback(&fixture, &customers), before);
        assert!(
            fixture
                .cashbook_day_close(OWNER, receipt_date())
                .unwrap()
                .entries
                .is_empty()
        );
        println!(
            "L14-03 {case}: suggestions={suggestions:?} Enter=unresolved Save={toast:?} ledger unchanged"
        );
    }
}

#[derive(Debug)]
enum Pick {
    Keyboard,
    Pointer,
}

#[test]
fn l14_03_picked_duplicate_id_routes_save_to_only_that_ledger() {
    for pick in [Pick::Keyboard, Pick::Pointer] {
        let (mut fixture, customers) = seeded();
        let before = readback(&fixture, &customers);
        let mut form = ReceivePayment::with_clock(|| Ok(CreationKey::new(1_700_000_000_000)));
        form.input("Twin Market", &customers);
        let (id, mode, amount, invoice, [paid, opening, closing]) = match pick {
            Pick::Keyboard => {
                form.arrow_down();
                assert_eq!(form.highlighted, Some(0));
                form.enter();
                ("twin-east", ReceiptMode::Online, 725, 1, [325, 1_675, 950])
            }
            Pick::Pointer => {
                form.pick(1);
                ("twin-west", ReceiptMode::Cash, 430, 2, [510, 1_490, 1_060])
            }
        };
        assert_eq!(form.text, "Twin Market");
        let observed_id = form.selected().unwrap().id.value().to_owned();
        assert_eq!(observed_id, id);
        form.save(OWNER, &mut fixture, receipt_date(), money(amount), mode)
            .unwrap();
        let after = readback(&fixture, &customers);
        assert_eq!(after.bill_totals, (money(6_000), money(1_110), 3));
        let amounts = |row: &CustomerLedgerEntry| (row.debit, row.credit, row.balance);
        for (customer, ((old, old_due), (new, new_due))) in customers
            .iter()
            .zip(before.accounts.iter().zip(&after.accounts))
        {
            let delta = new.closing.minor_units() - old.closing.minor_units();
            if customer.id.value() == id {
                let [bill, receipt] = new.entries.as_slice() else {
                    panic!("expected one bill and one receipt for {id}");
                };
                assert_eq!(old.entries, std::slice::from_ref(bill));
                assert_eq!(bill.date, bill_date());
                assert_eq!(
                    bill.kind,
                    CustomerLedgerKind::Bill {
                        draft_id: DraftId::new(invoice),
                        invoice
                    }
                );
                assert_eq!(amounts(bill), (money(2_000), money(paid), money(opening)));
                assert_eq!(receipt.date, receipt_date());
                assert_eq!(receipt.kind, CustomerLedgerKind::Receipt { mode });
                assert_eq!(amounts(receipt), (money(0), money(amount), money(closing)));
                assert_eq!((old.closing, *old_due), (money(opening), money(opening)));
                assert_eq!((new.closing, *new_due), (money(closing), money(closing)));
                println!(
                    "L14-03 {pick:?}: chosen={observed_id} mode={mode:?} amount_minor={} closing_minor={} delta_minor={delta} bill_paid_minor={}",
                    receipt.credit.minor_units(),
                    new.closing.minor_units(),
                    bill.credit.minor_units()
                );
            } else {
                assert_eq!((new, new_due), (old, old_due));
                let other_id = customer.id.value();
                println!("L14-03 {pick:?}: other={other_id} delta_minor={delta}");
            }
        }
    }
}

#[test]
fn l14_03_manager_cannot_post_valid_trimmed_exact_selection() {
    let (mut fixture, customers) = seeded();
    let before = readback(&fixture, &customers);
    let mut form = ReceivePayment::with_clock(|| panic!("Manager Save sampled clock"));
    form.input("  Solo Market \t", &customers);
    assert_eq!(form.highlighted, None);
    let id = form.selected().unwrap().id.value().to_owned();
    assert_eq!(id, "solo");
    let amount = 310;
    let result = form.save(
        Role::Manager,
        &mut fixture,
        receipt_date(),
        money(amount),
        ReceiptMode::Cash,
    );
    assert_eq!(
        result,
        Err(SaveError::Confirmation(ConfirmationError::AccessDenied))
    );
    assert_eq!(readback(&fixture, &customers), before);
    assert!(
        fixture
            .cashbook_day_close(OWNER, receipt_date())
            .unwrap()
            .entries
            .is_empty()
    );
    println!(
        "L14-03 trimmed exact: chosen={id} amount_minor={amount} Manager={result:?} ledger unchanged"
    );
}
