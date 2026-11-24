//! Opt-in end to end through REST (development evidence, synthetic data): the app's command layer
//! against a PostgREST over the migrated schema. Set `ALM_REST_URL` (the PostgREST root) and
//! `ALM_JWT_OWNER` / `ALM_JWT_MANAGER` (tokens for the same account, Owner and Manager role) on
//! a freshly seeded database (`alm-sync` test `reset_and_seed_for_the_app` with `ALM_RESET_ONLY`);
//! without them the test returns early. A local proxy drops one `post_sale` response to give a
//! lost response.
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use alm_core::{BillDate, DraftId, Money, Role};
use alm_store::OutboxStatus;
use alm_sync::{Outcome, RestAuthority};

use crate::Trial;
use crate::screen_tests::{Root, entry};

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.is_empty())
}

fn day() -> BillDate {
    BillDate::new(2025, 4, 30).unwrap()
}

fn map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

/// Reads one HTTP request (headers and a `Content-Length` body); `None` at end of stream.
fn request(reader: &mut BufReader<TcpStream>) -> Option<(String, Vec<u8>)> {
    let mut head = String::new();
    let mut length = 0;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).ok()? == 0 {
            return None;
        }
        if line == "\r\n" {
            break;
        }
        let lower = line.to_ascii_lowercase();
        if let Some(value) = lower.strip_prefix("content-length:") {
            length = value.trim().parse().ok()?;
        }
        if !lower.starts_with("connection:") {
            head.push_str(&line);
        }
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body).ok()?;
    Some((head, body))
}

/// Forwards each request to `upstream`; while `lose` is set, the next `post_sale` is delivered
/// but its response is dropped and the connection closed.
fn proxy(upstream: &str, lose: Arc<AtomicBool>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let local = format!("http://{}", listener.local_addr().unwrap());
    let upstream = upstream.trim_start_matches("http://").to_owned();
    std::thread::spawn(move || {
        for client in listener.incoming().flatten() {
            let (upstream, lose) = (upstream.clone(), lose.clone());
            std::thread::spawn(move || {
                let mut reader = BufReader::new(client.try_clone().unwrap());
                let mut client = client;
                while let Some((head, body)) = request(&mut reader) {
                    let mut server = TcpStream::connect(&upstream).unwrap();
                    server
                        .write_all(format!("{head}Connection: close\r\n\r\n").as_bytes())
                        .unwrap();
                    server.write_all(&body).unwrap();
                    let mut response = Vec::new();
                    server.read_to_end(&mut response).unwrap();
                    if head.contains("/rpc/post_sale ") && lose.swap(false, Ordering::SeqCst) {
                        return;
                    }
                    if client.write_all(&response).is_err() {
                        return;
                    }
                }
            });
        }
    });
    local
}

fn rest(url: &str, owner: &str, manager: &str) -> RestAuthority {
    RestAuthority::new(url, "")
        .with_token(Role::Owner, owner)
        .with_token(Role::Manager, manager)
}

fn use_rest(trial: &mut Trial, authority: RestAuthority) {
    trial.rest = Some(authority);
    trial.projection.clear().unwrap();
    trial.refresh().unwrap();
}

