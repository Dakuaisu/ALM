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
        .find(|sale| sale.draft_id.value() == draft)
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
        trial.authority.submit(
            Role::Owner,
            &request,
            &Intent::CancelSale {
                draft: alm_core::DraftId::new(1)
            }
        ),
        Err(alm_sync::AuthorityError::Conflict)
    ));

    // Restart mid-send: an intent stored but never answered, and a second saved draft.
    let pending = trial.outbox.next_request_id("trial").unwrap();
    trial
        .outbox
        .record(
            &pending,
            &Intent::CancelSale {
                draft: alm_core::DraftId::new(1),
            }
            .canonical(),
        )
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

#[test]
fn an_unknown_outcome_is_never_sent_under_a_new_request() {
    let root = Root::new("device-unknown");
    let mut trial = Trial::open(&root.0, Some(day())).unwrap();
    saved_draft(&mut trial, 1);
    let serial = review(&mut trial);
    trial.authority.lose_next_response();
    assert!(trial.sale_confirm(serial).is_err());
    // Confirming the same reviewed bill again is refused before any new request is minted.
    let again = trial.sale_confirm(serial).unwrap_err();
    assert!(
        again.contains("check status / retry with the same request"),
        "{again}"
    );
    let entries = trial.outbox.entries().unwrap();
    assert_eq!(entries.len(), 1, "no second request ID");
    let page = trial.render("review").unwrap();
    assert!(page.contains("Outcome unknown: check status / retry with the same request"));
    assert!(page.contains(&format!(r#"data-q="{}""#, entries[0].request_id)));
    // The status lookup resolves it from the stored intent; nothing is re-sent.
    trial.reconnect().unwrap();
    assert_eq!(
        trial.outbox.entries().unwrap()[0].status,
        OutboxStatus::Confirmed
    );
    assert_eq!(trial.authority.posted_count(), 1);
    assert!(!trial.render("review").unwrap().contains("Outcome unknown"));
}

/// Every file under `dir`, read whole.
fn files(dir: &std::path::Path) -> Vec<(std::path::PathBuf, Vec<u8>)> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for entry in entries {
        let path = entry.unwrap().path();
        if path.is_dir() {
            out.extend(files(&path));
        } else {
            out.push((path.clone(), std::fs::read(&path).unwrap()));
        }
    }
    out
}

/// Device files holding `needle`; the simulated authority's journal stands in for the server.
fn holds(dir: &std::path::Path, needle: &str) -> Vec<std::path::PathBuf> {
    files(dir)
        .into_iter()
        .filter(|(path, _)| !path.to_string_lossy().contains("simulated-authority"))
        .filter(|(_, bytes)| bytes.windows(needle.len()).any(|w| w == needle.as_bytes()))
        .map(|(path, _)| path)
        .collect()
}

/// An Owner device with a posted bill, its outbox entry and an output file.
fn owner_with_history(name: &str) -> (Root, Trial) {
    let root = Root::new(name);
    let mut trial = Trial::open(&root.0, Some(day())).unwrap();
    saved_draft(&mut trial, 1);
    let serial = review(&mut trial);
    trial.sale_confirm(serial).unwrap();
    trial.render("sales").unwrap();
    trial
        .save_output("bill-1.png", b"synthetic output")
        .unwrap();
    assert!(!holds(&root.0.join("projections"), "Synthetic Customer").is_empty());
    assert!(!holds(&root.0, "sale draft=1").is_empty());
    (root, trial)
}

#[test]
fn a_downgrade_seen_on_pull_purges_owner_data_and_blocks_owner_commands() {
    let (root, mut trial) = owner_with_history("device-downgrade");
    trial.authority.set_granted(Some(Role::Manager));
    let page = trial.render("sales").unwrap();
    assert_eq!(trial.role(), Role::Manager);
    assert!(page.contains("no longer an Owner"), "{page}");
    assert!(
        trial
            .send(&Intent::CancelSale {
                draft: alm_core::DraftId::new(1)
            })
            .is_err()
    );
    let projections = root.0.join("projections");
    assert!(
        files(&projections)
            .iter()
            .all(|(path, _)| !path.to_string_lossy().contains("-owner.sqlite3")),
        "the Owner projection file is gone"
    );
    assert!(holds(&projections, "Synthetic Customer").is_empty());
    assert!(
        !holds(&projections, "Synthetic Lime").is_empty(),
        "the Manager catalog"
    );
    assert!(
        holds(&root.0, "sale draft=1").is_empty(),
        "queued intents gone"
    );
    assert!(!root.0.join("outputs").exists(), "output files gone");
    assert!(trial.outbox.entries().unwrap().is_empty());
    // Request numbering continues: a purged ID is never minted again.
    assert_eq!(trial.outbox.next_request_id("trial").unwrap(), "trial-2");
}

#[test]
fn a_revocation_seen_on_pull_purges_the_account_and_blocks_every_command() {
    let (root, mut trial) = owner_with_history("device-revoke");
    trial.authority.set_granted(None);
    let page = trial.render("sales").unwrap();
    assert!(page.contains("revoked"), "{page}");
    assert!(!page.contains('₹'), "{page}");
    assert!(trial.require_owner().is_err());
    assert!(
        trial
            .send(&Intent::CancelSale {
                draft: alm_core::DraftId::new(1)
            })
            .is_err()
    );
    assert!(
        trial
            .bill_add(
                alm_core::ProductId::new(1),
                crate::bill_entry::quantity("1").unwrap()
            )
            .is_err()
    );
    for needle in ["Synthetic Customer", "Synthetic Lime", "sale draft=1"] {
        assert!(
            holds(&root.0.join("projections"), needle).is_empty(),
            "{needle}"
        );
    }
    assert!(holds(&root.0, "sale draft=1").is_empty());
    assert!(!root.0.join("outputs").exists());
    // Choosing another account is still possible; the revocation applies on its next pull too.
    trial.authority.set_granted(Some(Role::Owner));
    trial.switch_account("trial-b").unwrap();
    assert!(!trial.revoked);
}

#[test]
fn the_owner_can_point_this_device_at_a_rest_authority_and_back() {
    let root = Root::new("device-rest");
    let mut trial = Trial::open(&root.0, Some(day())).unwrap();
    let form = |url: &str| crate::dev_authority::RestForm::new(url, "http://127.0.0.1:9", "");
    trial.set_owner(false).unwrap();
    assert!(
        trial.settings_rest(&form("http://127.0.0.1:9")).is_err(),
        "Owner only"
    );
    trial.set_owner(true).unwrap();
    assert!(trial.settings_rest(&form("ftp://x")).is_err());
    assert!(
        trial
            .settings_rest(&crate::dev_authority::RestForm::new(
                "http://127.0.0.1:9",
                "",
                ""
            ))
            .is_err(),
        "a sign-in URL is needed"
    );
    // Nothing listens on the discard port. The device never falls back to the simulation:
    // every screen is the sign-in screen, nothing is sent, and sign-in finds no service.
    saved_draft(&mut trial, 1);
    let serial = review(&mut trial);
    trial.settings_rest(&form("http://127.0.0.1:9")).unwrap();
    let saved = std::fs::read_to_string(root.0.join("rest-authority.txt")).unwrap();
    assert_eq!(
        saved, "http://127.0.0.1:9\nhttp://127.0.0.1:9\n\n",
        "no token is kept"
    );
    for tab in ["sales", "settings", "review"] {
        let page = trial.render(tab).unwrap();
        assert!(
            page.contains(r#"data-action="rest_login""#) && !page.contains("review_role"),
            "{page}"
        );
        assert!(page.contains(r#"<template id="screen-nav""#), "{page}");
    }
    assert_eq!(trial.sale_confirm(serial).unwrap_err(), "Sign in first.");
    assert_eq!(trial.authority.posted_count(), 0);
    assert_eq!(
        trial
            .rest_login(&crate::dev_authority::LoginForm::new("x@example.test", "x"))
            .unwrap_err(),
        "No answer from the sign-in service."
    );
    // A restart keeps the device-local choice and asks for a sign-in again.
    drop(trial);
    let mut trial = Trial::open(&root.0, Some(day())).unwrap();
    assert!(trial.signed_out());
    assert!(trial.render("bill").unwrap().contains("Sign in"));
    trial.settings_rest_off().unwrap();
    assert!(trial.rest.is_none() && !root.0.join("rest-authority.txt").exists());
    assert!(
        trial
            .render("settings")
            .unwrap()
            .contains("built-in authority simulation")
    );
}

#[test]
fn request_ids_name_the_account_and_this_device_install_and_old_ids_stay_resolvable() {
    let root = Root::new("device-request-ids");
    let mut trial = Trial::open(&root.0, Some(day())).unwrap();
    let device = trial.device_id.clone();
    let hex = device.strip_prefix("dev-").unwrap();
    assert!(hex.len() == 16 && hex.bytes().all(|b| b.is_ascii_hexdigit()));
    let other = Root::new("device-request-ids-other");
    let second = Trial::open(&other.0, Some(day())).unwrap();
    assert_ne!(second.device_id, device, "each install draws its own ID");

    saved_draft(&mut trial, 1);
    let serial = review(&mut trial);
    assert_eq!(trial.sale_confirm(serial), Ok(1));
    let posted = trial.outbox.entries().unwrap().remove(0).request_id;
    assert_eq!(posted, format!("trial:{device}-1"));

    // An intent stored under the earlier `account-n` form before an upgrade.
    trial
        .outbox
        .record(
            "trial-2",
            &Intent::CancelSale {
                draft: alm_core::DraftId::new(1),
            }
            .canonical(),
        )
        .unwrap();
    drop(trial);
    let mut trial = Trial::open(&root.0, Some(day())).unwrap();
    assert_eq!(trial.device_id, device, "kept across a restart");
    assert_eq!(trial.retry("trial-2"), Ok(Outcome::Done));
    assert_eq!(cancelled(&mut trial, 1), Some(true));
    assert_eq!(
        trial.send(&Intent::RestoreSale {
            draft: alm_core::DraftId::new(1)
        }),
        Ok(Outcome::Done)
    );
    let ids: Vec<_> = trial
        .outbox
        .entries()
        .unwrap()
        .into_iter()
        .map(|e| e.request_id)
        .collect();
    assert_eq!(ids, [posted, "trial-2".into(), format!("trial:{device}-3")]);
}

#[test]
fn a_device_id_from_an_earlier_build_is_kept() {
    let root = Root::new("device-id-earlier");
    std::fs::write(root.0.join("device-id.txt"), "dev-0a1b2c3d\n").unwrap();
    let trial = Trial::open(&root.0, Some(day())).unwrap();
    assert_eq!(trial.device_id, "dev-0a1b2c3d");
}

#[test]
fn request_prefixes_fit_the_request_id_bound_for_any_account() {
    let long = "a".repeat(64);
    let prefix = alm_sync::request_prefix(&long, "dev-0123456789abcdef");
    let id = format!("{prefix}-{}", u64::MAX);
    assert!(RequestId::new(&id).is_ok(), "{id}");
    assert!(prefix.ends_with(":dev-0123456789abcdef"));
    assert_eq!(
        alm_sync::request_prefix("trial", "dev-1"),
        "trial:dev-1",
        "short accounts are kept whole"
    );
}

#[test]
fn a_manager_drafts_from_the_catalog_its_device_pulled() {
    let root = Root::new("manager-catalog");
    let mut trial = Trial::open(&root.0, Some(day())).unwrap();
    trial.set_owner(false).unwrap();
    let stored = trial.projection.load().unwrap().unwrap();
    assert!(stored.snapshot.owner.is_none());
    let mut snapshot = stored.snapshot.clone();
    let id = snapshot.catalog[0].id;
    // The authority's catalog as pulled (here a name and sale price the built-in directory
    // does not hold): New Bill searches, adds and prices from it.
    snapshot.catalog[0].name = "Projected Lime".into();
    snapshot.catalog[0].rate = alm_core::Money::from_minor_units(23_600);
    trial
        .projection
        .apply(&alm_core::projection::ProjectionBatch {
            role: Role::Manager,
            checkpoint: stored.checkpoint + 1,
            as_of_ms: stored.as_of_ms,
            snapshot: Some(snapshot),
            rows: None,
        })
        .unwrap();
    assert!(
        trial
            .bill_item_search("projected")
            .contains("Projected Lime")
    );
    assert!(
        !trial
            .bill_item_search("synthetic lime")
            .contains("Synthetic Lime")
    );
    trial.current = Some((
        alm_core::ProvisionalDraft::new(DraftId::new(1), Vec::new()).with_bill_date(Some(day())),
        false,
    ));
    trial
        .bill_add(
            alm_core::ProductId::new(id),
            std::num::NonZeroU32::new(2).unwrap(),
        )
        .unwrap();
    let body = trial.bill_live("1", "2").unwrap();
    assert!(
        body.contains("Projected Lime") && body.contains("₹472.00"),
        "{body}"
    );
    assert!(
        trial
            .bill_add(alm_core::ProductId::new(999), std::num::NonZeroU32::MIN)
            .is_err(),
        "a product outside the pulled catalog"
    );
}
