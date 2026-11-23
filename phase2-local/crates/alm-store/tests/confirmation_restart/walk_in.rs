use super::*;
use crate::cash_draft;
use alm_store::DurableDraftStore;
use alm_store::confirmation::{ConfirmationAck, PendingError};
use alm_store::draft_entry::{
    ActiveBillForm, BillEntryState, ConfirmationAttempt, ConfirmationOutcome,
};

const BILL: u64 = 81;
const UNRELATED: u64 = 82;
const REJECTED: u64 = 83;
const REQUEST: RequestId = RequestId::new(91);
const REJECTION: RequestId = RequestId::new(92);

#[test]
fn walk_in_cash_lost_reply_reopens_inertly_and_only_matching_retry_clears_entry() {
    let scratch = Scratch::new();
    let dir = &scratch.0;
    let db = dir.join(DATABASE);
    let product = ProductId::new(1);
    let mut authority = ConfirmationFixture::with_ordered_lots(
        product,
        &[
            (1, Money::from_minor_units(30)),
            (4, Money::from_minor_units(50)),
        ],
        Money::from_minor_units(125),
    )
    .unwrap();
    let draft = cash_draft(BILL, 2);
    authority.register_draft(draft.clone()).unwrap();
    let approved = OwnerRequest {
        id: REQUEST,
        review: authority
            .review_walk_in_cash(Role::Owner, draft.id())
            .unwrap(),
    };
    let expected = Receipt {
        draft_id: draft.id(),
        invoice: 1,
        quantity: 2,
        total: Money::from_minor_units(250),
        received: Some(Money::from_minor_units(250)),
        due: Some(Money::from_minor_units(0)),
        customer_balance: None,
    };

    assert!(!db.exists());
    let mut client = Client::start("walk_in_submit", dir);
    client.write_frame(&approved.encode());
    assert_eq!(client.sent(), approved);
    let store = alm_store::SqliteDraftStore::open(&db).unwrap();
    let original_pending = store.load_pending(REQUEST.0).unwrap().unwrap();
    assert_eq!(
        original_pending,
        "7|250|6|2024-02-29|2|81|1|0|125|1,,4C6F63616C202D204E65696768626F72|1,2"
    );
    assert_eq!(effects(&authority), (5, 0, 1, 0, 0));
    assert_eq!(authority.received_ledger().minor_units(), 0);
    assert!(authority.catalog.allocation_snapshot().is_empty());
    let original = authority
        .confirm_acknowledged(Role::Owner, approved.id, &approved.review)
        .unwrap();
    assert_eq!(original.receipt(), &expected);
    assert_eq!(client.finish(), ["MISSING"]);

    assert_eq!(Client::start("walk_in_reopen", dir).finish(), ["REOPENED"]);
    assert_eq!(Client::start("walk_in_denied", dir).finish(), ["DENIED"]);
    assert_eq!(
        authority.confirm_acknowledged(Role::Manager, approved.id, &approved.review),
        Err(ConfirmationError::AccessDenied)
    );
    authority
        .set_unit_price(Money::from_minor_units(400))
        .unwrap();
    for mode in ["walk_in_retry", "walk_in_edit"] {
        if mode == "walk_in_edit" {
            open_sales(&db)
                .session(Role::Owner)
                .save_draft(cash_draft(BILL, 3))
                .unwrap();
        }
        let mut retry = Client::start(mode, dir);
        let resent = retry.sent();
        assert_eq!(resent, approved);
        let replay = authority
            .confirm_acknowledged(Role::Owner, resent.id, &resent.review)
            .unwrap();
        assert_eq!(replay, original);
        write_ack(&mut retry, &replay);
        assert_eq!(
            retry.finish(),
            [if mode == "walk_in_retry" {
                format!("PREVIEW {}", receipt_frame(&expected))
            } else {
                "EDIT_KEPT".to_owned()
            }]
        );
    }
    assert_eq!(
        store.load_pending(REQUEST.0).unwrap(),
        Some(original_pending)
    );

    let rejection = cash_draft(REJECTED, 4);
    authority.register_draft(rejection.clone()).unwrap();
    let rejected = OwnerRequest {
        id: REJECTION,
        review: authority
            .review_walk_in_cash(Role::Owner, rejection.id())
            .unwrap(),
    };
    let mut client = Client::start("walk_in_reject", dir);
    client.write_frame(&rejected.encode());
    assert_eq!(client.sent(), rejected);
    assert_eq!(
        authority.confirm_acknowledged(Role::Owner, rejected.id, &rejected.review),
        Err(ConfirmationError::InsufficientStock)
    );
    client.write_frame("REJECT_STOCK");
    assert_eq!(client.finish(), ["REJECTED"]);

    assert_eq!(effects(&authority), (3, 250, 2, 1, 1));
    assert_eq!(authority.received_ledger().minor_units(), 250);
    assert!(authority.customer_balances.is_empty());
    assert_eq!(authority.catalog.lot_snapshot(product), [0, 3]);
    assert_eq!(
        authority.catalog.allocation_snapshot(),
        [(BILL, 0, 0, 1, Some(30)), (BILL, 0, 1, 1, Some(50))]
    );
    assert_eq!(authority.catalog.allocated_cost(draft.id()), Some(Some(80)));
}

