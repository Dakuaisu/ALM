use super::*;
use std::process::Command;

#[test]
fn mixed_approval_survives_client_restart_and_replays_after_price_change() {
    use crate::DraftLine;
    use std::num::NonZeroU32;
    let scratch = Scratch::new();
    let dir = &scratch.0;
    let mut authority =
        ConfirmationFixture::new(ProductId::new(1), 8, Money::from_minor_units(125)).unwrap();
    authority
        .add_product(ProductId::new(2), 5, Money::from_minor_units(210))
        .unwrap();
    let draft = crate::ProvisionalDraft::new(
        DraftId::new(43),
        [(1, 2), (2, 3), (1, 1)]
            .into_iter()
            .map(|(id, qty)| DraftLine::new(ProductId::new(id), NonZeroU32::new(qty).unwrap()))
            .collect(),
    );
    let approved = OwnerRequest {
        id: RequestId::new(987),
        review: authority.reconnect_review(Role::Owner, draft).unwrap(),
    };
    let mut first = Client::start("sqlite_submit", dir);
    first.write_frame(&approved.encode());
    let sent = first.sent();
    assert_eq!(sent, approved);
    let receipt = authority
        .confirm(Role::Owner, sent.id, &sent.review)
        .unwrap();
    first.write_frame("CONTINUE");
    assert_eq!(first.finish(), ["REPLY_EOF"]);
    assert_eq!(receipt.total, Money::from_minor_units(1005));
    let committed = (
        effects(&authority),
        authority.product_stock(ProductId::new(2)),
    );
    assert_eq!(committed, ((5, 1005, 2, 1, 1), Some(2)));
    authority
        .set_product_price(ProductId::new(2), Money::from_minor_units(400))
        .unwrap();
    let changed = authority.review(Role::Owner, DraftId::new(43)).unwrap();
    assert_eq!(
        authority.confirm(Role::Owner, approved.id, &changed),
        Err(ConfirmationError::RequestConflict)
    );
    assert_eq!(Client::start("sqlite_reopen", dir).finish(), ["REOPENED"]);
    assert_eq!(Client::start("sqlite_manager", dir).finish(), ["DENIED"]);
    let mut retry = Client::start("sqlite_retry", dir);
    let resent = retry.sent();
    assert_eq!(resent, approved);
    let replay = authority
        .confirm(Role::Owner, resent.id, &resent.review)
        .unwrap();
    assert_eq!(replay, receipt);
    retry.write_frame(&receipt_frame(&replay));
    assert_eq!(
        retry.finish(),
        [format!("RECEIVED {}", receipt_frame(&receipt))]
    );
    assert_eq!(
        (
            effects(&authority),
            authority.product_stock(ProductId::new(2))
        ),
        committed
    );
}

#[test]
fn sqlite_approval_survives_restart_and_replays_against_live_authority() {
    let scratch = Scratch::new();
    let dir = &scratch.0;
    let database = dir.join(DATABASE);
    let mut authority =
        ConfirmationFixture::new(ProductId::new(1), 8, Money::from_minor_units(77)).unwrap();
    authority.register_draft(draft(43, 1)).unwrap();
    authority.register_draft(draft(43, 3)).unwrap();
    authority
        .set_unit_price(Money::from_minor_units(193))
        .unwrap();
    let approved = OwnerRequest {
        id: RequestId::new(987),
        review: authority.review(Role::Owner, DraftId::new(43)).unwrap(),
    };

    let mut first = Client::start("sqlite_submit", dir);
    first.write_frame(&approved.encode());
    let sent = first.sent();
    assert_eq!(sent, approved);
    assert_no_sidecar(dir);
    let mut observer = open_sales(&database);
    observer
        .session(Role::Owner)
        .retry_pending(approved.id, |id, review| {
            assert_eq!((id, review), (approved.id, &approved.review));
        })
        .unwrap();
    let receipt = authority
        .confirm(Role::Owner, sent.id, &sent.review)
        .unwrap();
    first.write_frame("CONTINUE");
    assert_eq!(first.finish(), ["REPLY_EOF"]);
    let effects_after_sale = effects(&authority);
    assert_eq!(effects_after_sale, (5, 579, 2, 1, 1));
    authority
        .set_unit_price(Money::from_minor_units(888))
        .unwrap();

    assert_eq!(Client::start("sqlite_reopen", dir).finish(), ["REOPENED"]);
    assert_eq!(Client::start("sqlite_manager", dir).finish(), ["DENIED"]);
    assert_eq!(effects(&authority), effects_after_sale);
    let mut retry = Client::start("sqlite_retry", dir);
    let resent = retry.sent();
    assert_eq!(resent, approved);
    let replay = authority
        .confirm(Role::Owner, resent.id, &resent.review)
        .unwrap();
    assert_eq!(replay, receipt);
    retry.write_frame(&receipt_frame(&replay));
    assert_eq!(
        retry.finish(),
        [format!("RECEIVED {}", receipt_frame(&receipt))]
    );
    assert_eq!(effects(&authority), effects_after_sale);
    assert_no_sidecar(dir);
}

