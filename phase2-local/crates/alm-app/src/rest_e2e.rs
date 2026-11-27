//! Opt-in end to end through REST (development evidence, synthetic data): the app's command layer
//! against a PostgREST over the migrated schema. Set `ALM_REST_URL` (the PostgREST root) and
//! `ALM_JWT_OWNER` / `ALM_JWT_MANAGER` (tokens for the same account, Owner and Manager role) on
//! a freshly seeded database (`alm-sync` test `reset_and_seed_for_the_app` with `ALM_RESET_ONLY`);
//! without them the test returns early. A local proxy drops one `post_sale` response to give a
//! lost response. Such tokens name no GoTrue user: their role claim counts only on a database
//! with `alm.allow_claim_roles` on (the plain-Postgres test bootstrap sets it).
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use alm_core::{BillDate, DraftId, Money, Role};
use alm_store::{OutboxStatus, ProjectionStore};
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

/// The `sub` claim of a JWT (its payload decoded from base64url).
fn jwt_subject(token: &str) -> String {
    let payload = token.split('.').nth(1).unwrap();
    let mut bits = 0u32;
    let mut count = 0;
    let mut bytes = Vec::new();
    for c in payload.bytes() {
        let value = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'-' => 62,
            b'_' => 63,
            _ => continue,
        };
        bits = (bits << 6) | u32::from(value);
        count += 6;
        if count >= 8 {
            count -= 8;
            bytes.push(u8::try_from((bits >> count) & 0xff).unwrap());
        }
    }
    let claims = String::from_utf8(bytes).unwrap();
    let start = claims.find(r#""sub":""#).unwrap() + 7;
    let end = start + claims[start..].find('"').unwrap();
    claims[start..end].to_owned()
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
    let device = trial.device_id.clone();
    use_rest(
        &mut trial,
        rest(&lossy, &owner, &manager).with_device(&device),
    );

    // Manager: the Owner review is denied (DEC-034: it drafts from the cost-free book it
    // pulled from REST and opens Sales without profit).
    trial.set_owner(false).unwrap();
    assert!(trial.manager_book());
    assert_eq!(trial.manager_catalog(), None);
    assert!(trial.bill_item_search("lime").contains("Synthetic Lime"));
    let draft = crate::billing_tests::draft().with_bill_date(Some(day()));
    trial.drafts.save_draft(draft).unwrap();
    let sales = trial.render("sales").unwrap();
    assert!(!sales.contains("role=\"alert\"") && !sales.contains("Profit"));
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
    let review = trial.render("review").unwrap();
    assert!(
        review.contains("<td>Synthetic Lime</td>") && !review.contains("Product 1"),
        "{review}"
    );
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
        .send(&alm_sync::Intent::CancelSale {
            draft: alm_core::DraftId::new(1),
        })
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
    let template = trial.projected().unwrap().snapshot.crates[0].id;
    crate::bill_entry::crate_row::sell_on_new_bill(
        &mut trial,
        template,
        "batch1-market",
        2,
        "2025-04-30",
        "300",
    )
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
    assert_eq!(
        a.sales_unsupported, None,
        "the crate bill is listed in Sales"
    );
    assert_eq!(a.sales.len(), 2);
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

    // The authority recorded each action with this device.
    let log: Vec<String> = book
        .activity
        .iter()
        .map(|a| format!("{} {}", a.kind, a.message()))
        .collect();
    assert_eq!(
        log,
        [
            "bill-save #1 Synthetic Customer ₹121",
            "payment Synthetic Customer ₹10 (Cash)",
            "bill-edit #1 Synthetic Customer ₹362",
            "bill-cancel #1 Synthetic Customer ₹362",
            "grn PO-001 Synthetic Wholesale — goods received",
            "grn PO-001 Synthetic Wholesale — goods received",
            "crate-save Party Crate",
            "bill-save #2 Synthetic Market ₹800",
            "adjust Synthetic Soda: 34 → 33 (Damage / leakage)",
            "gst-save GST 1 Synthetic GST Buyer REST",
        ]
    );
    assert!(book.activity.iter().all(|a| a.device == device));

    // A second device of the same account pulls the same rows.
    let device_b = Root::new("rest-e2e-b");
    let mut other = Trial::open(&device_b.0, Some(day())).unwrap();
    let device_b_id = other.device_id.clone();
    use_rest(
        &mut other,
        rest(&url, &owner, &manager).with_device(&device_b_id),
    );
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
        // Everything but the freshness line (its pull time differs) and the session line (each
        // device names its own account; checked on its own below).
        let strip = |html: String| {
            let html = match html.find("data-freshness") {
                Some(i) => {
                    let end = html[i..].find("</div>").map_or(html.len(), |e| i + e);
                    format!("{}{}", &html[..i], &html[end..])
                }
                None => html,
            };
            match html.find(r#"<template id="screen-session""#) {
                Some(i) => {
                    let end = html[i..].find("</template>").map_or(html.len(), |e| i + e);
                    assert!(html[i..end].contains("data-authority=\"REST · "), "{html}");
                    format!("{}{}", &html[..i], &html[end..])
                }
                None => html,
            }
        };
        trial.message.clear();
        other.message.clear();
        assert_eq!(
            strip(trial.render(screen).unwrap()),
            strip(other.render(screen).unwrap()),
            "{screen}"
        );
    }

    // Both devices of the account send a command at the same moment: device B's first request
    // does not collide with device A's, and an identical retry returns B's original outcome.
    let pay = |amount: &str| crate::receipts::Terms {
        customer: "batch1-customer".into(),
        date: "2025-04-30".into(),
        amount: amount.into(),
        mode: crate::receipts::Mode::Online,
    };
    let (paid_a, paid_b) = std::thread::scope(|scope| {
        let a = scope.spawn(|| trial.receipt_save(pay("7")));
        let b = scope.spawn(|| other.receipt_save(pay("8")));
        (a.join().unwrap(), b.join().unwrap())
    });
    paid_a.unwrap();
    paid_b.unwrap();
    let sent = other.outbox.entries().unwrap();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].status, OutboxStatus::Confirmed);
    assert_eq!(
        sent[0].request_id,
        format!("trial:{}-1", other.device_id),
        "device B's first request"
    );
    assert_ne!(other.device_id, device);
    let request = alm_sync::RequestId::new(&sent[0].request_id).unwrap();
    let intent = alm_sync::Intent::parse(&sent[0].intent).unwrap();
    assert_eq!(
        alm_sync::SalesAuthority::submit(other.sales_authority(), Role::Owner, &request, &intent)
            .map(|o| o.canonical()),
        Ok(sent[0].outcome.clone().unwrap())
    );
    other.refresh().unwrap();
    let log = other
        .projected()
        .unwrap()
        .snapshot
        .owner
        .unwrap()
        .book
        .activity;
    let payments: Vec<_> = log
        .iter()
        .filter(|row| row.kind == "payment")
        .map(|row| (row.device.as_str(), row.message()))
        .collect();
    assert!(payments.contains(&(device.as_str(), "Synthetic Customer ₹7 (Online)".into())));
    assert!(payments.contains(&(
        other.device_id.as_str(),
        "Synthetic Customer ₹8 (Online)".into()
    )));
    // The account is the token's subject, never the account named in a request ID.
    let subject = jwt_subject(&owner);
    assert!(log.iter().all(|row| row.account == subject), "{log:?}");
    assert_ne!(subject, "trial");

    // Device B's first draft is numbered 1, like device A's posted bill #1, but carries device
    // B's origin: the authority posts it under its own invoice instead of refusing it as
    // already posted (the defect the Android smoke found).
    let first = other.local_draft(1);
    let base = crate::billing_tests::draft();
    let draft = alm_core::ProvisionalDraft::new(first, base.lines().to_vec())
        .with_customer(base.customer().clone())
        .with_bill_date(Some(day()));
    other.drafts.save_draft(draft.clone()).unwrap();
    other.current = Some((other.drafts.reopen_draft(first).unwrap().unwrap(), true));
    other.sale_prepare(&terms("20")).unwrap();
    let serial = other.posting.pending(Role::Owner).unwrap().serial();
    let invoice = other.sale_confirm(serial).unwrap();
    assert!(invoice > 1, "a new invoice, not #1");
    let posted = other.projected().unwrap().snapshot.sales;
    assert!(
        posted
            .iter()
            .any(|s| s.draft_id == first && s.invoice == invoice)
    );
    assert!(
        posted
            .iter()
            .any(|s| s.draft_id == DraftId::new(1) && s.invoice == 1)
    );

    // A legacy backup import through REST from Settings: dry run, explicit confirmation,
    // manifest and quarantine; the reconciliation from the REST book rows equals the dry run;
    // a second import changes nothing and nothing is posted.
    std::fs::create_dir_all(device_a.0.join("legacy-import")).unwrap();
    std::fs::write(
        device_a.0.join("legacy-import/ALM_backup_rehearsal.json"),
        include_bytes!("../../alm-core/tests/legacy/rehearsal-a.json"),
    )
    .unwrap();
    let book = |trial: &mut Trial| trial.projected().unwrap().snapshot.owner.unwrap().book;
    let before = book(&mut trial);
    trial.legacy_dry_run("ALM_backup_rehearsal.json").unwrap();
    let dry = match &trial.legacy.as_ref().unwrap().stage {
        crate::legacy_import::Stage::DryRun(r) => r.clone(),
        crate::legacy_import::Stage::Imported { .. } => panic!("a dry run"),
    };
    trial.legacy_import_confirm().unwrap();
    assert!(
        trial.message.contains("by the REST authority"),
        "{}",
        trial.message
    );
    let crate::legacy_import::Stage::Imported {
        manifest, report, ..
    } = &trial.legacy.as_ref().unwrap().stage
    else {
        panic!("imported")
    };
    assert_eq!((manifest.id, report), (1, &dry));
    assert_eq!(report.quarantined.len(), 4);
    let after = book(&mut trial);
    trial.legacy_dry_run("ALM_backup_rehearsal.json").unwrap();
    trial.legacy_import_confirm().unwrap();
    assert_eq!(book(&mut trial).legacy, after.legacy, "no duplicates");
    assert_eq!(after.sales, before.sales, "the import posts nothing");
    trial.legacy_close();

    // The Manager (DEC-034: it records payments) and its projection file, which holds no cost
    // field or value.
    trial.set_owner(false).unwrap();
    trial
        .receipt_save(crate::receipts::Terms {
            customer: "batch1-customer".into(),
            date: "2025-04-30".into(),
            amount: "10".into(),
            mode: crate::receipts::Mode::Cash,
        })
        .unwrap();
    assert!(
        trial
            .send(&alm_sync::Intent::SetReorderDefault { level: 2 })
            .is_err(),
        "Owner settings stay denied"
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
        "cost", "margin", "fifo", "profit", "owner_", "buy", "6900", "7150", "legacy", "buyrate",
        "avgcost",
    ] {
        assert!(!text.contains(word), "{word} in the Manager projection");
    }
}