#[test]
#[allow(clippy::too_many_lines)]
fn two_devices_through_rest_see_the_same_posted_rows() {
    let (Some(url), Some(owner), Some(manager)) = (
        env("ALM_REST_URL"),
        env("ALM_JWT_OWNER"),
        env("ALM_JWT_MANAGER"),
    ) else {
        return;
    };
    let lose = Arc::new(AtomicBool::new(false));
    let lossy = proxy(&url, lose.clone());
    let device_a = Root::new("rest-e2e-a");
    let mut trial = Trial::open(&device_a.0, Some(day())).unwrap();
    use_rest(&mut trial, rest(&lossy, &owner, &manager));

    // Manager: a provisional draft only; posting and posted screens are denied.
    trial.set_owner(false).unwrap();
    let draft = crate::billing_tests::draft().with_bill_date(Some(day()));
    trial.drafts.save_draft(draft).unwrap();
    assert!(trial.render("sales").unwrap().contains("role=\"alert\""));
    let terms = |paid: &str| crate::billing::Terms {
        date: "2025-04-30".into(),
        paid: paid.into(),
    };
    assert!(trial.sale_prepare(&terms("20")).is_err());

    // Owner review priced from the REST book rows; the confirm's response is lost.
    trial.set_owner(true).unwrap();
    trial.current = Some((
        trial.drafts.reopen_draft(DraftId::new(1)).unwrap().unwrap(),
        true,
    ));
    trial.sale_prepare(&terms("20")).unwrap();
    let serial = trial.posting.pending(Role::Owner).unwrap().serial();
    lose.store(true, Ordering::SeqCst);
    assert!(trial.sale_confirm(serial).is_err());
    let unknown = trial.outbox.unresolved().unwrap();
    assert_eq!(unknown.len(), 1);
    assert_eq!(unknown[0].status, OutboxStatus::Unknown);
    let Ok(Outcome::Sale(receipt)) = trial.retry(&unknown[0].request_id) else {
        panic!("the retry returns the committed sale")
    };
    assert_eq!(
        (receipt.invoice, receipt.total),
        (1, Money::from_minor_units(12_050))
    );
    assert_eq!(
        trial.outbox.entries().unwrap().len(),
        1,
        "no second request"
    );

    // Receipt, Cash Book close, edit and cancel of the bill.
    trial
        .receipt_save(crate::receipts::Terms {
            customer: "batch1-customer".into(),
            date: "2025-04-30".into(),
            amount: "10".into(),
            mode: crate::receipts::Mode::Cash,
        })
        .unwrap();
    assert!(
        trial.message.contains("by the REST authority"),
        "{}",
        trial.message
    );
    trial
        .cash_entry(entry("", "2025-04-30", "30", "save"))
        .unwrap();
    trial
        .sale_edit_save(map(&[("id", "1"), ("qty_0", "3"), ("rate_0", "120.50")]))
        .unwrap();
    let customer = |trial: &mut Trial| {
        trial
            .projected()
            .unwrap()
            .snapshot
            .customers
            .iter()
            .find(|c| c.id == "batch1-customer")
            .unwrap()
            .balance
    };
    assert_eq!(
        customer(&mut trial),
        Money::from_minor_units(36_150 - 2_000 - 1_000)
    );
    trial
        .send(&alm_sync::Intent::CancelSale { draft: 1 })
        .unwrap();
    assert_eq!(customer(&mut trial), Money::from_minor_units(-1_000));

    // A purchase order received in two parts at different rates.
    trial
        .order_add(&crate::order::Line {
            product: "1".into(),
            qty: "10".into(),
        })
        .unwrap();
    trial.order_checkout().unwrap();
    trial
        .order_po(&crate::order::Po {
            supplier: "Synthetic Wholesale".into(),
            date: "2025-04-30".into(),
            note: "rest".into(),
            paste: String::new(),
            intent: "save".into(),
        })
        .unwrap();
    for (qty, rate) in [("4", "69"), ("3", "71.50")] {
        trial
            .po_receive_save(map(&[
                ("po", "0"),
                ("date", "2025-04-30"),
                ("qty_0", qty),
                ("rate_0", rate),
            ]))
            .unwrap();
    }
    let owner_rows = trial.projected().unwrap().snapshot;
    let po = &owner_rows.owner.as_ref().unwrap().purchase_orders[0];
    assert_eq!(
        (
            po.status.as_str(),
            po.lines[0].received,
            po.lines[0].remaining
        ),
        ("PartiallyReceived", 7, 3)
    );

    // A crate sale, a stock count and a GST bill.
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
    trial
        .crate_sell(&map(&[
            ("id", &template),
            ("customer", "batch1-market"),
            ("qty", "2"),
            ("date", "2025-04-30"),
            ("paid", "300"),
        ]))
        .unwrap();
    let on_hand = trial
        .posted
        .open_physical_count(Role::Owner, alm_core::ProductId::new(2))
        .unwrap()
        .on_hand;
    trial
        .stock_count(&crate::stock::Count {
            product: "2".into(),
            on_hand: on_hand.to_string(),
            counted: (on_hand - 1).to_string(),
            fridge: "2".into(),
            reason: "damage".into(),
            note: "dropped".into(),
        })
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
                ("customer", "Synthetic GST Buyer REST"),
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
    assert!(
        trial
            .outbox
            .entries()
            .unwrap()
            .iter()
            .all(|e| e.status == OutboxStatus::Confirmed)
    );

    // Exact paise on device A.
    let a = trial.projected().unwrap().snapshot;
    let stock = |p: u64| a.stock.iter().find(|s| s.product == p).unwrap().on_hand;
    // Lime: 50 + 7 received - 4 (crate) with the bill cancelled; Soda: 40 - 6 (crate) - 1.
    assert_eq!((stock(1), stock(2)), (53, 33));
    let market = a
        .customers
        .iter()
        .find(|c| c.id == "batch1-market")
        .unwrap();
    assert_eq!(market.balance, Money::from_minor_units(80_000 - 30_000));
    assert_eq!(a.sales_unsupported, Some(2));
    assert_eq!(a.cash_days[0].amounts.cash, Money::from_minor_units(3_000));
    assert_eq!(a.gst_bills.len(), 1);
    let book = &a.owner.as_ref().unwrap().book;
    assert_eq!(book.sales.len(), 2);
    assert_eq!(book.sales[0].total, Money::from_minor_units(36_150));
    assert!(book.sales[0].cancelled);
    assert_eq!(book.sales[1].total, Money::from_minor_units(80_000));
    assert_eq!(
        book.purchasing.orders[0].lines[0]
            .receipts
            .iter()
            .map(|r| (r.quantity, r.rate.minor_units()))
            .collect::<Vec<_>>(),
        [(4, 6_900), (3, 7_150)]
    );

    // A second device of the same account pulls the same rows.
    let device_b = Root::new("rest-e2e-b");
    let mut other = Trial::open(&device_b.0, Some(day())).unwrap();
    other
        .settings_rest(&crate::dev_authority::RestForm::new(&url, "", &owner))
        .unwrap();
    let b = other.projected().unwrap().snapshot;
    assert_eq!(a, b, "identical projections on both devices");
    for screen in [
        "products",
        "purchases",
        "crates",
        "customers",
        "stock",
        "reports",
    ] {
        // Everything but the freshness line (its pull time differs).
        let strip = |html: String| match html.find("data-freshness") {
            Some(i) => {
                let end = html[i..].find("</div>").map_or(html.len(), |e| i + e);
                format!("{}{}", &html[..i], &html[end..])
            }
            None => html,
        };
        trial.message.clear();
        other.message.clear();
        assert_eq!(
            strip(trial.render(screen).unwrap()),
            strip(other.render(screen).unwrap()),
            "{screen}"
        );
    }

    // The Manager: denied, and its projection file holds no cost field or value.
    trial.set_owner(false).unwrap();
    assert!(
        trial
            .receipt_save(crate::receipts::Terms {
                customer: "batch1-customer".into(),
                date: "2025-04-30".into(),
                amount: "10".into(),
                mode: crate::receipts::Mode::Cash,
            })
            .is_err()
    );
    let manager_rows = trial.projected().unwrap().snapshot;
    assert!(manager_rows.owner.is_none() && manager_rows.sales.is_empty());
    let dir = device_a.0.canonicalize().unwrap().join("projections");
    let mut bytes = Vec::new();
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.to_string_lossy().contains("-manager.sqlite3") {
            bytes.extend(std::fs::read(&path).unwrap());
        }
    }
    let text = String::from_utf8_lossy(&bytes).to_lowercase();
    assert!(text.contains("synthetic lime"));
    for word in [
        "cost", "margin", "fifo", "profit", "owner_", "buy", "6900", "7150",
    ] {
        assert!(!text.contains(word), "{word} in the Manager projection");
    }
}