fn assert_no_sidecar(dir: &Path) {
    assert!(
        matches!(fs::symlink_metadata(dir.join(PENDING)), Err(error) if error.kind() == io::ErrorKind::NotFound)
    );
}

#[test]
fn sqlite_abort_and_duplicate_never_send_or_replace_approval() {
    let scratch = Scratch::new();
    let database = scratch.0.join(DATABASE);
    let mut sales = open_sales(&database);
    let mut authority =
        ConfirmationFixture::new(ProductId::new(1), 5, Money::from_minor_units(42)).unwrap();
    authority.register_draft(draft(45, 2)).unwrap();
    let approved = authority.review(Role::Owner, DraftId::new(45)).unwrap();
    let id = RequestId::new(456);
    let output = Command::new("sqlite3").arg(&database).arg("CREATE TRIGGER reject_pending BEFORE INSERT ON pending_owner_confirmations BEGIN SELECT RAISE(ABORT, 'rejected'); END;").output().unwrap();
    assert!(output.status.success());
    let mut client = Client::start("sqlite_fail", &scratch.0);
    client.write_frame(
        &OwnerRequest {
            id,
            review: approved.clone(),
        }
        .encode(),
    );
    assert_eq!(client.finish(), ["SAVE_FAILED"]);
    assert_no_sidecar(&scratch.0);
    let output = Command::new("sqlite3")
        .arg(&database)
        .arg("DROP TRIGGER reject_pending;")
        .output()
        .unwrap();
    assert!(output.status.success());
    sales
        .session(Role::Owner)
        .confirm_pending(id, &approved, |_, _| {})
        .unwrap();
    authority
        .set_unit_price(Money::from_minor_units(99))
        .unwrap();
    let changed = authority.review(Role::Owner, DraftId::new(45)).unwrap();
    let mut sent = false;
    assert!(
        sales
            .session(Role::Owner)
            .confirm_pending(id, &changed, |_, _| sent = true)
            .is_err()
    );
    assert!(!sent);
    sales
        .session(Role::Owner)
        .retry_pending(id, |stored_id, stored| {
            assert_eq!((stored_id, stored), (id, &approved))
        })
        .unwrap();
}

pub(super) fn worker(
    mode: &str,
    database: &Path,
    input: &mut impl Iterator<Item = io::Result<String>>,
) {
    let mut sales = open_sales(database);
    let dir = database.parent().unwrap();
    assert_no_sidecar(dir);
    match mode {
        "sqlite_submit" | "sqlite_fail" => {
            let approval = OwnerRequest::decode(&input.next().unwrap().unwrap());
            let result = sales.session(Role::Owner).confirm_pending(
                approval.id,
                &approval.review,
                |id, review| {
                    assert_no_sidecar(dir);
                    send(&OwnerRequest {
                        id,
                        review: review.clone(),
                    });
                },
            );
            if mode == "sqlite_fail" {
                assert!(result.is_err());
                emit("SAVE_FAILED");
                return;
            }
            result.unwrap();
            assert_eq!(input.next().unwrap().unwrap(), "CONTINUE");
            sales.session(Role::Owner).save_draft(draft(43, 1)).unwrap();
            assert!(input.next().is_none());
            emit("REPLY_EOF");
        }
        "sqlite_reopen" => {
            assert_eq!(
                sales
                    .session(Role::Owner)
                    .reopen_draft(DraftId::new(43))
                    .unwrap(),
                Some(draft(43, 1))
            );
            emit("REOPENED");
        }
        "sqlite_manager" => {
            assert!(matches!(
                sales
                    .session(Role::Manager)
                    .retry_pending(RequestId::new(987), |_, _| emit("UNEXPECTED_SEND")),
                Err(crate::confirmation::PendingError::AccessDenied)
            ));
            emit("DENIED");
        }
        "sqlite_retry" => {
            sales
                .session(Role::Owner)
                .retry_pending(RequestId::new(987), |id, review| {
                    assert_no_sidecar(dir);
                    send(&OwnerRequest {
                        id,
                        review: review.clone(),
                    })
                })
                .unwrap();
            let reply = input.next().unwrap().unwrap();
            let receipt = read_receipt(&reply).unwrap();
            emit(&format!("RECEIVED {}", receipt_frame(&receipt)));
        }
        _ => panic!("unknown SQLite pending worker mode"),
    }
    assert_no_sidecar(dir);
}
