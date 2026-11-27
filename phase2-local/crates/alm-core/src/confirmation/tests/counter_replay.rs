//! L02-01: source-derived Counter replay using the synthetic Rust fixture.

use std::num::NonZeroU32;

use crate::confirmation::{
    BalanceMode, ConfirmationError, ConfirmationFixture, CounterBill, CreationKey,
    CustomerReceiptInput, ReceiptMode, RequestId,
};
use crate::{
    BillDate, DraftCustomer, DraftCustomerId, DraftId, DraftLine, Money, ProductId,
    ProvisionalDraft, Role,
};

const OWNER: Role = Role::Owner;
const PRODUCT: ProductId = ProductId::new(1);
const PRICE: Money = Money::from_minor_units(4_000);
const CANCELLED: DraftId = DraftId::new(95);
const CUSTOMER_IDS: [&str; 4] = [
    "counter-due",
    "counter-zero",
    "counter-credit",
    "counter-sub-rupee",
];

fn register_bill(
    fixture: &mut ConfirmationFixture,
    (id, date): (DraftId, BillDate),
    buyer: &DraftCustomerId,
) {
    fixture
        .register_draft(
            ProvisionalDraft::new(id, vec![DraftLine::new(PRODUCT, NonZeroU32::MIN)])
                .with_bill_date(Some(date))
                .with_customer(DraftCustomer::Selected {
                    id: buyer.clone(),
                    name: format!("Synthetic {}", buyer.value()),
                }),
        )
        .unwrap();
}

#[test]
fn counter_keeps_first_post_key_on_retry_and_cancel_undo_and_stably_orders_missing_keys() {
    let today = BillDate::new(2025, 3, 14).unwrap();
    let mut fixture = ConfirmationFixture::new(PRODUCT, 8, PRICE).unwrap();
    let buyer = DraftCustomerId::new("tie-buyer".into());
    fixture.register_customer(OWNER, &buyer).unwrap();
    let [(first_review, first_ack), ..] = [(8, Some(9)), (2, None), (7, Some(0)), (3, Some(9))]
        .map(|(id, key)| {
            let draft = DraftId::new(id);
            register_bill(&mut fixture, (draft, today), &buyer);
            let review = fixture.review_named(OWNER, draft, PRICE).unwrap();
            let ack = fixture
                .confirm_acknowledged_with_creation_key(
                    OWNER,
                    RequestId::new(id),
                    &review,
                    key.map(CreationKey::new),
                )
                .unwrap();
            (review, ack)
        });
    let retry_ack = fixture
        .confirm_acknowledged_with_creation_key(
            OWNER,
            RequestId::new(8),
            &first_review,
            Some(CreationKey::new(99)),
        )
        .unwrap();
    assert_eq!(retry_ack, first_ack);
    fixture.cancel_sale(OWNER, DraftId::new(3)).unwrap();
    fixture.undo_cancel_sale(OWNER, DraftId::new(3)).unwrap();

    let recent_ids: Vec<_> = fixture
        .counter(OWNER, today)
        .unwrap()
        .recents
        .iter()
        .map(|row| row.draft_id.value())
        .collect();
    assert_eq!(recent_ids, [3, 8, 7, 2]);
    println!(
        "L02-01 Counter API recents after retry and cancel/undo: {recent_ids:?}; key 9 ties: 3, 8; zero/missing ties: 7, 2"
    );
}

#[test]
fn counter_includes_undated_active_sale_only_in_recents() {
    // Given: a stored sale without a document date (new posts still require one).
    let today = BillDate::new(2025, 3, 14).unwrap();
    let mut fixture = ConfirmationFixture::new(PRODUCT, 1, PRICE).unwrap();
    let buyer = DraftCustomerId::new("undated-buyer".into());
    fixture.register_customer(OWNER, &buyer).unwrap();
    let id = DraftId::new(10);
    register_bill(&mut fixture, (id, today), &buyer);
    let review = fixture.review_named(OWNER, id, PRICE).unwrap();
    fixture.confirm(OWNER, RequestId::new(10), &review).unwrap();
    fixture.current_sales.get_mut(&id).unwrap().0.draft =
        review.draft().clone().with_bill_date(None);

    // When: the Owner requests the Counter.
    let counter = fixture.counter(OWNER, today).unwrap();

    // Then: the sale remains in recents but never counts as billed today.
    assert_eq!(
        counter.recents,
        [CounterBill {
            draft_id: id,
            invoice: 1
        }]
    );
    assert_eq!(counter.billed_today_count, 0);
    assert!(counter.billed_today.is_empty());
}

