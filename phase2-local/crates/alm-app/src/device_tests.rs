//! The device data layer in the app: outbox recovery, projection persistence, account switch.
use alm_core::{DraftId, Role};
use alm_store::{OutboxError, OutboxStatus, ProjectionStore};
use alm_sync::{Intent, Outcome, RequestId, SalesAuthority};

use crate::Trial;
use crate::billing::Terms;
use crate::billing_tests::{day, draft};
use crate::screen_tests::Root;

fn saved_draft(trial: &mut Trial, id: u64) {
    let draft = alm_core::ProvisionalDraft::new(DraftId::new(id), draft().lines().to_vec())
        .with_customer(draft().customer().clone())
        .with_bill_date(Some(day()));
    trial.drafts.save_draft(draft.clone()).unwrap();
    trial.current = Some((draft, true));
}

fn review(trial: &mut Trial) -> u64 {
    trial
        .sale_prepare(&Terms {
            date: "2025-04-30".into(),
            paid: "20".into(),
        })
        .unwrap();
    trial.posting.pending(Role::Owner).unwrap().serial()
}

fn cancelled(trial: &mut Trial, draft: u64) -> Option<bool> {
    trial
        .projected()
        .unwrap()
        .snapshot
        .sales
        .iter()
        .find(|sale| sale.draft_id == draft)
        .map(|sale| sale.cancelled)
}

#[test]
fn outbox_resolves_lost_responses_and_restarts_without_ever_posting_by_itself() {
    let root = Root::new("device-outbox");
    let mut trial = Trial::open(&root.0, Some(day())).unwrap();
    saved_draft(&mut trial, 1);
    let serial = review(&mut trial);

    // Lost response: the authority commits, the device records "unknown".
    trial.authority.lose_next_response();
    let error = trial.sale_confirm(serial).unwrap_err();
    assert!(error.contains("outcome is unknown"), "{error}");
    let lost = trial.outbox.entries().unwrap().remove(0);
    assert_eq!(lost.status, OutboxStatus::Unknown);
    assert!(trial.render("review").unwrap().contains("Outcome unknown"));
    // Conflicting reuse of the stored request ID is refused before anything is sent.
    assert!(matches!(
        trial.outbox.record(&lost.request_id, "cancel draft=1"),
        Err(OutboxError::Conflict)
    ));
    let intent = Intent::parse(&lost.intent).unwrap();
    let request = RequestId::new(&lost.request_id).unwrap();
    assert!(matches!(
        trial
            .authority
            .submit(Role::Owner, &request, &Intent::CancelSale { draft: 1 }),
        Err(alm_sync::AuthorityError::Conflict)
    ));

    // Restart mid-send: an intent stored but never answered, and a second saved draft.
    let pending = trial.outbox.next_request_id("trial").unwrap();
    trial
        .outbox
        .record(&pending, &Intent::CancelSale { draft: 1 }.canonical())
        .unwrap();
    saved_draft(&mut trial, 2);
    drop(trial);

    let mut trial = Trial::open(&root.0, Some(day())).unwrap();
    let entries = trial.outbox.entries().unwrap();
    assert_eq!(
        entries.iter().map(|e| e.status).collect::<Vec<_>>(),
        [OutboxStatus::Confirmed, OutboxStatus::Unknown],
        "status lookup resolved the lost response; the unsent cancel stays unknown"
    );
    assert_eq!(
        cancelled(&mut trial, 1),
        Some(false),
        "nothing re-sent on reconnect"
    );
    assert!(
        trial
            .authority
            .current_sale(Role::Owner, DraftId::new(2))
            .is_err(),
        "a saved draft is never posted automatically"
    );
    let Some(Outcome::Sale(original)) = entries[0].outcome.as_deref().and_then(Outcome::parse)
    else {
        panic!("confirmed sale outcome")
    };
    let checkpoint = trial.authority.checkpoint();
    assert_eq!(
        trial.authority.submit(Role::Owner, &request, &intent),
        Ok(Outcome::Sale(original)),
        "an identical retry returns the original receipt"
    );
    assert_eq!(trial.authority.checkpoint(), checkpoint);

    // The explicit retry sends the stored intent under its original request ID.
    assert_eq!(trial.retry(&pending), Ok(Outcome::Done));
    assert_eq!(cancelled(&mut trial, 1), Some(true));
    assert!(
        trial.retry(&pending).is_err(),
        "resolved intents are not retried"
    );
    assert!(trial.outbox.unresolved().unwrap().is_empty());
}