/// Owner review in REST mode names each item line from the catalog the device pulled from
/// the REST authority, as Sales and the bill do (OPEN-013); it reads only, so it can run beside
/// the other REST tests on the same database.
#[test]
fn owner_review_in_rest_mode_names_item_lines_from_the_pulled_catalog() {
    let (Some(url), Some(owner), Some(manager)) = (
        env("ALM_REST_URL"),
        env("ALM_JWT_OWNER"),
        env("ALM_JWT_MANAGER"),
    ) else {
        return;
    };
    let root = Root::new("rest-review-names");
    let mut trial = Trial::open(&root.0, Some(day())).unwrap();
    let device = trial.device_id.clone();
    use_rest(
        &mut trial,
        rest(&url, &owner, &manager).with_device(&device),
    );
    trial.set_owner(true).unwrap();
    let catalog = trial.projected().unwrap().snapshot.catalog;
    let draft = crate::billing_tests::draft().with_bill_date(Some(day()));
    trial.current = Some((draft.clone(), false));
    let review = trial.render("review").unwrap();
    assert!(review.contains("<b>Owner total"), "{review}");
    for line in draft.lines() {
        let name = &catalog
            .iter()
            .find(|p| p.id == line.product_id().value())
            .expect("the draft's product is in the pulled catalog")
            .name;
        assert!(review.contains(&format!("<td>{name}</td>")), "{review}");
        assert!(
            !review.contains(&format!("<td>Product {}</td>", line.product_id())),
            "{review}"
        );
    }
}