#[test]
fn owner_counter_replay_keeps_recent_preview_read_only_and_manager_cannot_post() {
    // Given: a fixed reporting date and seven active sales plus one cancellation.
    let today = BillDate::new(2025, 3, 14).unwrap();
    let yesterday = BillDate::new(2025, 3, 13).unwrap();
    let backdate = BillDate::new(2025, 3, 12).unwrap();
    let mut fixture = ConfirmationFixture::new(PRODUCT, 12, PRICE).unwrap();
    let [due, zero, credit, sub_rupee] = CUSTOMER_IDS.map(|id| DraftCustomerId::new(id.into()));
    let customers = [&due, &zero, &credit, &sub_rupee];
    for id in customers {
        fixture.register_customer(OWNER, id).unwrap();
    }

    // Posting/invoice order deliberately differs from modeled creation order.
    // (draft ID, document date, modeled created, customer, paid paise)
    let seeds = [
        (51, today, None, &zero, 4_000),
        (14, backdate, Some(75), &due, 2_749),
        (84, today, Some(40), &zero, 4_000),
        (27, yesterday, Some(20), &credit, 4_000),
        (63, today, Some(60), &zero, 4_000),
        (38, yesterday, Some(30), &sub_rupee, 3_951),
        (72, today, Some(50), &zero, 4_000),
        (95, today, Some(80), &zero, 1_000),
    ];
    for (draft, date, created, buyer, paid) in seeds {
        let id = DraftId::new(draft);
        register_bill(&mut fixture, (id, date), buyer);
        let review = fixture
            .review_named(OWNER, id, Money::from_minor_units(paid))
            .unwrap();
        fixture
            .confirm_with_creation_key(
                OWNER,
                RequestId::new(draft),
                &review,
                created.map(CreationKey::new),
            )
            .unwrap();
    }
    fixture.cancel_sale(OWNER, CANCELLED).unwrap();
    fixture
        .record_customer_receipt(
            OWNER,
            CustomerReceiptInput {
                customer_id: credit.clone(),
                date: today,
                amount: Money::from_minor_units(850),
                mode: ReceiptMode::Cash,
            },
        )
        .unwrap();

    let pending = DraftId::new(96);
    register_bill(&mut fixture, (pending, today), &zero);
    let pending_review = fixture.review_named(OWNER, pending, PRICE).unwrap();
    let before = fixture.clone();

    // When: the Owner reads the Counter projection.
    let counter = fixture.counter(OWNER, today).unwrap();
    let today_ids: Vec<_> = counter
        .billed_today
        .iter()
        .map(|row| (row.draft_id.value(), row.invoice))
        .collect();
    assert_eq!(counter.billed_today_count, 4);
    assert_eq!(today_ids, [(51, 1), (84, 3), (63, 5), (72, 7)]);
    println!(
        "L02-01 Owner Counter API {today}: billed_today_count={} (draft, invoice)={today_ids:?}",
        counter.billed_today_count
    );

    let recent_ids: Vec<_> = counter
        .recents
        .iter()
        .map(|row| (row.draft_id.value(), row.invoice))
        .collect();
    assert_eq!(
        recent_ids,
        [(14, 2), (63, 5), (72, 7), (84, 3), (38, 6), (27, 4)]
    );
    println!("L02-01 Owner Counter API ordered recents (draft, invoice)={recent_ids:?}");

    // Legacy rounds the closing balance (Math.round, ties toward +infinity) before
    // counting debtors or filtering Customers (__due): lines 1114-1126, 2376-2395, 7151-7156.
    let balances = customers.map(|id| {
        let paise = fixture
            .customer_ledger(OWNER, id)
            .unwrap()
            .closing
            .minor_units();
        let rupees = paise.div_euclid(100) + i64::from(paise.rem_euclid(100) >= 50);
        (id.value(), paise, rupees)
    });
    assert_eq!(
        balances,
        [
            ("counter-due", 1251, 13),
            ("counter-zero", 0, 0),
            ("counter-credit", -850, -8),
            ("counter-sub-rupee", 49, 0),
        ]
    );
    let debtor_ids: Vec<_> = counter
        .debtor_ids
        .iter()
        .map(DraftCustomerId::value)
        .collect();
    assert_eq!(debtor_ids, ["counter-due"]);
    println!("L02-01 Owner Counter API debtor IDs={debtor_ids:?}");
    println!("L02-01 Owner ledger control (customer, paise, rounded rupees)={balances:?}");

    // Test-only recent-row selection models showInvoice(s), not any of its output actions.
    let selected = counter.recents[0];
    let preview = fixture
        .bill_preview(OWNER, selected.draft_id, BalanceMode::AsOnBillDate)
        .unwrap();
    assert_eq!(
        (preview.invoice, preview.date, preview.cancelled),
        (2, backdate, false)
    );
    assert_eq!(preview.invoice, selected.invoice);
    assert_eq!(fixture, before);
    println!(
        "L02-01 modeled recent-row selection draft={} -> Owner bill_preview invoice={} date={}; fixture unchanged; no Print, PDF or WhatsApp calls in replay",
        selected.draft_id.value(),
        preview.invoice,
        preview.date
    );

    let manager_preview =
        fixture.bill_preview(Role::Manager, selected.draft_id, BalanceMode::AsOnBillDate);
    let manager_counter = fixture.counter(Role::Manager, today);
    assert_eq!(manager_counter, Err(ConfirmationError::AccessDenied));
    // DEC-034: a Manager reads the posted bill (Counter recents, edit, print).
    assert_eq!(manager_preview, Ok(preview.clone()));
    assert_eq!(fixture, before);
    let manager_post = fixture.confirm(
        Role::Manager,
        RequestId::new(pending.value()),
        &pending_review,
    );
    assert_eq!(manager_post, Err(ConfirmationError::AccessDenied));
    assert_eq!(fixture, before);
    println!(
        "L02-01 Manager Counter API={:?} (no payload); posted preview invoice={:?}; posting draft {}={:?}; fixture unchanged",
        manager_counter.unwrap_err(),
        manager_preview.map(|preview| preview.invoice),
        pending.value(),
        manager_post.unwrap_err()
    );
}
