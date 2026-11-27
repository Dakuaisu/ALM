//! Every remaining write goes through the outbox and the authority and survives an app restart;
//! the device GST tables move into the authority once, without losing a record.
use std::collections::HashMap;
use std::num::NonZeroU32;

use alm_core::confirmation::{GstBillDraft, GstBillId, GstBillLineInput, GstBillStore as _};
use alm_core::{BillDate, Money, ProductId, Role};
use alm_store::{GstBillDatabase, OutboxStatus};
use alm_sync::{Intent, SalesAuthority as _};

use crate::Trial;
use crate::billing_tests::day;
use crate::screen_tests::{Root, entry};

fn map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

fn product_form(item: &str, buy: &str, sell: &str) -> crate::products::Form {
    crate::products::Form {
        item: item.into(),
        brand: "Synthetic Co".into(),
        buy: buy.into(),
        margin: String::new(),
        sell: sell.into(),
        reorder: "6".into(),
        changed: String::new(),
    }
}

fn add_product(trial: &mut Trial, item: &str, buy: &str, sell: &str) -> ProductId {
    let id = trial.posted.next_product_id(Role::Owner).unwrap();
    trial.products.form = Some(trial.posted.begin_product_entry(Role::Owner, id).unwrap());
    trial.product_save(&product_form(item, buy, sell)).unwrap();
    id
}

/// Every write of the Owner screens, through the app's own handlers.
fn write_everything(trial: &mut Trial) -> ProductId {
    let orange = add_product(trial, "Synthetic Orange", "30", "45");
    trial.products.form = Some(
        trial
            .posted
            .open_product_entry(Role::Owner, orange)
            .unwrap(),
    );
    trial
        .product_save(&product_form("Synthetic Orange", "30", "47"))
        .unwrap();
    let gone = add_product(trial, "Synthetic Grape", "20", "30");
    trial.products.form = Some(trial.posted.open_product_entry(Role::Owner, gone).unwrap());
    assert!(trial.product_delete("ask").unwrap().is_some());
    assert!(trial.product_delete("confirm").unwrap().is_none());

    trial
        .send(&Intent::AddSupplier {
            name: "Synthetic Traders".into(),
        })
        .unwrap();
    let traders = trial
        .posted
        .purchase_suppliers(Role::Owner)
        .unwrap()
        .iter()
        .find(|s| s.name == "Synthetic Traders")
        .unwrap()
        .id
        .index();
    let product = orange.value().to_string();
    trial
        .supplier_quotes(map(&[
            ("product", &product),
            (&format!("s{traders}"), "29.50"),
        ]))
        .unwrap();

    trial
        .order_add(&crate::order::Line {
            product: product.clone(),
            qty: "10".into(),
        })
        .unwrap();
    trial.order_checkout().unwrap();
    trial
        .order_po(&crate::order::Po {
            supplier: "Synthetic Traders".into(),
            date: "2025-04-30".into(),
            note: "restart".into(),
            paste: String::new(),
            intent: "save".into(),
        })
        .unwrap();
    trial
        .po_receive_save(map(&[
            ("po", "0"),
            ("date", "2025-04-30"),
            ("qty_0", "4"),
            ("rate_0", "29.50"),
        ]))
        .unwrap();
    let po = trial.posted.purchase_orders(Role::Owner).unwrap()[0].id;
    let remaining: Vec<u32> = trial.posted.purchase_orders(Role::Owner).unwrap()[0]
        .items
        .iter()
        .map(alm_core::confirmation::SavedPurchaseLine::remaining)
        .collect();
    assert_eq!(remaining, [6]);
    trial
        .send(&Intent::ClosePurchaseOrder {
            po: u64::try_from(po.index()).unwrap(),
            remaining,
        })
        .unwrap();

    trial.crate_form = Some(crate::views::CrateForm {
        rows: vec![(String::new(), "1".into(), "Case".into())],
        ..crate::views::CrateForm::default()
    });
    trial
        .crate_submit(map(&[
            ("name", "Restart Crate"),
            ("price", "400"),
            ("product_0", "1"),
            ("qty_0", "2"),
            ("unit_0", "Case"),
            ("intent", "save"),
        ]))
        .unwrap();

    crate::customers::save_opening(
        trial,
        crate::customers::Opening {
            customer: "batch1-market".into(),
            amount: "150".into(),
            date: "2025-04-01".into(),
        },
    )
    .unwrap();
    crate::godown::save(
        trial,
        &crate::godown::Row {
            item: "Synthetic Soda".into(),
            cases: "9".into(),
            note: "back room".into(),
        },
    )
    .unwrap();
    let on_hand = trial
        .posted
        .open_physical_count(Role::Owner, ProductId::new(2))
        .unwrap()
        .on_hand;
    trial
        .stock_count(&crate::stock::Count {
            product: "2".into(),
            on_hand: on_hand.to_string(),
            counted: (on_hand - 3).to_string(),
            fridge: "2".into(),
            reason: "damage".into(),
            note: "dropped".into(),
        })
        .unwrap();
    trial
        .cash_entry(entry("", "2025-04-30", "25", "save"))
        .unwrap();

    trial.gst_form = Some(
        trial
            .posted
            .new_gst_bill_fields(Role::Owner, day())
            .unwrap(),
    );
    trial
        .gst_submit(
            map(&[
                ("customer", "Synthetic GST Buyer B"),
                ("date", "2025-04-30"),
                ("item_0", "Synthetic Soda"),
                ("unit_0", "Case"),
                ("qty_0", "2"),
                ("rate_0", "62.50"),
                ("gst_0", "18"),
                ("pack_0", ""),
                ("intent", "save"),
            ]),
            50,
        )
        .unwrap();
    trial.gst_cancel_bill(GstBillId(2)).unwrap();
    orange
}

