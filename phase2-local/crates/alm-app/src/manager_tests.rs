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

#[test]
fn a_manager_sees_balances_adds_customers_records_payments_and_closes_today_only() {
    let (_root, mut trial) = manager("manager-money");
    // The device's day is the authority's (the Cash Book closes the shop's today).
    let today = alm_core::projection::shop_today(
        u64::try_from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis(),
        )
        .unwrap(),
    )
    .unwrap();
    trial.today = Some(today);
    saved_bill(&mut trial, 1);
    trial.bill_post("20").unwrap();

    // Balances, mobiles and addresses, without the Owner's ledger or opening balance.
    let page = trial.render("customers").unwrap();
    assert!(!page.contains("role=\"alert\""), "{page}");
    assert!(
        page.contains("90000 00001") && page.contains("12 Synthetic Lane"),
        "{page}"
    );
    assert!(page.contains("₹102"), "the balance due: {page}");
    assert!(!page.contains("customer_open") && !page.contains("customer_opening"));
    assert!(
        trial
            .send(&Intent::SaveOpening {
                customer: "batch1-customer".into(),
                amount: Some(Money::from_minor_units(100)),
                date: None,
            })
            .is_err()
    );

    // Add a customer, then change its contact.
    trial.customer_edit("new").unwrap();
    trial
        .customer_save(crate::customers::Contact {
            id: String::new(),
            name: " Synthetic Corner ".into(),
            mobile: "90000 00009".into(),
            addr1: "3 Synthetic Road".into(),
            addr2: String::new(),
        })
        .unwrap();
    let twin = trial.customer_save(crate::customers::Contact {
        id: String::new(),
        name: "synthetic corner".into(),
        mobile: String::new(),
        addr1: String::new(),
        addr2: String::new(),
    });
    assert!(twin.unwrap_err().contains("already exists"));
    trial
        .customer_save(crate::customers::Contact {
            id: "cust-1".into(),
            name: "Synthetic Corner".into(),
            mobile: "90000 00010".into(),
            addr1: "4 Synthetic Road".into(),
            addr2: "Kanpur".into(),
        })
        .unwrap();
    let page = trial.render("customers").unwrap();
    assert!(
        page.contains("Synthetic Corner") && page.contains("90000 00010"),
        "{page}"
    );
    assert!(
        trial
            .bill_customer_search("corner")
            .contains("Synthetic Corner")
    );

    // A payment against the bill.
    let page = trial.render("receipt").unwrap();
    assert!(page.contains(r#"data-action="receipt_save""#), "{page}");
    trial
        .receipt_save(crate::receipts::Terms {
            customer: "batch1-customer".into(),
            date: today.to_string(),
            amount: "50".into(),
            mode: crate::receipts::Mode::Online,
        })
        .unwrap();
    assert!(trial.render("customers").unwrap().contains("₹52"));

    // Today's Cash Book: closed once, never an earlier day, a closed day or the month.
    let page = trial.render("expenses").unwrap();
    assert!(
        !page.contains("(month)") && page.contains("not closed yet"),
        "{page}"
    );
    let earlier = crate::screen_tests::entry("", "2025-04-30", "10", "save");
    assert!(trial.cash_entry(earlier).is_err());
    trial
        .cash_entry(crate::screen_tests::entry(
            "",
            &today.to_string(),
            "70",
            "save",
        ))
        .unwrap();
    let page = trial.render("expenses").unwrap();
    assert!(page.contains("closed") && page.contains("₹70"), "{page}");
    assert!(!page.contains("cash_edit"));
    assert!(
        trial
            .cash_entry(crate::screen_tests::entry(
                "",
                &today.to_string(),
                "80",
                "save"
            ))
            .is_err(),
        "today closes once"
    );
    assert!(
        trial
            .send(&Intent::SaveCashDay {
                id: Some(0),
                date: today,
                amounts: alm_core::confirmation::manual_cashbook::CashBookAmounts::ZERO,
            })
            .is_err(),
        "a closed day is the Owner's"
    );
}

/// The Owner's purchasing in the simulation: a PO for Synthetic Soda at a buy rate.
fn owner_purchase(trial: &mut Trial) {
    trial.set_owner(true).unwrap();
    trial
        .send(&Intent::SavePurchaseOrder(alm_sync::PurchaseOrderTerms {
            supplier: Some((None, "Synthetic Depot".into())),
            date: Some(day()),
            note: String::new(),
            lines: vec![alm_sync::PurchaseLineTerms {
                item: "Synthetic Soda".into(),
                product: Some(2),
                ordered: 10,
                rate: Money::from_minor_units(3_610),
            }],
        }))
        .unwrap();
    trial.set_owner(false).unwrap();
}

#[test]
fn a_manager_receives_counts_and_lists_stock_without_costs() {
    let (_root, mut trial) = manager("manager-stock");
    owner_purchase(&mut trial);
    // Receiving: quantities only, no rate column or rate sent; the order's rate applies.
    trial.render("purchases").unwrap();
    trial.receiving = Some(trial.purchase_id(0).unwrap());
    let page = trial.render("purchases").unwrap();
    assert!(
        page.contains("Receive now") && !page.contains("Rate ₹"),
        "{page}"
    );
    assert!(
        !page.contains("po_close") && !page.contains("poprint"),
        "{page}"
    );
    let mut terms = std::collections::HashMap::new();
    terms.insert("po".to_owned(), "0".to_owned());
    terms.insert("date".to_owned(), "2025-04-30".to_owned());
    terms.insert("qty_0".to_owned(), "4".to_owned());
    terms.insert("rate_0".to_owned(), "1".to_owned());
    trial.po_receive_save(terms).unwrap();
    assert!(
        trial
            .outbox
            .entries()
            .unwrap()
            .iter()
            .any(|e| e.intent.starts_with("receivegoods po=0 ") && e.intent.contains("lines=4")),
        "quantities only"
    );
    let soda = |trial: &mut Trial| {
        let stored = trial.projected().unwrap();
        trial
            .screen_rows(&stored)
            .stock
            .iter()
            .find(|row| row.product == 2)
            .unwrap()
            .on_hand
    };
    assert_eq!(soda(&mut trial), 44);
    trial.set_owner(true).unwrap();
    let lots = trial
        .authority
        .open_stock_lots(Role::Owner, alm_core::ProductId::new(2))
        .unwrap();
    assert!(
        lots.rows
            .iter()
            .any(|lot| lot.unit_cost == Some(Money::from_minor_units(3_610))),
        "the order's rate: {lots:?}"
    );
    trial.set_owner(false).unwrap();
    // A buy rate sent by a Manager is refused by the authority.
    assert!(
        trial
            .send(&Intent::ReceivePurchaseOrder {
                po: 0,
                date: None,
                today: day(),
                lines: vec![(1, Money::from_minor_units(1))],
                remaining: vec![6],
            })
            .is_err()
    );

    // Counting stock, the godown list and a crate template.
    trial
        .stock_count(&crate::stock::Count {
            product: "2".into(),
            on_hand: "44".into(),
            counted: "43".into(),
            fridge: String::new(),
            reason: "damage".into(),
            note: String::new(),
        })
        .unwrap();
    assert_eq!(soda(&mut trial), 43);
    crate::godown::save(
        &mut trial,
        &crate::godown::Row {
            item: "Synthetic Soda".into(),
            cases: "9".into(),
            note: "back".into(),
        },
    )
    .unwrap();
    assert!(trial.render("godown").unwrap().contains("9 cases"));
    trial
        .send(&Intent::SaveCrate(alm_sync::CrateTerms {
            id: None,
            name: "Soda Pair".into(),
            price: Money::from_minor_units(12_000),
            components: vec![(2, 2, false)],
        }))
        .unwrap();
    assert!(trial.render("crates").unwrap().contains("Soda Pair"));
    assert!(
        trial.send(&Intent::DeleteCrate { id: 1 }).is_err(),
        "Owner only"
    );
    for refused in [
        Intent::ClosePurchaseOrder {
            po: 0,
            remaining: vec![6],
        },
        Intent::SetSupplierPrices {
            product: 2,
            rates: vec![(0, Some(Money::from_minor_units(3_000)))],
        },
    ] {
        assert!(trial.send(&refused).is_err(), "{refused:?}");
    }
    // The Reorder and Rate List texts a Manager copies or shares: quantities and selling rates.
    for text in [
        trial.reorder_copy_text().unwrap(),
        trial.rate_copy_text().unwrap(),
    ] {
        for cost in ["70", "38", "36.1", "69", "72"] {
            assert!(!text.contains(&format!("₹{cost}")), "{text}");
        }
    }
}

/// DEC-034: every screen a Manager opens, over a shop with sales, quotes, a received PO, a
/// crate, a godown row and a count, shows no cost, buy rate, supplier price, FIFO lot, stock
/// value, margin or profit; the Owner's same screens do.
#[test]
fn no_manager_screen_shows_a_cost() {
    let (_root, mut trial) = manager("manager-no-cost");
    owner_purchase(&mut trial);
    trial.set_owner(true).unwrap();
    trial
        .send(&Intent::ReceivePurchaseOrder {
            po: 0,
            date: Some(day()),
            today: day(),
            lines: vec![(4, Money::from_minor_units(3_620))],
            remaining: vec![10],
        })
        .unwrap();
    trial
        .send(&Intent::SaveCrate(alm_sync::CrateTerms {
            id: None,
            name: "Party".into(),
            price: Money::from_minor_units(40_000),
            components: vec![(1, 2, false)],
        }))
        .unwrap();
    trial.set_owner(false).unwrap();
    saved_bill(&mut trial, 2);
    trial.bill_post("20").unwrap();
    trial.stock.adjust = Some(alm_core::ProductId::new(2));
    trial.receiving = Some(trial.purchase_id(0).unwrap());
    let mut owner_saw = String::new();
    for role in [Role::Owner, Role::Manager] {
        trial.set_owner(role == Role::Owner).unwrap();
        for tab in crate::screen_tests::MANAGER_SCREENS.iter().chain(&["bill"]) {
            let html = trial.render(tab).unwrap_or_else(|e| panic!("{tab}: {e}"));
            if role == Role::Owner {
                owner_saw += &html;
                continue;
            }
            assert!(!html.contains("role=\"alert\""), "{tab}: {html}");
            // Buy rates, quotes, PO and received rates, lot costs and the FIFO stock value.
            for cost in [
                "70.00", "38.00", "69.00", "72.00", "37.00", "36.00", "36.10", "36.20", "₹70",
                "₹38", "₹36",
            ] {
                assert!(!html.contains(cost), "{tab} shows {cost}");
            }
            for word in [
                "Profit",
                "profit is",
                "Stock Value",
                "Est. Cost",
                "Best Supplier",
                "Margin",
                "Buy",
                "buy rate",
                "FIFO",
                "Lots",
                "Synthetic Wholesale",
            ] {
                assert!(!html.contains(word), "{tab} shows {word}");
            }
        }
    }
    // The bill a Manager prints and shares: the estimate document and the image's text.
    trial.set_owner(false).unwrap();
    let bill = sales(&mut trial)[0].draft_id;
    let estimate = trial.estimate_html(bill).unwrap();
    let (_, image) = trial.bill_image(bill).unwrap();
    let texts: String = image
        .ops
        .iter()
        .filter_map(|op| match op {
            alm_core::confirmation::bill_image::DrawOp::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");
    for output in [estimate.as_str(), texts.as_str()] {
        for cost in [
            "70.00", "38.00", "36.10", "36.20", "₹70", "₹38", "Profit", "Cost",
        ] {
            assert!(!output.contains(cost), "a bill output shows {cost}");
        }
    }
    assert!(texts.contains("Synthetic Customer"));
    for shown in ["Stock Value", "Profit", "Rate ₹"] {
        assert!(
            owner_saw.contains(shown),
            "the Owner's screens show {shown}"
        );
    }
}