pub(super) fn worker(
    mode: &str,
    database: &Path,
    input: &mut impl Iterator<Item = io::Result<String>>,
) {
    let mut sales = open_sales(database);
    let role = if mode == "walk_in_denied" {
        Role::Manager
    } else {
        Role::Owner
    };
    let mut session = sales.session(role);
    let unrelated = draft(UNRELATED, 1);
    let approval = match mode {
        "walk_in_submit" | "walk_in_reject" => {
            let approval = OwnerRequest::decode(&input.next().unwrap().unwrap());
            if mode == "walk_in_submit" {
                session.save_draft(unrelated.clone()).unwrap();
            }
            session.save_draft(approval.review.draft().clone()).unwrap();
            Some(approval)
        }
        _ => None,
    };
    let id = approval
        .as_ref()
        .map_or(DraftId::new(BILL), |request| request.review.draft().id());
    let saved = session.reopen_draft(id).unwrap().unwrap();
    let mut entry = BillEntryState::default();
    entry.open(&ActiveBillForm::reopen(&saved));
    assert_eq!(entry.active_draft(), Some(&saved));
    assert_eq!(entry.preview(), None);

    if mode == "walk_in_reopen" {
        assert_eq!(saved, cash_draft(BILL, 2));
        emit("REOPENED");
        return;
    }

    let attempt = match &approval {
        Some(approval) => ConfirmationAttempt::Submit {
            request: approval.id,
            review: &approval.review,
        },
        None => ConfirmationAttempt::Retry { request: REQUEST },
    };
    let outcome = entry.confirm(&mut session, attempt, |id, review| {
        assert_ne!(role, Role::Manager);
        send(&OwnerRequest {
            id,
            review: review.clone(),
        });
        read_ack(input)
    });
    let event = match mode {
        "walk_in_retry" => {
            let ConfirmationOutcome::Acknowledged(receipt) = outcome.unwrap() else {
                panic!("expected acknowledgement")
            };
            assert_eq!(entry.preview(), Some(&receipt));
            assert_eq!(entry.active_draft(), None);
            format!("PREVIEW {}", receipt_frame(entry.preview().unwrap()))
        }
        mode => {
            assert_eq!(entry.preview(), None);
            assert_eq!(entry.active_draft(), Some(&saved));
            match mode {
                "walk_in_submit" => {
                    assert_eq!(outcome.unwrap(), ConfirmationOutcome::Missing);
                    "MISSING".to_owned()
                }
                "walk_in_denied" => {
                    assert!(matches!(outcome, Err(PendingError::AccessDenied)));
                    "DENIED".to_owned()
                }
                "walk_in_edit" => {
                    assert_eq!(saved, cash_draft(BILL, 3));
                    assert!(matches!(
                        outcome.unwrap(),
                        ConfirmationOutcome::Acknowledged(_)
                    ));
                    "EDIT_KEPT".to_owned()
                }
                "walk_in_reject" => {
                    assert_eq!(
                        outcome.unwrap(),
                        ConfirmationOutcome::Rejected(ConfirmationError::InsufficientStock)
                    );
                    "REJECTED".to_owned()
                }
                _ => unreachable!(),
            }
        }
    };
    assert_eq!(session.reopen_draft(id).unwrap(), Some(saved));
    assert_eq!(
        session.reopen_draft(unrelated.id()).unwrap(),
        Some(unrelated)
    );
    emit(&event);
}

fn write_ack(client: &mut Client, ack: &ConfirmationAck) {
    let request = OwnerRequest {
        id: ack.request,
        review: ack.review.clone(),
    };
    client.write_frame(&format!(
        "{}\t{}",
        request.encode(),
        receipt_frame(ack.receipt())
    ));
}

// Only the subprocess harness decodes its trusted parent pipe into the opaque ack.
fn read_ack(
    input: &mut impl Iterator<Item = io::Result<String>>,
) -> Result<Option<ConfirmationAck>, ConfirmationError> {
    let Some(frame) = input.next() else {
        return Ok(None);
    };
    let frame = frame.unwrap();
    if frame == "REJECT_STOCK" {
        return Err(ConfirmationError::InsufficientStock);
    }
    let (request, receipt) = frame.split_once('\t').unwrap();
    let request = OwnerRequest::decode(request);
    Ok(Some(ConfirmationAck {
        request: request.id,
        review: request.review,
        receipt: read_receipt(receipt).unwrap(),
    }))
}