#[test]
fn posted_data_is_read_from_the_projection_and_survives_a_restart() {
    let root = Root::new("device-projection");
    let mut trial = Trial::open(&root.0, Some(day())).unwrap();
    saved_draft(&mut trial, 1);
    let serial = review(&mut trial);
    assert_eq!(trial.sale_confirm(serial), Ok(1));
    let sales = trial.render("sales").unwrap();
    assert!(sales.contains("<b>#1</b>") && sales.contains("Posted data as of "));
    drop(trial);

    // The device file alone holds the posted sale, as of its checkpoint.
    let stored = ProjectionStore::open_scoped(&root.0, "trial", Role::Owner)
        .unwrap()
        .load()
        .unwrap()
        .unwrap();
    assert_eq!(stored.snapshot.sales[0].invoice, 1);
    assert!(stored.checkpoint >= 1 && stored.as_of_ms > 0);

    // A lost authority history (reset journal) rebuilds the projection from scratch.
    std::fs::remove_file(root.0.join("simulated-authority/journal.txt")).unwrap();
    let mut trial = Trial::open(&root.0, Some(day())).unwrap();
    assert!(trial.projected().unwrap().snapshot.sales.is_empty());
    let customers = trial.render("customers").unwrap();
    assert!(customers.contains("data-freshness"));
}

#[test]
fn switching_accounts_drops_the_previous_accounts_projection_outbox_and_drafts() {
    let root = Root::new("device-switch");
    let mut trial = Trial::open(&root.0, Some(day())).unwrap();
    saved_draft(&mut trial, 1);
    let serial = review(&mut trial);
    trial.authority.lose_next_response();
    assert!(trial.sale_confirm(serial).is_err());
    let before = std::fs::read_dir(root.0.join("projections"))
        .unwrap()
        .count();
    let first = |root: &std::path::Path| {
        ProjectionStore::open_scoped(root, "trial", Role::Owner)
            .unwrap()
            .load()
            .unwrap()
    };

    trial.switch_account("trial-b").unwrap();
    assert_eq!(trial.account, "trial-b");
    assert!(trial.current.is_none() && trial.reviewed.is_none());
    assert!(trial.posting.pending(Role::Owner).is_none());
    assert!(trial.outbox.entries().unwrap().is_empty());
    assert!(
        trial
            .drafts
            .reopen_draft(DraftId::new(1))
            .unwrap()
            .is_none()
    );
    let page = trial.render("review").unwrap();
    assert!(
        !page.contains("Outcome unknown"),
        "the other account's outbox is not shown"
    );
    assert!(page.contains("account trial-b"));
    assert!(
        std::fs::read_dir(root.0.join("projections"))
            .unwrap()
            .count()
            > before,
        "trial-b has its own projection file"
    );
    let frozen = first(&root.0);
    saved_draft(&mut trial, 7);
    let serial = review(&mut trial);
    assert!(trial.sale_confirm(serial).is_ok());
    trial.render("sales").unwrap();
    assert_eq!(
        first(&root.0),
        frozen,
        "the previous account's projection is no longer used or updated"
    );

    trial.switch_account("trial").unwrap();
    let entries = trial.outbox.entries().unwrap();
    assert_eq!(entries.len(), 1, "trial's own outbox is back");
    assert_eq!(
        entries[0].status,
        OutboxStatus::Confirmed,
        "reconnect resolved the lost response by status lookup"
    );
    assert!(trial.switch_account("not valid!").is_err());
    assert_eq!(trial.account, "trial");
}
