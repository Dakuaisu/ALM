//! L02-01: source-derived Counter replay. Navigation and creation timestamps are
//! test-only; posting, cancellation, ledger readbacks and preview use the Rust fixture.

use std::cmp::Reverse;
use std::num::NonZeroU32;

use crate::confirmation::{
    BalanceMode, ConfirmationError, ConfirmationFixture, CustomerReceiptInput, ReceiptMode,
    RequestId,
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

struct Sale {
    draft: DraftId,
    invoice: u64,
    date: BillDate,
    cancelled: bool,
    modeled_created: Option<u64>,
}

impl Sale {
    const fn ids(&self) -> (u64, u64) {
        (self.draft.value(), self.invoice)
    }
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
    for (draft, date, _, buyer, paid) in seeds {
        let id = DraftId::new(draft);
        register_bill(&mut fixture, (id, date), buyer);
        let review = fixture
            .review_named(OWNER, id, Money::from_minor_units(paid))
            .unwrap();
        fixture
            .confirm(OWNER, RequestId::new(draft), &review)
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

    // Use existing Owner readbacks for the sale facts; only `created` comes from the model.
    // The preview readback here supplies cancellation state, not a modeled recent-row click.
    let sales = seeds.map(|(draft, _, modeled_created, _, _)| {
        let id = DraftId::new(draft);
        let (review, receipt) = fixture.current_sale(OWNER, id).unwrap();
        let saved = fixture
            .bill_preview(OWNER, id, BalanceMode::AsOnBillDate)
            .unwrap();
        Sale {
            draft: receipt.draft_id,
            invoice: receipt.invoice,
            date: review.draft().bill_date().unwrap(),
            cancelled: saved.cancelled,
            modeled_created,
        }
    });

    // When: project the Counter from the saved facts (legacy lines 2326-2331, 2406-2407).
    let mut active: Vec<_> = sales.iter().filter(|sale| !sale.cancelled).collect();
    let today_ids: Vec<_> = active
        .iter()
        .filter(|sale| sale.date == today)
        .map(|sale| sale.ids())
        .collect();
    assert_eq!(today_ids.len(), 4);
    assert_eq!(today_ids, [(51, 1), (84, 3), (63, 5), (72, 7)]);
    println!(
        "L02-01 test-only Counter {today}: billed today={} (draft, invoice)={today_ids:?}",
        today_ids.len()
    );

    active.sort_by_key(|sale| Reverse(sale.modeled_created.unwrap_or_default()));
    let (recent, trimmed) = active.split_at(active.len().min(6));
    let recent_ids: Vec<_> = recent.iter().map(|sale| sale.ids()).collect();
    assert_eq!(
        recent_ids,
        [(14, 2), (63, 5), (72, 7), (84, 3), (38, 6), (27, 4)]
    );
    let trimmed_ids: Vec<_> = trimmed.iter().map(|sale| sale.ids()).collect();
    assert_eq!(trimmed_ids, [(51, 1)]);
    let cancelled: Vec<_> = sales
        .iter()
        .filter(|sale| sale.cancelled)
        .map(|sale| (sale.ids(), sale.date, sale.modeled_created))
        .collect();
    assert_eq!(cancelled, [((95, 8), today, Some(80))]);
    let [(cancelled_ids, cancelled_date, cancelled_created)] = cancelled.as_slice() else {
        unreachable!("expected exactly one cancelled sale");
    };
    assert!(*cancelled_created > recent[0].modeled_created);
    println!(
        "L02-01 test-only created order (draft, invoice)={recent_ids:?}; trimmed={trimmed_ids:?}; cancelled={cancelled_ids:?} date={cancelled_date} modeled_created={cancelled_created:?}"
    );

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
    let debtor_count = balances.iter().filter(|(_, _, rupees)| *rupees > 0).count();
    assert_eq!(debtor_count, 1);
    let (view, query) = (debtor_count > 0)
        .then_some(("customers", "__due"))
        .unwrap();
    let due_rows: Vec<_> = balances
        .into_iter()
        .filter(|(_, _, rupees)| query != "__due" || *rupees > 0)
        .collect();
    assert_eq!((view, query), ("customers", "__due"));
    assert_eq!(due_rows, [("counter-due", 1251, 13)]);
    println!(
        "L02-01 Owner ledger (customer, paise, rounded rupees)={balances:?}; test-only debtor flag={debtor_count} -> {view}/{query} {due_rows:?}"
    );

    // Test-only recent-row selection models showInvoice(s), not any of its output actions.
    let selected = recent[0];
    let preview = fixture
        .bill_preview(OWNER, selected.draft, BalanceMode::AsOnBillDate)
        .unwrap();
    assert_eq!(
        (preview.invoice, preview.date, preview.cancelled),
        (2, backdate, false)
    );
    assert_eq!(preview.invoice, selected.invoice);
    assert_eq!(fixture, before);
    println!(
        "L02-01 test-only recent selection draft={} -> Owner bill_preview invoice={} date={}; fixture unchanged; no Print, PDF or WhatsApp calls in replay",
        selected.draft.value(),
        preview.invoice,
        preview.date
    );

    let manager_preview =
        fixture.bill_preview(Role::Manager, selected.draft, BalanceMode::AsOnBillDate);
    assert_eq!(manager_preview, Err(ConfirmationError::AccessDenied));
    assert_eq!(fixture, before);
    let manager_post = fixture.confirm(
        Role::Manager,
        RequestId::new(pending.value()),
        &pending_review,
    );
    assert_eq!(manager_post, Err(ConfirmationError::AccessDenied));
    assert_eq!(fixture, before);
    println!(
        "L02-01 Manager posted preview={:?}; posting draft {}={:?}; fixture unchanged",
        manager_preview.unwrap_err(),
        pending.value(),
        manager_post.unwrap_err()
    );
}