const SCREENS: [&str; 10] = [
    "products",
    "suppliers",
    "purchases",
    "crates",
    "customers",
    "godown",
    "stock",
    "expenses",
    "gstsales",
    "reports",
];

#[test]
fn every_write_goes_through_the_outbox_and_survives_a_restart() {
    let root = Root::new("restart-writes");
    let mut trial = Trial::open(&root.0, Some(day())).unwrap();
    let orange = write_everything(&mut trial);
    let entries = trial.outbox.entries().unwrap();
    assert_eq!(entries.len(), 16, "{entries:?}");
    assert!(
        entries.iter().all(|e| e.status == OutboxStatus::Confirmed),
        "{entries:?}"
    );
    let before = trial.authority.pull(Role::Owner, None).unwrap();
    trial.message.clear();
    trial.customers.open = None;
    let screens: Vec<String> = SCREENS
        .iter()
        .map(|tab| trial.render(tab).unwrap())
        .collect();
    drop(trial);

    let mut trial = Trial::open(&root.0, Some(day())).unwrap();
    let after = trial.authority.pull(Role::Owner, None).unwrap();
    assert_eq!(after.checkpoint, before.checkpoint, "journal replayed");
    assert_eq!(after.snapshot, before.snapshot);
    for (tab, html) in SCREENS.iter().zip(&screens) {
        assert_eq!(&trial.render(tab).unwrap(), html, "{tab}");
    }
    let snapshot = trial.projected().unwrap().snapshot;
    let product = snapshot
        .products
        .iter()
        .find(|p| p.id == orange.value())
        .unwrap();
    assert_eq!(product.sell, Money::from_minor_units(4_700));
    assert!(
        !snapshot
            .products
            .iter()
            .any(|p| p.item == "Synthetic Grape")
    );
    let owner = snapshot.owner.as_ref().unwrap();
    let order = &owner.purchase_orders[0];
    assert_eq!(order.status, "Completed");
    assert_eq!((order.lines[0].received, order.lines[0].remaining), (4, 0));
    assert!(
        owner
            .supplier_prices
            .contains(&(orange.value(), order.id, Money::from_minor_units(2_950)))
            || owner
                .supplier_prices
                .iter()
                .any(|(p, _, r)| *p == orange.value() && r.minor_units() == 2_950)
    );
    assert!(snapshot.crates.iter().any(|c| c.name == "Restart Crate"));
    assert!(
        snapshot
            .godown
            .iter()
            .any(|g| g.item == "Synthetic Soda" && g.cases == 9)
    );
    assert!(
        snapshot
            .customers
            .iter()
            .any(|c| c.id == "batch1-market" && c.balance.minor_units() == 15_000)
    );
    assert_eq!(snapshot.cash_days.len(), 1);
    assert!(
        snapshot
            .gst_bills
            .iter()
            .any(|b| b.number == 2 && b.cancelled)
    );
    let stock = trial
        .posted
        .open_physical_count(Role::Owner, ProductId::new(2))
        .unwrap();
    assert_eq!((stock.fridge, stock.movements.len()), (2, 1));
    // Nothing is re-sent on reopen: the outbox is unchanged.
    assert_eq!(trial.outbox.entries().unwrap().len(), 16);
    crate::device::assert_book_view(&mut trial);
}

#[test]
fn stale_counts_and_receipts_are_refused_without_effects() {
    let root = Root::new("restart-stale");
    let mut trial = Trial::open(&root.0, Some(day())).unwrap();
    let before = trial.authority.pull(Role::Owner, None).unwrap();
    let error = trial
        .stock_count(&crate::stock::Count {
            product: "2".into(),
            on_hand: "1".into(),
            counted: "0".into(),
            fridge: String::new(),
            reason: "count".into(),
            note: String::new(),
        })
        .unwrap_err();
    assert!(error.contains("review"), "{error}");
    let after = trial.authority.pull(Role::Owner, None).unwrap();
    assert_eq!(
        (after.checkpoint, after.snapshot),
        (before.checkpoint, before.snapshot)
    );
    // DEC-034: the godown list is a Manager's too; the reorder default stays the Owner's.
    trial.set_owner(false).unwrap();
    assert!(
        trial
            .send(&alm_sync::Intent::SetReorderDefault { level: 3 })
            .is_err()
    );
    assert!(
        trial
            .outbox
            .entries()
            .unwrap()
            .iter()
            .all(|e| e.status != OutboxStatus::Confirmed)
    );
}

