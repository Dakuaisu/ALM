//! DEC-034 on the device: a Manager drafts from its cost-free book, posts when online through
//! the same idempotent path, gets stale terms back for review, adds crate rows, edits and
//! cancels posted bills, and never posts a draft by reconnecting.
use alm_core::{Money, Role};
use alm_sync::{Intent, ProductTerms};

use crate::Trial;
use crate::billing_tests::day;
use crate::screen_tests::Root;

fn manager(name: &str) -> (Root, Trial) {
    let root = Root::new(name);
    let mut trial = Trial::open(&root.0, Some(day())).unwrap();
    trial
        .send(&Intent::SetLoadingDefault {
            rate: Money::from_minor_units(150),
        })
        .unwrap();
    trial.set_owner(false).unwrap();
    assert!(trial.manager_book(), "the Manager pulled its book");
    (root, trial)
}

/// Customer, date, one line of product 1 and save, as New Bill sends them.
fn saved_bill(trial: &mut Trial, quantity: u32) {
    trial.render("bill").unwrap();
    trial
        .bill_add(
            alm_core::ProductId::new(1),
            std::num::NonZeroU32::new(quantity).unwrap(),
        )
        .unwrap();
    trial
        .bill_header(crate::bill_entry::Header::new(
            "batch1-customer",
            "2025-04-30",
            None,
            crate::bill_entry::HeaderIntent::Save,
        ))
        .unwrap();
}

fn sales(trial: &mut Trial) -> Vec<alm_core::projection::ProjectedSale> {
    let stored = trial.projected().unwrap();
    trial.screen_rows(&stored).sales.clone()
}

