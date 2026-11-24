//! Every remaining write goes through the outbox and the authority and survives an app restart;
//! the device GST tables move into the authority once, without losing a record.
use std::collections::HashMap;

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
    trial.set_owner(false).unwrap();
    assert!(
        crate::godown::save(
            &mut trial,
            &crate::godown::Row {
                item: "X".into(),
                cases: "1".into(),
                note: String::new(),
            },
        )
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
    trial.bill_open(alm_core::DraftId::new(1)).unwrap();
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
    let entry = trial.outbox.entries().unwrap().remove(0);
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
    let template = trial.projected().unwrap().snapshot.crates[0].id.to_string();
    let sell = map(&[
        ("id", &template),
        ("customer", "batch1-customer"),
        ("qty", "2"),
        ("date", "2025-04-30"),
        ("paid", "300"),
    ]);
    trial.set_owner(false).unwrap();
    assert!(trial.crate_sell(&sell).is_err(), "Manager denied");
    trial.set_owner(true).unwrap();
    trial.crate_sell(&sell).unwrap();
    assert!(
        trial.message.contains("Crate bill #1 posted: 10 packets"),
        "{}",
        trial.message
    );
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
        assert_eq!(s.sales_unsupported, Some(1));
    };
    check(&mut trial);
    crate::device::assert_book_view(&mut trial);
    drop(trial);
    let mut trial = Trial::open(&root.0, Some(day())).unwrap();
    check(&mut trial);
    let sent = trial.outbox.entries().unwrap();
    assert!(
        sent.iter()
            .any(|e| e.intent.starts_with("cratesale draft=1 ")
                && e.status == OutboxStatus::Confirmed)
    );
    // Selling again makes a new bill under the next draft number.
    assert!(trial.crate_sell(&sell).is_ok());
    assert_eq!(
        trial.projected().unwrap().snapshot.sales_unsupported,
        Some(2)
    );
}