fn device_bill(id: u64, number: u64, customer: &str) -> alm_core::confirmation::GstBill {
    const LINES: [GstBillLineInput<'static>; 1] = [GstBillLineInput {
        item: "Synthetic Water 1 L",
        unit: alm_core::confirmation::GstUnit::Piece,
        quantity: 1,
        inclusive_rate: Money::from_minor_units(10_500),
        gst_basis_points: 500,
        ..GstBillLineInput::BLANK
    }];
    let fixture = alm_core::confirmation::demo_fixture::fixture().unwrap();
    let draft = GstBillDraft {
        id: GstBillId(id),
        number: alm_core::confirmation::GstBillNumber(number),
        customer,
        date: Some(BillDate::new(2025, 4, 20).unwrap()),
        lines: &LINES,
        ..GstBillDraft::default()
    };
    fixture.gst_bill_record(Role::Owner, &draft).unwrap()
}

#[test]
fn device_gst_tables_move_into_the_authority_once_without_losing_any() {
    let root = Root::new("restart-gst-migration");
    {
        // An older install: bills and a deletion marker in this account's device tables.
        let mut db = GstBillDatabase::open_scoped(&root.0, "trial").unwrap();
        db.put_gst_bill(&device_bill(5, 7, "Old Device Buyer"))
            .unwrap();
        db.put_gst_bill(&device_bill(6, 8, "Deleted Buyer"))
            .unwrap();
        db.delete_gst_bill(GstBillId(6)).unwrap();
        // Same ID as the authority's sample bill, different content: must not be dropped.
        db.put_gst_bill(&device_bill(1, 9, "Conflicting Buyer"))
            .unwrap();
    }
    let device_before = GstBillDatabase::open_scoped(&root.0, "trial")
        .unwrap()
        .load_gst_bills()
        .unwrap();
    let trial = Trial::open(&root.0, Some(day())).unwrap();
    let ledger = trial.posted.gst_bill_ledger(Role::Owner).unwrap();
    let moved = ledger.bills.iter().find(|b| b.id == GstBillId(5)).unwrap();
    assert_eq!(
        (moved.number.0, moved.customer.as_str()),
        (7, "Old Device Buyer")
    );
    assert_eq!(ledger.deleted, [GstBillId(6)]);
    let sample = ledger.bills.iter().find(|b| b.id == GstBillId(1)).unwrap();
    assert_eq!(sample.customer, "Synthetic GST Buyer");
    assert!(trial.message.contains("1 GST record"), "{}", trial.message);
    let statuses: Vec<_> = trial
        .outbox
        .entries()
        .unwrap()
        .into_iter()
        .map(|e| (e.request_id, e.status))
        .collect();
    assert_eq!(statuses.len(), 3, "{statuses:?}");
    drop(trial);
    // The device tables are the untouched migration source.
    assert_eq!(
        GstBillDatabase::open_scoped(&root.0, "trial")
            .unwrap()
            .load_gst_bills()
            .unwrap(),
        device_before
    );
    let mut trial = Trial::open(&root.0, Some(day())).unwrap();
    assert_eq!(trial.outbox.entries().unwrap().len(), 3, "nothing re-sent");
    assert_eq!(
        trial
            .authority
            .allocate_gst_number(Role::Owner, &alm_sync::gst_bill_key(10))
            .unwrap(),
        8,
        "the next number follows the moved bills"
    );
}