#[test]
fn a_manager_posts_its_saved_bill_at_the_shop_loading_and_the_authority_rechecks_it() {
    let (_root, mut trial) = manager("manager-post");
    let page = trial.render("bill").unwrap();
    assert!(!page.contains("Post bill"), "an unsaved bill is not posted");
    saved_bill(&mut trial, 2);
    let (draft, _) = trial.current.clone().unwrap();
    assert_eq!(draft.loading_rate(), Money::from_minor_units(150));
    let page = trial.render("bill").unwrap();
    assert!(page.contains(r#"data-action="bill_post""#), "{page}");
    let invoice = trial.bill_post("20").unwrap();
    assert_eq!(invoice, 1);
    let posted = sales(&mut trial);
    assert_eq!(posted.len(), 1);
    // Two packets at ₹120.50 and ₹1.50 loading each.
    assert_eq!(posted[0].total, Money::from_minor_units(24_400));
    let page = trial.render("sales").unwrap();
    assert!(page.contains("#1"), "{page}");
    assert!(!page.contains("Est. Profit"), "{page}");

    // The Owner changes the selling rate; the Manager's bill comes back for review.
    trial.set_owner(true).unwrap();
    let pricing = trial
        .posted
        .open_product_entry(Role::Owner, alm_core::ProductId::new(1))
        .unwrap()
        .pricing();
    trial
        .send(&Intent::SaveProduct(ProductTerms {
            id: 1,
            new: false,
            item: "Synthetic Lime".into(),
            brand: String::new(),
            buy: pricing.buy(),
            sell: Money::from_minor_units(13_000),
            reorder: None,
            reviewed: Some((pricing.buy(), pricing.sell())),
        }))
        .unwrap();
    trial.set_owner(false).unwrap();
    saved_bill(&mut trial, 1);
    let stale_draft = trial.current.clone().unwrap().0;
    let stale = alm_sync::SaleIntent {
        date: day(),
        paid: Money::from_minor_units(0),
        draft: stale_draft,
        prices: vec![Money::from_minor_units(12_050)],
    };
    assert!(
        trial
            .send(&Intent::PostSale(stale))
            .unwrap_err()
            .contains("review again")
    );
    assert_eq!(sales(&mut trial).len(), 1, "nothing posted");
    // Re-priced from the refreshed book, it posts at the new rate.
    assert_eq!(trial.bill_post("0").unwrap(), 2);
    assert_eq!(sales(&mut trial)[0].total, Money::from_minor_units(13_150));
}

#[test]
fn a_managers_offline_draft_is_never_posted_by_reconnecting() {
    let (_root, mut trial) = manager("manager-offline");
    saved_bill(&mut trial, 1);
    trial.authority.lose_next_response();
    let error = trial.bill_post("0").unwrap_err();
    assert!(error.contains("outcome is unknown"), "{error}");
    let entries = trial.outbox.entries().unwrap();
    let request = entries.last().unwrap().request_id.clone();
    // A second saved draft waits; reconnecting resolves the unknown one and posts nothing new.
    saved_bill(&mut trial, 3);
    trial.reconnect().unwrap();
    assert_eq!(sales(&mut trial).len(), 1, "only the sent bill committed");
    assert_eq!(
        trial.outbox.get(&request).unwrap().unwrap().status,
        alm_store::OutboxStatus::Confirmed
    );
    assert_eq!(trial.outbox.entries().unwrap().len(), entries.len());
}

#[test]
fn a_manager_adds_a_crate_row_and_edits_and_cancels_posted_bills() {
    let (_root, mut trial) = manager("manager-edit");
    trial.set_owner(true).unwrap();
    trial
        .send(&Intent::SaveCrate(alm_sync::CrateTerms {
            id: None,
            name: "Party".into(),
            price: Money::from_minor_units(40_000),
            components: vec![(1, 2, false)],
        }))
        .unwrap();
    trial.set_owner(false).unwrap();
    saved_bill(&mut trial, 1);
    let crates = trial.bill_crates();
    assert_eq!(crates.len(), 1, "a Manager sees the saved crates");
    trial
        .bill_crate_add(crates[0].0, std::num::NonZeroU32::MIN)
        .unwrap();
    trial
        .bill_header(crate::bill_entry::Header::new(
            "batch1-customer",
            "2025-04-30",
            None,
            crate::bill_entry::HeaderIntent::Save,
        ))
        .unwrap();
    let invoice = trial.bill_post("50").unwrap();
    let bill = sales(&mut trial)
        .into_iter()
        .find(|s| s.invoice == invoice)
        .unwrap();
    assert_eq!(bill.items.len(), 2, "the item and the crate");

    // Edit: three packets of the item, the crate unchanged.
    let mut terms = std::collections::HashMap::new();
    terms.insert("id".to_owned(), bill.draft_id.to_string());
    terms.insert("qty_0".to_owned(), "3".to_owned());
    terms.insert("rate_0".to_owned(), "120.50".to_owned());
    terms.insert("qty_1".to_owned(), "1".to_owned());
    terms.insert("rate_1".to_owned(), "400".to_owned());
    trial.sale_edit_save(terms).unwrap();
    let edited = sales(&mut trial)
        .into_iter()
        .find(|s| s.invoice == invoice)
        .unwrap();
    assert!(
        edited.total.minor_units() > bill.total.minor_units(),
        "{edited:?}"
    );

    trial
        .send(&Intent::CancelSale {
            draft: bill.draft_id,
        })
        .unwrap();
    assert!(
        sales(&mut trial)
            .iter()
            .find(|s| s.invoice == invoice)
            .unwrap()
            .cancelled
    );
    // Restoring stays the Owner's.
    assert!(
        trial
            .send(&Intent::RestoreSale {
                draft: bill.draft_id,
            })
            .is_err()
    );
    // The Owner's activity log names who edited and cancelled.
    trial.set_owner(true).unwrap();
    let activity = trial
        .projected()
        .unwrap()
        .snapshot
        .owner
        .unwrap()
        .book
        .activity;
    for kind in ["bill-save", "bill-edit", "bill-cancel"] {
        assert!(
            activity
                .iter()
                .any(|row| row.kind == kind && row.account == trial.account),
            "{kind}: {activity:?}"
        );
    }
    assert_eq!(Role::Owner, trial.role());
}