/// Real accounts on the device (opt-in): `ALM_REST_URL`, `ALM_GOTRUE_URL` and two GoTrue users
/// on a freshly seeded database, `ALM_OWNER_EMAIL` and `ALM_MANAGER_EMAIL` (role in
/// `app_metadata`), with the password in `ALM_TEST_PASSWORD`. The sign-in screen replaces the
/// synthetic role switch; the role comes from the token; signing out removes the account's
/// posted data from the device; after a restart the person signs in again.
#[test]
fn people_sign_in_with_their_own_accounts_and_the_role_comes_from_the_token() {
    use crate::dev_authority::{LoginForm, RestForm};
    let (Some(url), Some(auth), Some(owner), Some(manager), Some(password)) = (
        env("ALM_REST_URL"),
        env("ALM_GOTRUE_URL"),
        env("ALM_OWNER_EMAIL"),
        env("ALM_MANAGER_EMAIL"),
        env("ALM_TEST_PASSWORD"),
    ) else {
        return;
    };
    let device = Root::new("rest-login");
    let mut trial = Trial::open(&device.0, Some(day())).unwrap();
    trial
        .settings_rest(&RestForm::new(&url, &auth, ""))
        .unwrap();
    assert!(trial.signed_out());
    let page = trial.render("review").unwrap();
    assert!(page.contains(r#"data-action="rest_login""#) && !page.contains("review_role"));
    let host = url.split_once("://").map_or(url.as_str(), |(_, h)| h);
    let host = host.split('/').next().unwrap();
    let header = |account: &str| {
        format!(
            r#"<template id="screen-session" data-account="{account}" data-authority="REST · {host}" data-dot="syncdot ok">"#
        )
    };
    assert!(
        page.contains(&format!(
            r#"<template id="screen-session" data-account="Not signed in" data-authority="REST · {host}""#
        )),
        "{page}"
    );
    assert!(
        trial
            .rest_login(&LoginForm::new(&manager, "wrong"))
            .unwrap_err()
            .starts_with("Sign-in refused")
    );

    // The Manager: Manager projection only, no Owner screens or commands, no role switch.
    trial
        .rest_login(&LoginForm::new(&manager, &password))
        .unwrap();
    assert_eq!(trial.role(), Role::Manager);
    let review = trial.render("review").unwrap();
    assert!(
        review.contains("Signed in as") && !review.contains("review_role"),
        "{review}"
    );
    assert!(
        review.contains(&header(&format!("{manager} · Manager"))),
        "{review}"
    );
    assert!(trial.projected().unwrap().snapshot.owner.is_none());
    // DEC-034: a Manager opens Sales, without profit; Owner screens stay denied.
    let sales = trial.render("sales").unwrap();
    assert!(!sales.contains("role=\"alert\"") && !sales.contains("Profit"));
    assert!(
        trial
            .render("dashboard")
            .unwrap()
            .contains("role=\"alert\"")
    );
    let manager_account = trial.account.clone();
    trial.rest_logout().unwrap();
    assert!(trial.signed_out());

    // The Owner, on the same device: its own account tables; a sale through Owner review.
    trial
        .rest_login(&LoginForm::new(&owner, &password))
        .unwrap();
    assert_eq!(trial.role(), Role::Owner);
    assert_ne!(trial.account, manager_account);
    let sales = trial.render("sales").unwrap();
    assert!(
        sales.contains(&header(&format!("{owner} · Owner"))),
        "{sales}"
    );
    assert!(!sales.contains("Built-in simulation"));
    let draft = crate::billing_tests::draft().with_bill_date(Some(day()));
    trial.drafts.save_draft(draft.clone()).unwrap();
    trial.current = Some((draft, true));
    trial
        .sale_prepare(&crate::billing::Terms {
            date: "2025-04-30".into(),
            paid: "20".into(),
        })
        .unwrap();
    let serial = trial.posting.pending(Role::Owner).unwrap().serial();
    let invoice = trial.sale_confirm(serial).unwrap();
    let owner_account = trial.account.clone();
    assert!(
        ProjectionStore::open_scoped(&device.0, &owner_account, Role::Owner)
            .unwrap()
            .load()
            .unwrap()
            .is_some()
    );
    trial.rest_logout().unwrap();
    assert!(
        ProjectionStore::open_scoped(&device.0, &owner_account, Role::Owner)
            .unwrap()
            .load()
            .unwrap()
            .is_none(),
        "signing out removes the account's posted data from the device"
    );

    // Tokens are in memory only: after a restart the Owner signs in again and finds the bill.
    drop(trial);
    let mut trial = Trial::open(&device.0, Some(day())).unwrap();
    assert!(trial.signed_out());
    trial
        .rest_login(&LoginForm::new(&owner, &password))
        .unwrap();
    let sales = trial.projected().unwrap().snapshot.sales;
    assert!(sales.iter().any(|s| s.invoice == invoice));
    trial.rest_logout().unwrap();
}

/// The shop's calendar day now (Asia/Kolkata), which a Manager's Cash Book closes.
fn shop_today() -> BillDate {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    alm_core::projection::shop_today(u64::try_from(now).unwrap()).unwrap()
}

/// DEC-034 smoke through local GoTrue and PostgREST: a signed-in Manager posts a mixed bill,
/// records a payment, edits and cancels bills, receives goods on a PO by quantity, adjusts
/// stock and closes today's Cash Book; no screen or device file shows a cost or profit; the
/// Owner then sees each of these in the activity log under the Manager's login email.
#[test]
fn a_signed_in_manager_works_the_shop_through_rest_without_costs() {
    use crate::dev_authority::{LoginForm, RestForm};
    use alm_sync::Intent;
    let (Some(url), Some(auth), Some(owner), Some(manager), Some(password)) = (
        env("ALM_REST_URL"),
        env("ALM_GOTRUE_URL"),
        env("ALM_OWNER_EMAIL"),
        env("ALM_MANAGER_EMAIL"),
        env("ALM_TEST_PASSWORD"),
    ) else {
        return;
    };
    let today = shop_today();
    let device = Root::new("rest-manager-smoke");
    let mut trial = Trial::open(&device.0, Some(today)).unwrap();
    trial
        .settings_rest(&RestForm::new(&url, &auth, ""))
        .unwrap();

    // The Owner: the shop's loading charge, a crate template and a purchase order.
    trial
        .rest_login(&LoginForm::new(&owner, &password))
        .unwrap();
    trial
        .send(&Intent::SetLoadingDefault {
            rate: Money::from_minor_units(150),
        })
        .unwrap();
    trial
        .send(&Intent::SaveCrate(alm_sync::CrateTerms {
            id: None,
            name: "Smoke Crate".into(),
            price: Money::from_minor_units(40_000),
            components: vec![(1, 2, false)],
        }))
        .unwrap();
    trial
        .send(&Intent::SavePurchaseOrder(alm_sync::PurchaseOrderTerms {
            supplier: Some((None, "Synthetic Depot".into())),
            date: Some(today),
            note: String::new(),
            lines: vec![alm_sync::PurchaseLineTerms {
                item: "Synthetic Soda".into(),
                product: Some(2),
                ordered: 10,
                rate: Money::from_minor_units(3_610),
            }],
        }))
        .unwrap();
    trial.rest_logout().unwrap();

    // The Manager.
    trial
        .rest_login(&LoginForm::new(&manager, &password))
        .unwrap();
    assert_eq!(trial.role(), Role::Manager);
    let bill = |trial: &mut Trial, product: u64, quantity: u32, crate_row: bool| {
        trial.current = None;
        trial.render("bill").unwrap();
        trial
            .bill_add(
                alm_core::ProductId::new(product),
                std::num::NonZeroU32::new(quantity).unwrap(),
            )
            .unwrap();
        if crate_row {
            let (template, _) = trial
                .bill_crates()
                .into_iter()
                .find(|(_, name)| name == "Smoke Crate")
                .unwrap();
            trial
                .bill_crate_add(template, std::num::NonZeroU32::MIN)
                .unwrap();
        }
        trial
            .bill_header(crate::bill_entry::Header::new(
                "batch1-customer",
                &today.to_string(),
                None,
                crate::bill_entry::HeaderIntent::Save,
            ))
            .unwrap();
        assert_eq!(
            trial.current.as_ref().unwrap().0.loading_rate(),
            Money::from_minor_units(150)
        );
    };
    bill(&mut trial, 1, 2, true);
    let mixed = trial.bill_post("50").unwrap();
    bill(&mut trial, 2, 1, false);
    let second = trial.bill_post("20").unwrap();
    let sales = |trial: &mut Trial| {
        let stored = trial.projected().unwrap();
        trial.screen_rows(&stored).sales.clone()
    };
    let posted = sales(&mut trial);
    let mixed = posted.iter().find(|s| s.invoice == mixed).unwrap().clone();
    let second = posted.iter().find(|s| s.invoice == second).unwrap().clone();
    assert_eq!(mixed.items.len(), 2, "the item and the crate");
    trial
        .receipt_save(crate::receipts::Terms {
            customer: "batch1-customer".into(),
            date: today.to_string(),
            amount: "30".into(),
            mode: crate::receipts::Mode::Cash,
        })
        .unwrap();
    // Only the bill whose stock moved last can be cancelled (the core's rule).
    trial
        .send(&Intent::CancelSale {
            draft: second.draft_id,
        })
        .unwrap();
    trial
        .sale_edit_save(map(&[
            ("id", &mixed.draft_id.to_string()),
            ("qty_0", "1"),
            ("rate_0", "120.50"),
            ("qty_1", "1"),
            ("rate_1", "400"),
        ]))
        .unwrap();
    let after = sales(&mut trial);
    assert!(
        after
            .iter()
            .any(|s| s.draft_id == second.draft_id && s.cancelled)
    );
    assert!(
        after
            .iter()
            .any(|s| s.draft_id == mixed.draft_id
                && s.total.minor_units() < mixed.total.minor_units())
    );
    let po = trial
        .posted
        .purchase_orders(Role::Manager)
        .unwrap()
        .last()
        .unwrap()
        .id
        .index();
    trial.render("purchases").unwrap();
    trial
        .po_receive_save(map(&[
            ("po", &po.to_string()),
            ("date", &today.to_string()),
            ("qty_0", "4"),
        ]))
        .unwrap();
    let stored = trial.projected().unwrap();
    let soda = trial
        .screen_rows(&stored)
        .stock
        .iter()
        .find(|r| r.product == 2)
        .unwrap()
        .on_hand;
    trial
        .stock_count(&crate::stock::Count {
            product: "2".into(),
            on_hand: soda.to_string(),
            counted: (soda - 1).to_string(),
            fridge: String::new(),
            reason: "damage".into(),
            note: String::new(),
        })
        .unwrap();
    trial
        .cash_entry(entry("", &today.to_string(), "100", "save"))
        .unwrap();
    assert!(trial.render("expenses").unwrap().contains("closed"));
    // Owner-only commands stay refused at the authority.
    assert!(
        trial
            .send(&Intent::ClosePurchaseOrder {
                po: u64::try_from(po).unwrap(),
                remaining: vec![6],
            })
            .is_err()
    );

    // No cost or profit on any Manager screen or in its device files.
    for tab in crate::screen_tests::MANAGER_SCREENS.iter().chain(&["bill"]) {
        let html = trial.render(tab).unwrap();
        assert!(!html.contains("role=\"alert\""), "{tab}");
        for hidden in [
            "Profit",
            "Stock Value",
            "Est. Cost",
            "Best Supplier",
            "70.00",
            "38.00",
            "69.00",
            "72.00",
            "36.10",
            "₹70",
            "₹38",
            "₹36",
        ] {
            assert!(!html.contains(hidden), "{tab} shows {hidden}");
        }
    }
    let mut bytes = Vec::new();
    for entry in std::fs::read_dir(device.0.join("projections")).unwrap() {
        let path = entry.unwrap().path();
        if path.to_string_lossy().contains("-manager.sqlite3") {
            bytes.extend(std::fs::read(&path).unwrap());
        }
    }
    let text = String::from_utf8_lossy(&bytes).to_lowercase();
    assert!(text.contains("synthetic lime"));
    for word in [
        "unit_cost",
        "margin",
        "fifo",
        "profit",
        "\"buy\"",
        "3610",
        "6900",
        "7200",
    ] {
        assert!(!text.contains(word), "{word} in the Manager projection");
    }
    trial.rest_logout().unwrap();

    // The Owner sees the Manager's actions under its login email.
    trial
        .rest_login(&LoginForm::new(&owner, &password))
        .unwrap();
    let activity = trial
        .projected()
        .unwrap()
        .snapshot
        .owner
        .unwrap()
        .book
        .activity;
    for kind in [
        "bill-save",
        "payment",
        "bill-edit",
        "bill-cancel",
        "grn",
        "adjust",
    ] {
        assert!(
            activity
                .iter()
                .any(|row| row.kind == kind && row.account == manager),
            "{kind}: {activity:?}"
        );
    }
    let log = trial.render("settings").unwrap();
    assert!(
        log.contains(&manager),
        "the Owner's activity log names the Manager"
    );
    trial.rest_logout().unwrap();
}