#[test]
fn new_bill_applies_the_default_loading_charge_through_review_posting_and_restart() {
    use std::num::NonZeroU32;

    use crate::bill_entry::{Header, HeaderIntent};
    let root = Root::new("restart-loading");
    let mut trial = Trial::open(&root.0, Some(day())).unwrap();
    let profile = crate::settings::Profile {
        name: "ALM".into(),
        loading: "1.50".into(),
        ..Default::default()
    };
    trial.settings_profile(&profile, 1).unwrap();
    trial.current = None;
    let html = trial.render("bill").unwrap();
    assert!(
        html.contains(r#"name="loading" inputmode="decimal" value="1.50""#),
        "{html}"
    );
    trial
        .bill_add(ProductId::new(1), NonZeroU32::new(2).unwrap())
        .unwrap();
    trial
        .bill_header(Header::new(
            "batch1-customer",
            "2025-04-30",
            None,
            HeaderIntent::Save,
        ))
        .unwrap();
    let html = trial.render("bill").unwrap();
    // Provisional ₹118 × 2 + loading ₹1.50 × 2 packets.
    assert!(
        html.contains("<span class=\"muted\">Loading</span><b>₹3.00</b>"),
        "{html}"
    );
    assert!(
        html.contains("<span class=\"muted\">Bill Amount</span><b>₹239.00</b>"),
        "{html}"
    );
    drop(trial);

    let mut trial = Trial::open(&root.0, Some(day())).unwrap();
    trial.bill_open(trial.local_draft(1)).unwrap();
    assert_eq!(
        trial.current.as_ref().unwrap().0.loading_rate(),
        Money::from_minor_units(150),
        "the draft keeps its loading rate across a restart"
    );
    trial
        .sale_prepare(&crate::billing::Terms {
            date: "2025-04-30".into(),
            paid: "0".into(),
        })
        .unwrap();
    let html = trial.render("review").unwrap();
    assert!(html.contains("<td>Loading</td>"), "{html}");
    let serial = trial.posting.pending(Role::Owner).unwrap().serial();
    trial.sale_confirm(serial).unwrap();
    let sale = trial.projected().unwrap().snapshot.sales.remove(0);
    // Owner rate ₹120.50 × 2 + ₹3 loading.
    assert_eq!(sale.total, Money::from_minor_units(24_400));
    let entry = trial
        .outbox
        .entries()
        .unwrap()
        .into_iter()
        .find(|e| e.intent.starts_with("sale "))
        .unwrap();
    assert!(entry.intent.ends_with(" load=150"), "{}", entry.intent);

    // A typed rate replaces the default on that draft only; blank means none.
    trial.current = None;
    trial.render("bill").unwrap();
    trial
        .bill_add(ProductId::new(2), NonZeroU32::new(1).unwrap())
        .unwrap();
    trial
        .bill_header(Header::new(
            "batch1-customer",
            "2025-04-30",
            Some(""),
            HeaderIntent::Select,
        ))
        .unwrap();
    assert!(trial.current.as_ref().unwrap().0.loading_rate().is_zero());
    assert!(
        trial
            .bill_header(Header::new(
                "batch1-customer",
                "2025-04-30",
                Some("-1"),
                HeaderIntent::Select,
            ))
            .is_err()
    );
}

#[test]
fn a_crate_sale_goes_through_the_authority_and_survives_a_restart() {
    let root = Root::new("crate-sale");
    let mut trial = Trial::open(&root.0, Some(day())).unwrap();
    trial.crate_form = Some(crate::views::CrateForm {
        rows: vec![
            ("1".into(), "2".into(), "Case".into()),
            ("2".into(), "3".into(), "Case".into()),
        ],
        ..Default::default()
    });
    trial
        .crate_submit(map(&[
            ("name", "Party Crate"),
            ("price", "400"),
            ("product_0", "1"),
            ("qty_0", "2"),
            ("unit_0", "Case"),
            ("product_1", "2"),
            ("qty_1", "3"),
            ("unit_1", "Case"),
            ("intent", "save"),
        ]))
        .unwrap();
    let template = trial.projected().unwrap().snapshot.crates[0].id;
    let sell = |trial: &mut Trial| {
        crate::bill_entry::crate_row::sell_on_new_bill(
            trial,
            template,
            "batch1-customer",
            2,
            "2025-04-30",
            "300",
        )
    };
    // DEC-034: a Manager posts the crate bill.
    trial.set_owner(false).unwrap();
    sell(&mut trial).unwrap();
    assert!(
        trial.message.contains("Crate bill #1 posted: 10 packets"),
        "{}",
        trial.message
    );
    trial.set_owner(true).unwrap();
    let check = |trial: &mut Trial| {
        let s = trial.projected().unwrap().snapshot;
        let on_hand = |p: u64| s.stock.iter().find(|r| r.product == p).unwrap().on_hand;
        assert_eq!((on_hand(1), on_hand(2)), (46, 34));
        let customer = s
            .customers
            .iter()
            .find(|c| c.id == "batch1-customer")
            .unwrap();
        assert_eq!(customer.balance, Money::from_minor_units(50_000));
        assert_eq!(s.sales_unsupported, None, "crate bills are listed in Sales");
    };
    check(&mut trial);
    crate::device::assert_book_view(&mut trial);
    drop(trial);
    let mut trial = Trial::open(&root.0, Some(day())).unwrap();
    check(&mut trial);
    let sent = trial.outbox.entries().unwrap();
    assert!(sent.iter().any(|e| {
        e.intent
            .starts_with(&format!("cratesale draft={} ", trial.local_draft(1)))
            && e.status == OutboxStatus::Confirmed
    }));
    // Selling again makes a new bill under the next draft number.
    assert!(sell(&mut trial).is_ok());
    assert_eq!(trial.projected().unwrap().snapshot.sales.len(), 2);
}

#[test]
fn a_crate_bill_is_listed_viewed_edited_cancelled_restored_and_imaged_in_sales() {
    let root = Root::new("crate-bill-sales");
    let mut trial = Trial::open(&root.0, Some(day())).unwrap();
    trial.crate_form = Some(crate::views::CrateForm {
        rows: vec![
            ("1".into(), "2".into(), "Case".into()),
            ("2".into(), "3".into(), "Case".into()),
        ],
        ..Default::default()
    });
    trial
        .crate_submit(map(&[
            ("name", "Party Crate"),
            ("price", "400"),
            ("product_0", "1"),
            ("qty_0", "2"),
            ("unit_0", "Case"),
            ("product_1", "2"),
            ("qty_1", "3"),
            ("unit_1", "Case"),
            ("intent", "save"),
        ]))
        .unwrap();
    let template = trial.projected().unwrap().snapshot.crates[0].id;
    crate::bill_entry::crate_row::sell_on_new_bill(
        &mut trial,
        template,
        "batch1-customer",
        2,
        "2025-04-30",
        "300",
    )
    .unwrap();
    let sales = trial.render("sales").unwrap();
    assert!(!sales.contains("role=\"alert\""), "{sales}");
    assert!(
        sales.contains("<b>#1</b>") && sales.contains("1 items · 10 pkt"),
        "{sales}"
    );
    assert!(sales.contains(&format!(
        r#"data-action="sale_edit_open" data-id="{}""#,
        trial.local_draft(1)
    )));
    trial.message.clear();
    trial.sales_filter.q = "party".into();
    assert!(
        trial.render("sales").unwrap().contains("<b>#1</b>"),
        "found by crate name"
    );
    trial.sales_filter.q = "synthetic lime".into();
    assert!(
        !trial.render("sales").unwrap().contains("<b>#1</b>"),
        "not by its contents"
    );
    trial.sales_filter.q.clear();

    // View (estimate) and bill image name the crate line as the HTML does.
    let estimate = trial.estimate_html(trial.local_draft(1)).unwrap();
    assert!(estimate.contains("Crate: Party Crate"), "{estimate}");
    let (name, png) = trial.bill_image_png(trial.local_draft(1)).unwrap();
    assert_eq!(name, "Bill-1.png");
    assert!(png.starts_with(b"\x89PNG"));

    // Edit: 3 crates at ₹380 (15 packets: Lime 6, Soda 9).
    trial.sale_edit = Some(trial.local_draft(1));
    let form = trial.render("sales").unwrap();
    assert!(
        form.contains("Crate: Party Crate") && form.contains(r#"name="qty_0""#),
        "{form}"
    );
    trial
        .sale_edit_save(map(&[
            ("id", &trial.local_draft(1).to_string()),
            ("qty_0", "3"),
            ("rate_0", "380"),
        ]))
        .unwrap();
    let stock = |trial: &mut Trial| {
        let s = trial.projected().unwrap().snapshot;
        let on_hand = |p: u64| s.stock.iter().find(|r| r.product == p).unwrap().on_hand;
        (
            on_hand(1),
            on_hand(2),
            s.sales[0].total,
            s.sales[0].cancelled,
        )
    };
    assert_eq!(
        stock(&mut trial),
        (44, 31, Money::from_minor_units(114_000), false)
    );
    assert!(
        trial
            .sale_edit_save(map(&[
                ("id", &trial.local_draft(1).to_string()),
                ("qty_0", "1"),
                ("rate_0", "1"),
            ]))
            .unwrap_err()
            .contains("below the amount already paid")
    );
    trial
        .send(&alm_sync::Intent::CancelSale {
            draft: trial.local_draft(1),
        })
        .unwrap();
    assert_eq!(
        stock(&mut trial),
        (50, 40, Money::from_minor_units(114_000), true)
    );
    trial
        .send(&alm_sync::Intent::RestoreSale {
            draft: trial.local_draft(1),
        })
        .unwrap();
    assert_eq!(
        stock(&mut trial),
        (44, 31, Money::from_minor_units(114_000), false)
    );
    crate::device::assert_book_view(&mut trial);
    let log: Vec<_> = trial
        .projected()
        .unwrap()
        .snapshot
        .owner
        .unwrap()
        .book
        .activity
        .iter()
        .map(|a| a.kind.clone())
        .collect();
    assert_eq!(
        log,
        [
            "crate-save",
            "bill-save",
            "bill-edit",
            "bill-cancel",
            "bill-restore"
        ]
    );
}

#[test]
fn the_crate_picker_lists_every_product_including_new_ones() {
    let root = Root::new("crate-picker");
    let mut trial = Trial::open(&root.0, Some(day())).unwrap();
    let orange = add_product(&mut trial, "Synthetic Orange", "30", "45");
    trial.crate_form = Some(crate::views::CrateForm {
        rows: vec![(String::new(), "1".into(), "Case".into())],
        ..Default::default()
    });
    let html = trial.render("crates").unwrap();
    for (id, item) in [
        (1, "Synthetic Lime"),
        (2, "Synthetic Soda"),
        (orange.value(), "Synthetic Orange"),
    ] {
        assert!(
            html.contains(&format!(r#"<option value="{id}">{item}</option>"#)),
            "{item}: {html}"
        );
    }
    trial
        .crate_submit(map(&[
            ("name", "Orange Box"),
            ("price", "90"),
            ("product_0", &orange.value().to_string()),
            ("qty_0", "2"),
            ("unit_0", "Case"),
            ("intent", "save"),
        ]))
        .unwrap();
    let crates = trial.projected().unwrap().snapshot.crates;
    assert!(
        crates
            .iter()
            .any(|c| c.name == "Orange Box" && c.components == 1)
    );
}

#[test]
fn the_default_reorder_level_goes_through_the_authority_and_survives_a_restart() {
    let root = Root::new("reorder-default");
    let mut trial = Trial::open(&root.0, Some(day())).unwrap();
    let form = crate::settings::Profile {
        name: "ALM".into(),
        reorder: "9".into(),
        ..Default::default()
    };
    trial.settings_profile(&form, 1).unwrap();
    assert!(
        trial
            .outbox
            .entries()
            .unwrap()
            .iter()
            .any(|e| e.intent == "reorderdefault level=9" && e.status == OutboxStatus::Confirmed)
    );
    // Saving again with the same level sends no reorder level (DEC-034: the shop's loading
    // charge goes with every save, for a Manager's New Bill).
    let reorder_sent = |trial: &Trial| {
        trial
            .outbox
            .entries()
            .unwrap()
            .iter()
            .filter(|e| e.intent.starts_with("reorderdefault"))
            .count()
    };
    let sent = reorder_sent(&trial);
    trial.settings_profile(&form, 2).unwrap();
    assert_eq!(reorder_sent(&trial), sent);
    let id = trial.posted.next_product_id(Role::Owner).unwrap();
    trial.products.form = Some(trial.posted.begin_product_entry(Role::Owner, id).unwrap());
    trial
        .product_save(&crate::products::Form {
            reorder: String::new(),
            ..product_form("Synthetic Default", "30", "45")
        })
        .unwrap();
    drop(trial);
    let mut trial = Trial::open(&root.0, Some(day())).unwrap();
    let s = trial.projected().unwrap().snapshot;
    assert_eq!(s.owner.unwrap().book.settings.reorder_default, 9);
    assert_eq!(
        s.products
            .iter()
            .find(|p| p.id == id.value())
            .map(|p| p.reorder_level),
        Some(9)
    );
    assert!(
        trial
            .render("settings")
            .unwrap()
            .contains(r#"name="reorder""#)
    );
    assert_eq!(trial.reorder_default().unwrap(), 9);
}

#[test]
fn the_activity_log_is_recorded_by_the_authority_with_device_and_account() {
    let root = Root::new("activity-log");
    let mut trial = Trial::open(&root.0, Some(day())).unwrap();
    let device = trial.device_id.clone();
    assert!(device.starts_with("dev-"));
    let html = trial.render("settings").unwrap();
    assert!(html.contains("No actions logged yet"));
    trial
        .receipt_save(crate::receipts::Terms {
            customer: "batch1-customer".into(),
            date: "2025-04-30".into(),
            amount: "10.50".into(),
            mode: crate::receipts::Mode::Online,
        })
        .unwrap();
    let on_hand = trial
        .posted
        .open_physical_count(Role::Owner, ProductId::new(2))
        .unwrap()
        .on_hand;
    trial
        .stock_count(&crate::stock::Count {
            product: "2".into(),
            on_hand: on_hand.to_string(),
            counted: "1234".into(),
            fridge: String::new(),
            reason: "count".into(),
            note: String::new(),
        })
        .unwrap();
    drop(trial);
    let mut trial = Trial::open(&root.0, Some(day())).unwrap();
    assert_eq!(trial.device_id, device, "the device keeps its identity");
    let html = trial.render("settings").unwrap();
    assert!(html.contains("Last 2 recorded actions"), "{html}");
    let adjust = html
        .find("Synthetic Soda: 40 → 1,234 (Count correction)")
        .unwrap();
    let payment = html.find("Synthetic Customer ₹11 (Online)").unwrap();
    assert!(adjust < payment, "newest first");
    assert!(html.contains(&format!(
        r#"<td class="muted" title="account trial">{device}</td><td>adjust</td>"#
    )));
    // The logged device is the install whose request IDs carried the commands.
    let entries = trial.outbox.entries().unwrap();
    assert_eq!(entries.len(), 2);
    assert!(
        entries
            .iter()
            .all(|e| e.request_id.starts_with(&format!("trial:{device}-")))
    );
    let rows = trial
        .projected()
        .unwrap()
        .snapshot
        .owner
        .unwrap()
        .book
        .activity;
    assert!(
        rows.iter()
            .all(|row| row.device == device && row.account == "trial")
    );
    // "When" in the device's local time, as the HTML's en-IN `toLocaleString`.
    trial.utc_offset_minutes = 330;
    let html = trial.render("settings").unwrap();
    let when = alm_core::projection::activity_time(rows[0].at_ms, 330);
    assert!(
        html.contains(&format!(r#"<td class="muted">{when}</td>"#)),
        "{when}: {html}"
    );
    assert!(when.ends_with(" am") || when.ends_with(" pm"));
    trial.set_owner(false).unwrap();
    let html = trial.render("settings").unwrap();
    assert!(!html.contains("Synthetic Customer ₹11") && !html.contains("Synthetic Soda: 40"));
}

#[test]
fn new_bill_takes_a_crate_row_and_posts_it_through_the_crate_sale_command() {
    use crate::bill_entry::{Header, HeaderIntent};
    let root = Root::new("new-bill-crate");
    let mut trial = Trial::open(&root.0, Some(day())).unwrap();
    trial.crate_form = Some(crate::views::CrateForm {
        rows: vec![
            ("1".into(), "2".into(), "Case".into()),
            ("2".into(), "3".into(), "Case".into()),
        ],
        ..Default::default()
    });
    trial
        .crate_submit(map(&[
            ("name", "Party Crate"),
            ("price", "400"),
            ("product_0", "1"),
            ("qty_0", "2"),
            ("unit_0", "Case"),
            ("product_1", "2"),
            ("qty_1", "3"),
            ("unit_1", "Case"),
            ("intent", "save"),
        ]))
        .unwrap();
    let template = trial.projected().unwrap().snapshot.crates[0].id;
    assert!(
        !trial.render("crates").unwrap().contains("crate_sell"),
        "the HTML's Crates screen has no Sell form"
    );
    let html = trial.render("bill").unwrap();
    assert!(html.contains(r#"data-action="bill_crate_add""#) && html.contains("Party Crate"));
    // DEC-034: a Manager adds crate rows too.
    trial.set_owner(false).unwrap();
    trial.current = None;
    assert!(
        trial.render("bill").unwrap().contains("bill_crate_add"),
        "a Manager adds crates"
    );
    assert!(trial.bill_crate_add(template, NonZeroU32::MIN).is_ok());
    trial.set_owner(true).unwrap();
    trial.current = None;
    trial.render("bill").unwrap();
    trial
        .bill_header(Header::new(
            "batch1-customer",
            "2025-04-30",
            None,
            HeaderIntent::Select,
        ))
        .unwrap();
    trial
        .bill_crate_add(template, NonZeroU32::new(2).unwrap())
        .unwrap();
    let html = trial.render("bill").unwrap();
    assert!(html.contains("📦 Party Crate"), "{html}");
    assert!(
        html.contains("2 Synthetic Lime + 3 Synthetic Soda"),
        "{html}"
    );
    assert!(html.contains("<b>₹800.00</b>"), "2 crates at ₹400: {html}");
    assert!(!html.contains("bill_crate_post"), "posted only once saved");
    assert!(!html.contains(">Owner review</button>"));
    // An item sits beside the crate row (OPEN-013 mixed bill); removed again here.
    trial.bill_add(ProductId::new(1), NonZeroU32::MIN).unwrap();
    assert_eq!(trial.current.as_ref().unwrap().0.lines().len(), 1);
    trial
        .edit_bill(|draft| Ok(draft.remove_line(std::num::NonZeroUsize::MIN).is_some()))
        .unwrap();
    trial
        .bill_crate_quantity(0, NonZeroU32::new(3).unwrap())
        .unwrap();
    assert!(trial.render("bill").unwrap().contains("<b>₹1,200.00</b>"));
    trial
        .bill_header(Header::new(
            "batch1-customer",
            "2025-04-30",
            None,
            HeaderIntent::Save,
        ))
        .unwrap();
    trial.bill_crate_post("500").unwrap();
    assert!(
        trial.message.contains("Crate bill #1 posted: 15 packets"),
        "{}",
        trial.message
    );
    let s = trial.projected().unwrap().snapshot;
    assert_eq!(
        (s.sales[0].invoice, s.sales[0].total, s.sales[0].paid),
        (
            1,
            Money::from_minor_units(120_000),
            Money::from_minor_units(50_000)
        )
    );
    let on_hand = |p: u64| s.stock.iter().find(|r| r.product == p).unwrap().on_hand;
    assert_eq!((on_hand(1), on_hand(2)), (44, 31));
    let sent = trial.outbox.entries().unwrap();
    assert!(sent.iter().any(|e| {
        e.intent
            .starts_with(&format!("cratesale draft={} ", trial.local_draft(1)))
    }));
    // A new bill starts empty; removing a crate row empties the bill.
    let html = trial.render("bill").unwrap();
    assert!(!html.contains("📦 Party Crate"));
    trial.bill_crate_add(template, NonZeroU32::MIN).unwrap();
    trial.bill_crate_remove(0).unwrap();
    assert!(trial.current.as_ref().unwrap().0.crate_rows().is_empty());
}

#[test]
fn new_bill_mixes_item_lines_and_crate_rows_and_posts_them_as_one_sale() {
    use crate::bill_entry::{Header, HeaderIntent};
    let root = Root::new("new-bill-mixed");
    let mut trial = Trial::open(&root.0, Some(day())).unwrap();
    trial.crate_form = Some(crate::views::CrateForm {
        rows: vec![
            ("1".into(), "2".into(), "Case".into()),
            ("2".into(), "3".into(), "Case".into()),
        ],
        ..Default::default()
    });
    trial
        .crate_submit(map(&[
            ("name", "Party Crate"),
            ("price", "400"),
            ("product_0", "1"),
            ("qty_0", "2"),
            ("unit_0", "Case"),
            ("product_1", "2"),
            ("qty_1", "3"),
            ("unit_1", "Case"),
            ("intent", "save"),
        ]))
        .unwrap();
    let template = trial.projected().unwrap().snapshot.crates[0].id;
    let header = |intent| Header::new("batch1-customer", "2025-04-30", Some(""), intent);
    trial.current = None;
    trial.render("bill").unwrap();
    trial.bill_header(header(HeaderIntent::Select)).unwrap();
    trial
        .bill_add(ProductId::new(1), NonZeroU32::new(2).unwrap())
        .unwrap();
    trial.bill_crate_add(template, NonZeroU32::MIN).unwrap();
    trial.bill_crate_add(template, NonZeroU32::MIN).unwrap();
    trial
        .bill_crate_quantity(1, NonZeroU32::new(2).unwrap())
        .unwrap();
    trial.bill_crate_remove(0).unwrap();
    let draft = &trial.current.as_ref().unwrap().0;
    assert_eq!((draft.lines().len(), draft.crate_rows().len()), (1, 1));
    let html = trial.render("bill").unwrap();
    assert!(
        html.contains("📦 Party Crate") && html.contains("Synthetic Lime"),
        "{html}"
    );
    assert!(
        html.contains(r#"data-action="bill_crate_remove" data-line="0""#),
        "{html}"
    );
    assert!(html.contains("<b>₹800.00</b>"), "2 crates at ₹400: {html}");
    // New Bill prices every row from one result: 2 Lime at the provisional ₹118 shown on the
    // row and 2 crates; no loading. Owner review re-prices at the Owner rate (₹1,041).
    assert!(
        html.contains("<span class=\"muted\">Bill Amount</span><b>₹1,036.00</b>"),
        "{html}"
    );
    assert!(!html.contains(">Owner review</button>"), "only once saved");
    trial.bill_header(header(HeaderIntent::Save)).unwrap();
    let html = trial.render("bill").unwrap();
    assert!(html.contains(">Owner review</button>"), "{html}");
    assert!(
        !html.contains("bill_crate_post"),
        "posted through Owner review"
    );
    assert!(trial.bill_crate_post("0").is_err());
    trial
        .sale_prepare(&crate::billing::Terms {
            date: "2025-04-30".into(),
            paid: "500".into(),
        })
        .unwrap();
    let review = trial.render("review").unwrap();
    assert!(review.contains("📦 Crate: Party Crate"), "{review}");
    let serial = trial.posting.pending(Role::Owner).unwrap().serial();
    assert_eq!(trial.sale_confirm(serial).unwrap(), 1);
    let entries = trial.outbox.entries().unwrap();
    let entry = entries
        .iter()
        .find(|e| e.intent.starts_with("sale draft="))
        .unwrap();
    assert!(
        entry.intent.contains(" crates=2,40000,"),
        "{}",
        entry.intent
    );
    let s = trial.projected().unwrap().snapshot;
    assert_eq!(
        (
            s.sales[0].lines,
            s.sales[0].items.as_slice(),
            s.sales[0].packets
        ),
        (
            2,
            &["Synthetic Lime".to_owned(), "Party Crate".to_owned()][..],
            2 + 10
        )
    );
    assert_eq!(s.sales[0].total, Money::from_minor_units(104_100));
    let on_hand = |s: &alm_core::projection::ProjectionSnapshot, p: u64| {
        s.stock.iter().find(|r| r.product == p).unwrap().on_hand
    };
    assert_eq!((on_hand(&s, 1), on_hand(&s, 2)), (44, 34));
    let bill = s.sales[0].draft_id;
    let estimate = trial.estimate_html(bill).unwrap();
    assert!(
        estimate.contains("Crate: Party Crate") && estimate.contains("Synthetic Lime"),
        "{estimate}"
    );
    let (_, png) = trial.bill_image_png(bill).unwrap();
    assert!(png.starts_with(b"\x89PNG"));

    // Edit: 1 Lime, 1 crate at ₹390.
    let id = s.sales[0].draft_id.to_string();
    trial
        .sale_edit_save(map(&[
            ("id", id.as_str()),
            ("qty_0", "1"),
            ("rate_0", "120.50"),
            ("qty_1", "1"),
            ("rate_1", "390"),
        ]))
        .unwrap();
    let s = trial.projected().unwrap().snapshot;
    assert_eq!(s.sales[0].total, Money::from_minor_units(51_050));
    assert_eq!((on_hand(&s, 1), on_hand(&s, 2)), (47, 37));
    drop(trial);
    let mut trial = Trial::open(&root.0, Some(day())).unwrap();
    let s = trial.projected().unwrap().snapshot;
    assert_eq!(s.sales[0].items, ["Synthetic Lime", "Party Crate"]);
}
